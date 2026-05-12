use bevy::prelude::*;
use bevy::window::WindowResizeConstraints;

use crate::squad_battler::combat::BattleGrid;
use crate::squad_battler::encounters::{SquadNodeKind, SquadRouteNode};
use crate::squad_battler::rewards::RecruitDestination;
use crate::squad_battler::roster::SquadMemberView;
use crate::squad_battler::state::{FightCommand, SquadBattlerApp, SquadBattlerView};

use super::assets;
use super::board::{self, BoardGeometry};
use super::camera;
use super::combat_fx;
use super::units;

const DEMO_SEED: u64 = 0x5155_4144_4256_0001;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GameScreen {
    MainMenu,
    Route,
    FightPreview,
    Combat,
    RewardReview,
    Roster,
    RunOver,
}

#[derive(Resource)]
pub struct VisualGameState {
    pub app: SquadBattlerApp,
    pub view: SquadBattlerView,
}

#[derive(Resource)]
struct VisualNav {
    screen: GameScreen,
    previous_screen: GameScreen,
    selected_node_index: usize,
    selected_recruit_index: usize,
    selected_active_index: usize,
    selected_bench_index: usize,
    speed: u32,
    paused: bool,
    dirty: bool,
    rendered_screen: Option<GameScreen>,
    message: String,
}

impl Default for VisualNav {
    fn default() -> Self {
        Self {
            screen: GameScreen::MainMenu,
            previous_screen: GameScreen::Route,
            selected_node_index: 0,
            selected_recruit_index: 0,
            selected_active_index: 0,
            selected_bench_index: 0,
            speed: 1,
            paused: false,
            dirty: true,
            rendered_screen: None,
            message: "Press N to roll a squad.".to_string(),
        }
    }
}

#[derive(Resource)]
struct CombatTickTimer(Timer);

#[derive(Component)]
struct ScreenEntity;

pub fn run() {
    App::new()
        .insert_resource(ClearColor(assets::clear_color()))
        .insert_resource(BoardGeometry::new(BattleGrid::default()))
        .insert_resource(CombatTickTimer(Timer::from_seconds(
            1.0,
            TimerMode::Repeating,
        )))
        .insert_resource(VisualNav::default())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "HackMaster Squad Battler".to_string(),
                resolution: (1280.0, 768.0).into(),
                resize_constraints: WindowResizeConstraints {
                    min_width: 860.0,
                    min_height: 520.0,
                    ..default()
                },
                ..default()
            }),
            ..default()
        }))
        .add_plugins(combat_fx::CombatFxPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_keyboard_input,
                render_current_screen,
                camera::fit_camera_to_board,
                advance_combat,
                units::sync_unit_targets,
                units::animate_unit_motion,
            ),
        )
        .run();
}

fn setup(mut commands: Commands) {
    let app = SquadBattlerApp::new().expect("failed to create squad battler app");
    let view = app.view();
    camera::spawn_camera(&mut commands);
    commands.insert_resource(VisualGameState { app, view });
}

fn handle_keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut nav: ResMut<VisualNav>,
    mut state: ResMut<VisualGameState>,
) {
    if keys.just_pressed(KeyCode::KeyN) {
        start_new_run(&mut state, &mut nav);
        return;
    }

    match nav.screen {
        GameScreen::MainMenu => {}
        GameScreen::Route => handle_route_input(&keys, &mut state, &mut nav),
        GameScreen::FightPreview => handle_fight_preview_input(&keys, &mut state, &mut nav),
        GameScreen::Combat => handle_combat_input(&keys, &mut state, &mut nav),
        GameScreen::RewardReview => handle_reward_input(&keys, &mut state, &mut nav),
        GameScreen::Roster => handle_roster_input(&keys, &mut state, &mut nav),
        GameScreen::RunOver => {}
    }
}

fn handle_route_input(
    keys: &ButtonInput<KeyCode>,
    state: &mut VisualGameState,
    nav: &mut VisualNav,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        nav.previous_screen = nav.screen;
        nav.screen = GameScreen::Roster;
        nav.dirty = true;
        return;
    }
    let available_count = state.view.available_nodes.len();
    if available_count == 0 {
        return;
    }
    if keys.just_pressed(KeyCode::ArrowLeft) {
        nav.selected_node_index = nav
            .selected_node_index
            .checked_sub(1)
            .unwrap_or(available_count.saturating_sub(1));
        nav.dirty = true;
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        nav.selected_node_index = (nav.selected_node_index + 1) % available_count;
        nav.dirty = true;
    }
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        let Some(node_id) = selected_node_id(&state.view, nav) else {
            return;
        };
        apply_view_result(
            state.app.choose_node(node_id),
            state,
            nav,
            "Encounter selected.",
        );
    }
}

fn handle_fight_preview_input(
    keys: &ButtonInput<KeyCode>,
    state: &mut VisualGameState,
    nav: &mut VisualNav,
) {
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        apply_view_result(
            state.app.start_fight(),
            state,
            nav,
            "Fight started. Press Space to pause.",
        );
    }
}

fn handle_combat_input(
    keys: &ButtonInput<KeyCode>,
    state: &mut VisualGameState,
    nav: &mut VisualNav,
) {
    if keys.just_pressed(KeyCode::Space) {
        nav.paused = !nav.paused;
        nav.message = if nav.paused {
            "Paused.".to_string()
        } else {
            "Combat running.".to_string()
        };
    }
    if keys.just_pressed(KeyCode::Digit1) {
        nav.speed = 1;
        nav.message = "Speed 1x.".to_string();
    }
    if keys.just_pressed(KeyCode::Digit2) {
        nav.speed = 2;
        nav.message = "Speed 2x.".to_string();
    }
    if keys.just_pressed(KeyCode::Digit3) {
        nav.speed = 4;
        nav.message = "Speed 4x.".to_string();
    }
    if keys.just_pressed(KeyCode::KeyS) {
        apply_view_result(
            state.app.fight_command(FightCommand::Step, Some(1)),
            state,
            nav,
            "Stepped one second.",
        );
    }
    if keys.just_pressed(KeyCode::KeyI) {
        apply_view_result(
            state
                .app
                .fight_command(FightCommand::SkipToNextInitiative, None),
            state,
            nav,
            "Skipped to next initiative.",
        );
    }
    if keys.just_pressed(KeyCode::KeyF) {
        apply_view_result(
            state.app.fight_command(FightCommand::Finish, None),
            state,
            nav,
            "Fight resolved.",
        );
    }
}

fn handle_reward_input(
    keys: &ButtonInput<KeyCode>,
    state: &mut VisualGameState,
    nav: &mut VisualNav,
) {
    let offer_count = state.view.recruit_offer.len();
    if offer_count == 0 {
        nav.screen = screen_from_view(&state.view);
        nav.dirty = true;
        return;
    }
    if keys.just_pressed(KeyCode::ArrowLeft) {
        nav.selected_recruit_index = nav
            .selected_recruit_index
            .checked_sub(1)
            .unwrap_or(offer_count.saturating_sub(1));
        nav.dirty = true;
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        nav.selected_recruit_index = (nav.selected_recruit_index + 1) % offer_count;
        nav.dirty = true;
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        nav.selected_active_index = nav.selected_active_index.saturating_sub(1);
        nav.dirty = true;
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        nav.selected_active_index =
            (nav.selected_active_index + 1).min(state.view.squad.active.len().saturating_sub(1));
        nav.dirty = true;
    }

    let candidate_id = state
        .view
        .recruit_offer
        .get(
            nav.selected_recruit_index
                .min(offer_count.saturating_sub(1)),
        )
        .map(|member| member.id.clone());
    let Some(candidate_id) = candidate_id else {
        return;
    };

    if keys.just_pressed(KeyCode::KeyA) {
        apply_view_result(
            state
                .app
                .recruit_choice(candidate_id, RecruitDestination::Active, None),
            state,
            nav,
            "Recruit added to active squad.",
        );
    } else if keys.just_pressed(KeyCode::KeyB) {
        apply_view_result(
            state
                .app
                .recruit_choice(candidate_id, RecruitDestination::Bench, None),
            state,
            nav,
            "Recruit sent to bench.",
        );
    } else if keys.just_pressed(KeyCode::KeyD) {
        apply_view_result(
            state
                .app
                .recruit_choice(candidate_id, RecruitDestination::Decline, None),
            state,
            nav,
            "Recruit declined.",
        );
    } else if keys.just_pressed(KeyCode::KeyX) {
        let replace_id = state
            .view
            .squad
            .active
            .get(
                nav.selected_active_index
                    .min(state.view.squad.active.len().saturating_sub(1)),
            )
            .map(|member| member.id.clone());
        apply_view_result(
            state
                .app
                .recruit_choice(candidate_id, RecruitDestination::Replace, replace_id),
            state,
            nav,
            "Recruit replaced active member.",
        );
    }
}

fn handle_roster_input(
    keys: &ButtonInput<KeyCode>,
    state: &mut VisualGameState,
    nav: &mut VisualNav,
) {
    if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::KeyR) {
        nav.screen = nav.previous_screen;
        nav.dirty = true;
        return;
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        nav.selected_active_index = nav.selected_active_index.saturating_sub(1);
        nav.dirty = true;
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        nav.selected_active_index =
            (nav.selected_active_index + 1).min(state.view.squad.active.len().saturating_sub(1));
        nav.dirty = true;
    }
    if keys.just_pressed(KeyCode::ArrowLeft) {
        nav.selected_bench_index = nav.selected_bench_index.saturating_sub(1);
        nav.dirty = true;
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        nav.selected_bench_index =
            (nav.selected_bench_index + 1).min(state.view.squad.bench.len().saturating_sub(1));
        nav.dirty = true;
    }

    let active_id = state
        .view
        .squad
        .active
        .get(
            nav.selected_active_index
                .min(state.view.squad.active.len().saturating_sub(1)),
        )
        .map(|member| member.id.clone());
    let bench_id = state
        .view
        .squad
        .bench
        .get(
            nav.selected_bench_index
                .min(state.view.squad.bench.len().saturating_sub(1)),
        )
        .map(|member| member.id.clone());

    if keys.just_pressed(KeyCode::KeyP) {
        if let Some(bench_id) = bench_id.clone() {
            apply_view_result(
                state.app.roster_promote(bench_id),
                state,
                nav,
                "Bench member promoted.",
            );
        }
    } else if keys.just_pressed(KeyCode::KeyS) {
        if let (Some(active_id), Some(bench_id)) = (active_id, bench_id.clone()) {
            apply_view_result(
                state.app.roster_swap(active_id, bench_id),
                state,
                nav,
                "Roster members swapped.",
            );
        }
    } else if keys.just_pressed(KeyCode::KeyD) {
        if let Some(bench_id) = bench_id {
            apply_view_result(
                state.app.roster_dismiss(bench_id),
                state,
                nav,
                "Bench member dismissed.",
            );
        }
    }
}

fn advance_combat(
    time: Res<Time>,
    mut timer: ResMut<CombatTickTimer>,
    mut state: ResMut<VisualGameState>,
    mut nav: ResMut<VisualNav>,
) {
    if nav.screen != GameScreen::Combat || nav.paused {
        timer.0.tick(time.delta());
        return;
    }
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    if state
        .view
        .live_fight
        .as_ref()
        .is_none_or(|fight| fight.done)
    {
        return;
    }
    let seconds = nav.speed.max(1).min(4);
    apply_view_result(
        state.app.fight_command(FightCommand::Tick, Some(seconds)),
        &mut state,
        &mut nav,
        "Combat advances.",
    );
}

fn render_current_screen(
    mut commands: Commands,
    mut nav: ResMut<VisualNav>,
    state: Res<VisualGameState>,
    screen_entities: Query<Entity, With<ScreenEntity>>,
    board_entities: Query<Entity, With<board::BoardVisual>>,
    unit_entities: Query<Entity, With<units::UnitToken>>,
) {
    if !nav.dirty {
        return;
    }

    for entity in &screen_entities {
        commands.entity(entity).despawn_recursive();
    }
    for entity in &board_entities {
        commands.entity(entity).despawn_recursive();
    }
    for entity in &unit_entities {
        commands.entity(entity).despawn_recursive();
    }

    let screen_changed = nav.rendered_screen != Some(nav.screen);
    if screen_changed {
        for entity in &board_entities {
            commands.entity(entity).despawn_recursive();
        }
        for entity in &unit_entities {
            commands.entity(entity).despawn_recursive();
        }
    }

    if screen_changed && nav.screen == GameScreen::Combat {
        if let Some(fight) = state.view.live_fight.as_ref() {
            let geometry = BoardGeometry::new(fight.grid);
            commands.insert_resource(geometry);
            board::spawn_board(&mut commands, geometry);
            units::spawn_units(&mut commands, geometry, fight);
        }
    }

    spawn_screen_text(&mut commands, &state.view, &nav);
    nav.rendered_screen = Some(nav.screen);
    nav.dirty = false;
}

fn spawn_screen_text(commands: &mut Commands, view: &SquadBattlerView, nav: &VisualNav) {
    spawn_text(
        commands,
        header_text(view, nav),
        UiRect::new(Val::Px(22.0), Val::Auto, Val::Px(16.0), Val::Auto),
        24.0,
        Color::rgb(1.0, 0.86, 0.48),
    );
    let body = match nav.screen {
        GameScreen::MainMenu => main_menu_text(),
        GameScreen::Route => route_text(view, nav),
        GameScreen::FightPreview => fight_preview_text(view),
        GameScreen::Combat => combat_text(view, nav),
        GameScreen::RewardReview => reward_text(view, nav),
        GameScreen::Roster => roster_text(view, nav),
        GameScreen::RunOver => run_over_text(view),
    };
    spawn_text(
        commands,
        body,
        UiRect::new(Val::Px(22.0), Val::Auto, Val::Px(58.0), Val::Auto),
        17.0,
        Color::rgb(0.94, 0.84, 0.68),
    );
}

fn spawn_text(commands: &mut Commands, text: String, margin: UiRect, font_size: f32, color: Color) {
    commands.spawn((
        ScreenEntity,
        TextBundle {
            text: Text::from_section(
                text,
                TextStyle {
                    font_size,
                    color,
                    ..default()
                },
            ),
            style: Style {
                position_type: PositionType::Absolute,
                margin,
                max_width: Val::Percent(92.0),
                ..default()
            },
            ..default()
        },
    ));
}

fn start_new_run(state: &mut VisualGameState, nav: &mut VisualNav) {
    state.view = state.app.new_run(Some(DEMO_SEED));
    nav.screen = GameScreen::Route;
    nav.previous_screen = GameScreen::Route;
    nav.selected_node_index = 0;
    nav.selected_recruit_index = 0;
    nav.selected_active_index = 0;
    nav.selected_bench_index = 0;
    nav.paused = false;
    nav.message = "Squad rolled. Choose a route node.".to_string();
    nav.dirty = true;
}

fn apply_view_result(
    result: Result<SquadBattlerView, String>,
    state: &mut VisualGameState,
    nav: &mut VisualNav,
    ok_message: &str,
) {
    match result {
        Ok(view) => {
            state.view = view;
            nav.screen = screen_from_view(&state.view);
            nav.message = ok_message.to_string();
            nav.selected_node_index = nav
                .selected_node_index
                .min(state.view.available_nodes.len().saturating_sub(1));
            nav.selected_recruit_index = nav
                .selected_recruit_index
                .min(state.view.recruit_offer.len().saturating_sub(1));
            nav.selected_active_index = nav
                .selected_active_index
                .min(state.view.squad.active.len().saturating_sub(1));
            nav.selected_bench_index = nav
                .selected_bench_index
                .min(state.view.squad.bench.len().saturating_sub(1));
            nav.dirty = true;
        }
        Err(err) => {
            nav.message = err;
            nav.dirty = true;
        }
    }
}

fn screen_from_view(view: &SquadBattlerView) -> GameScreen {
    if !view.has_run {
        return GameScreen::MainMenu;
    }
    match view.phase.as_str() {
        "choose_node" => GameScreen::Route,
        "fight_preview" => GameScreen::FightPreview,
        "combat_playback" => GameScreen::Combat,
        "reward_review" => GameScreen::RewardReview,
        "run_over" => GameScreen::RunOver,
        _ => GameScreen::Route,
    }
}

fn selected_node_id(view: &SquadBattlerView, nav: &VisualNav) -> Option<usize> {
    view.available_nodes
        .get(
            nav.selected_node_index
                .min(view.available_nodes.len().saturating_sub(1)),
        )
        .copied()
}

fn header_text(view: &SquadBattlerView, nav: &VisualNav) -> String {
    format!(
        "{} | {:?} | Depth {} | Gold {} | {}",
        view.title, nav.screen, view.depth, view.gold, nav.message
    )
}

fn main_menu_text() -> String {
    [
        "HACKMASTER SQUAD BATTLER",
        "",
        "N  Roll a new company",
        "",
        "This Bevy client is now the playable game surface. The browser demo remains a debug tool.",
    ]
    .join("\n")
}

fn route_text(view: &SquadBattlerView, nav: &VisualNav) -> String {
    let selected = selected_node_id(view, nav);
    let mut lines = vec![
        "ROUTE".to_string(),
        "Left/Right select node | Enter choose | R roster | N restart".to_string(),
        "".to_string(),
    ];
    for node in &view.route {
        lines.push(route_node_line(node, selected, &view.available_nodes));
    }
    lines.push("".to_string());
    lines.push("Active Squad".to_string());
    for member in &view.squad.active {
        lines.push(member_line(member, false));
    }
    lines.join("\n")
}

fn route_node_line(node: &SquadRouteNode, selected: Option<usize>, available: &[usize]) -> String {
    let cursor = if selected == Some(node.id) { ">" } else { " " };
    let status = if node.completed {
        "done"
    } else if available.contains(&node.id) {
        "open"
    } else {
        "locked"
    };
    format!(
        "{cursor} Floor {} Lane {}  #{:02} {:<7} {:<6} {}",
        node.floor + 1,
        node.lane + 1,
        node.id,
        node_kind_label(node.kind),
        status,
        tier_label(node.tier)
    )
}

fn fight_preview_text(view: &SquadBattlerView) -> String {
    let mut lines = vec![
        "FIGHT PREVIEW".to_string(),
        "Enter start fight | N restart".to_string(),
        "".to_string(),
        "Company".to_string(),
    ];
    for member in &view.squad.active {
        lines.push(member_line(member, false));
    }
    lines.push("".to_string());
    if let Some(fight) = view.pending_fight.as_ref() {
        lines.push(format!(
            "Enemy Squad: {} ({})",
            fight.tier, fight.enemy_count
        ));
        for enemy in &fight.enemies {
            lines.push(format!("  {}  L{}", enemy.name, enemy.level));
        }
    }
    lines.join("\n")
}

fn combat_text(view: &SquadBattlerView, nav: &VisualNav) -> String {
    let Some(fight) = view.live_fight.as_ref() else {
        return "Combat is loading.".to_string();
    };
    let mut lines = vec![
        format!(
            "COMBAT  {}s/{}s  Speed {}x  {}",
            fight.elapsed_seconds,
            fight.max_seconds,
            nav.speed,
            if nav.paused { "Paused" } else { "Running" }
        ),
        "Space pause | 1/2/3 speed | S step | I skip to initiative | F finish".to_string(),
        "".to_string(),
        "Initiative".to_string(),
    ];
    for item in fight.initiative.iter().take(8) {
        lines.push(format!(
            "  {:<18} team {}  {}",
            item.name,
            item.team_id,
            if item.ready {
                "ready".to_string()
            } else {
                format!("{:.0}s", item.next_action_in_seconds)
            }
        ));
    }
    lines.push("".to_string());
    lines.push("Recent Log".to_string());
    for log in fight.log_tail.iter().rev().take(8) {
        lines.push(format!("  {log}"));
    }
    lines.join("\n")
}

fn reward_text(view: &SquadBattlerView, nav: &VisualNav) -> String {
    let mut lines = vec![
        "REWARDS".to_string(),
        "Left/Right select recruit | A active | B bench | X replace selected active | D decline"
            .to_string(),
        "".to_string(),
    ];
    if let Some(reward) = view.last_reward.as_ref() {
        lines.push(format!(
            "Gold +{} | XP +{} per survivor",
            reward.gold, reward.xp_per_survivor
        ));
        if !reward.deaths.is_empty() {
            lines.push(format!("Lost: {}", reward.deaths.join(", ")));
        }
        if !reward.level_ups.is_empty() {
            lines.push(format!("Level-ups: {}", reward.level_ups.join(", ")));
        }
    }
    if !view.recruit_offer.is_empty() {
        lines.push("".to_string());
        lines.push("Recruit Offers".to_string());
        for (idx, recruit) in view.recruit_offer.iter().enumerate() {
            lines.push(member_line(recruit, idx == nav.selected_recruit_index));
        }
        lines.push("".to_string());
        lines.push("Replace Target".to_string());
        for (idx, member) in view.squad.active.iter().enumerate() {
            lines.push(member_line(member, idx == nav.selected_active_index));
        }
    }
    lines.join("\n")
}

fn roster_text(view: &SquadBattlerView, nav: &VisualNav) -> String {
    let mut lines = vec![
        "ROSTER".to_string(),
        "Up/Down active | Left/Right bench | P promote | S swap | D dismiss | R/Esc back"
            .to_string(),
        "".to_string(),
        format!(
            "Active Squad ({}/{})",
            view.squad.active.len(),
            view.squad.max_active
        ),
    ];
    for (idx, member) in view.squad.active.iter().enumerate() {
        lines.push(member_line(member, idx == nav.selected_active_index));
    }
    lines.push("".to_string());
    lines.push(format!(
        "Bench ({}/{})",
        view.squad.bench.len(),
        view.squad.max_bench
    ));
    for (idx, member) in view.squad.bench.iter().enumerate() {
        lines.push(member_line(member, idx == nav.selected_bench_index));
    }
    lines.join("\n")
}

fn run_over_text(view: &SquadBattlerView) -> String {
    format!(
        "RUN OVER\n\n{}\n\nN  Start another run",
        view.terminal
            .clone()
            .unwrap_or_else(|| "The route is complete.".to_string())
    )
}

fn member_line(member: &SquadMemberView, selected: bool) -> String {
    let cursor = if selected { ">" } else { " " };
    let level_marker = if member.level_up_available { " +" } else { "" };
    format!(
        "{cursor} {:<18} L{}{} HP {}/{} {:<12} {} {:?}",
        member.name,
        member.level,
        level_marker,
        member.hp,
        member.max_hp,
        member.weapon,
        member.rarity.label(),
        member.status
    )
}

fn node_kind_label(kind: SquadNodeKind) -> &'static str {
    match kind {
        SquadNodeKind::Fight => "fight",
        SquadNodeKind::Recruit => "recruit",
        SquadNodeKind::Event => "event",
        SquadNodeKind::Elite => "elite",
        SquadNodeKind::Boss => "boss",
        SquadNodeKind::Rest => "rest",
    }
}

fn tier_label(tier: crate::squad_battler::encounters::SquadEncounterTier) -> &'static str {
    match tier {
        crate::squad_battler::encounters::SquadEncounterTier::Normal => "normal",
        crate::squad_battler::encounters::SquadEncounterTier::Elite => "elite",
        crate::squad_battler::encounters::SquadEncounterTier::Boss => "boss",
    }
}
