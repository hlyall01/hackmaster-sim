use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::render::view::screenshot::ScreenshotManager;
use bevy::window::{PrimaryWindow, WindowResizeConstraints};
use std::fs;

use crate::squad_battler::combat::BattleGrid;
use crate::squad_battler::encounters::SquadNodeKind;
use crate::squad_battler::rewards::{RecruitDestination, SquadReward};
use crate::squad_battler::roster::SquadMemberView;
use crate::squad_battler::state::{FightCommand, SquadBattlerApp, SquadBattlerView};

use super::assets;
use super::board::{self, BoardGeometry};
use super::camera;
use super::combat_fx;
use super::fight_preview::{self, BackHint, FormationMoveRequest, StartFightHint};
use super::hud::{self, HudAction, SquadBattlerHudButton};
use super::rewards::{self, RewardScreenVisible, RewardUiEvent};
use super::roster_ui::{self, RosterActionRequested, RosterUiVisible};
use super::route::{self, RouteMapVisible, SelectedRouteNode};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScreenshotTourStage {
    MainMenu,
    StartRun,
    Route,
    OpenRoster,
    Roster,
    OpenFightPreview,
    FightPreview,
    StartCombat,
    Combat,
    FinishCombat,
    Reward,
    ForceRunOver,
    RunOver,
    Done,
}

#[derive(Resource)]
struct ScreenshotTour {
    enabled: bool,
    stage: ScreenshotTourStage,
    wait_frames: u8,
    output_dir: String,
}

impl ScreenshotTour {
    fn from_env() -> Self {
        Self {
            enabled: std::env::var_os("SQUAD_BATTLER_SCREENSHOTS").is_some(),
            stage: ScreenshotTourStage::MainMenu,
            wait_frames: 18,
            output_dir: std::env::var("SQUAD_BATTLER_SCREENSHOT_DIR")
                .unwrap_or_else(|_| "screenshots/squad_battler".to_string()),
        }
    }
}

#[derive(Component)]
struct ScreenEntity;

#[derive(Component)]
enum ScreenAction {
    NewRun,
    OpenRoster,
    CloseRoster,
}

pub fn run() {
    App::new()
        .insert_resource(ClearColor(assets::clear_color()))
        .insert_resource(BoardGeometry::new(BattleGrid::default()))
        .insert_resource(CombatTickTimer(Timer::from_seconds(
            1.0,
            TimerMode::Repeating,
        )))
        .insert_resource(VisualNav::default())
        .insert_resource(ScreenshotTour::from_env())
        .init_resource::<fight_preview::FormationDragState>()
        .add_event::<FormationMoveRequest>()
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
        .add_plugins(route::RouteMapPlugin)
        .add_plugins(rewards::RewardScreenPlugin)
        .add_plugins(roster_ui::RosterUiPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                sync_auxiliary_screen_visibility,
                handle_keyboard_input,
                handle_screen_buttons,
                consume_route_requests,
                handle_fight_preview_buttons,
                fight_preview::handle_formation_drag,
                handle_formation_move_requests,
                handle_hud_buttons,
                handle_reward_events,
                handle_roster_events,
                render_current_screen,
                hud::sync_hud,
                camera::fit_camera_to_board,
                advance_combat,
                units::sync_unit_targets,
                units::animate_unit_motion,
                units::animate_stick_figures,
            ),
        )
        .add_systems(
            Update,
            run_screenshot_tour
                .after(render_current_screen)
                .after(hud::sync_hud)
                .after(units::animate_unit_motion),
        )
        .run();
}

fn setup(mut commands: Commands) {
    let app = SquadBattlerApp::new().expect("failed to create squad battler app");
    let view = app.view();
    camera::spawn_camera(&mut commands);
    commands.insert_resource(VisualGameState { app, view });
}

fn sync_auxiliary_screen_visibility(
    nav: Res<VisualNav>,
    mut route_visible: ResMut<RouteMapVisible>,
    mut rewards_visible: ResMut<RewardScreenVisible>,
    mut roster_visible: ResMut<RosterUiVisible>,
) {
    route_visible.0 = nav.screen == GameScreen::Route;
    rewards_visible.0 = nav.screen == GameScreen::RewardReview;
    roster_visible.0 = nav.screen == GameScreen::Roster;
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
        GameScreen::Route => handle_route_input(&keys, &mut nav),
        GameScreen::FightPreview => handle_fight_preview_input(&keys, &mut state, &mut nav),
        GameScreen::Combat => handle_combat_input(&keys, &mut state, &mut nav),
        GameScreen::RewardReview => handle_reward_input(&keys, &mut state, &mut nav),
        GameScreen::Roster => handle_roster_input(&keys, &mut state, &mut nav),
        GameScreen::RunOver => {}
    }
}

fn handle_screen_buttons(
    mut nav: ResMut<VisualNav>,
    mut state: ResMut<VisualGameState>,
    buttons: Query<(&Interaction, &ScreenAction), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            ScreenAction::NewRun => start_new_run(&mut state, &mut nav),
            ScreenAction::OpenRoster => {
                nav.previous_screen = nav.screen;
                nav.screen = GameScreen::Roster;
                nav.message = "Manage the company roster.".to_string();
                nav.dirty = true;
            }
            ScreenAction::CloseRoster => {
                nav.screen = nav.previous_screen;
                nav.message = "Roster closed.".to_string();
                nav.dirty = true;
            }
        }
    }
}

fn consume_route_requests(
    mut selected: ResMut<SelectedRouteNode>,
    mut state: ResMut<VisualGameState>,
    mut nav: ResMut<VisualNav>,
) {
    if nav.screen != GameScreen::Route {
        selected.requested_node_id = None;
        return;
    }
    let Some(node_id) = selected.take_requested() else {
        return;
    };
    apply_view_result(
        state.app.choose_node(node_id),
        &mut state,
        &mut nav,
        "Encounter selected.",
    );
}

fn handle_fight_preview_buttons(
    start_buttons: Query<&Interaction, (Changed<Interaction>, With<StartFightHint>)>,
    back_buttons: Query<&Interaction, (Changed<Interaction>, With<BackHint>)>,
    mut state: ResMut<VisualGameState>,
    mut nav: ResMut<VisualNav>,
) {
    if nav.screen != GameScreen::FightPreview {
        return;
    }
    if start_buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        apply_view_result(
            state.app.start_fight(),
            &mut state,
            &mut nav,
            "Fight started.",
        );
    }
    if back_buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        nav.message = "Back is disabled once the encounter is previewed.".to_string();
        nav.dirty = true;
    }
}

fn handle_formation_move_requests(
    mut events: EventReader<FormationMoveRequest>,
    mut state: ResMut<VisualGameState>,
    mut nav: ResMut<VisualNav>,
) {
    if nav.screen != GameScreen::FightPreview {
        events.clear();
        return;
    }
    for event in events.read() {
        apply_view_result(
            state
                .app
                .set_formation_position(event.member_id.clone(), event.x, event.y),
            &mut state,
            &mut nav,
            "Formation updated.",
        );
    }
}

fn handle_hud_buttons(
    buttons: Query<(&Interaction, &SquadBattlerHudButton), (Changed<Interaction>, With<Button>)>,
    mut state: ResMut<VisualGameState>,
    mut nav: ResMut<VisualNav>,
) {
    if nav.screen != GameScreen::Combat {
        return;
    }
    for (interaction, button) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let HudAction::Fight(command) = button.action;
        match command {
            FightCommand::Play => nav.paused = false,
            FightCommand::Pause => nav.paused = true,
            _ => {}
        }
        apply_view_result(
            state.app.fight_command(command, None),
            &mut state,
            &mut nav,
            "Combat command applied.",
        );
    }
}

fn handle_reward_events(
    mut events: EventReader<RewardUiEvent>,
    mut state: ResMut<VisualGameState>,
    mut nav: ResMut<VisualNav>,
) {
    if nav.screen != GameScreen::RewardReview {
        events.clear();
        return;
    }
    for event in events.read() {
        match event {
            RewardUiEvent::ClaimReward | RewardUiEvent::Continue => {
                nav.screen = screen_from_view(&state.view);
                nav.message = "Rewards closed.".to_string();
                nav.dirty = true;
            }
            RewardUiEvent::RecruitChoice {
                candidate_id,
                destination,
                replace_member_id,
            } => {
                apply_view_result(
                    state.app.recruit_choice(
                        candidate_id.clone(),
                        *destination,
                        replace_member_id.clone(),
                    ),
                    &mut state,
                    &mut nav,
                    "Recruit decision applied.",
                );
                if !state.view.recruit_offer.is_empty() {
                    nav.screen = GameScreen::RewardReview;
                    nav.dirty = true;
                }
            }
        }
    }
}

fn handle_roster_events(
    mut events: EventReader<RosterActionRequested>,
    mut state: ResMut<VisualGameState>,
    mut nav: ResMut<VisualNav>,
) {
    if nav.screen != GameScreen::Roster {
        events.clear();
        return;
    }
    for event in events.read() {
        match event {
            RosterActionRequested::Promote { bench_member_id } => apply_view_result(
                state.app.roster_promote(bench_member_id.clone()),
                &mut state,
                &mut nav,
                "Bench member promoted.",
            ),
            RosterActionRequested::Swap {
                active_member_id,
                bench_member_id,
            } => apply_view_result(
                state
                    .app
                    .roster_swap(active_member_id.clone(), bench_member_id.clone()),
                &mut state,
                &mut nav,
                "Roster members swapped.",
            ),
            RosterActionRequested::Dismiss { bench_member_id } => apply_view_result(
                state.app.roster_dismiss(bench_member_id.clone()),
                &mut state,
                &mut nav,
                "Bench member dismissed.",
            ),
        }
        nav.screen = GameScreen::Roster;
        nav.dirty = true;
    }
}

fn handle_route_input(keys: &ButtonInput<KeyCode>, nav: &mut VisualNav) {
    if keys.just_pressed(KeyCode::KeyR) {
        nav.previous_screen = nav.screen;
        nav.screen = GameScreen::Roster;
        nav.dirty = true;
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

fn run_screenshot_tour(
    mut tour: ResMut<ScreenshotTour>,
    mut state: ResMut<VisualGameState>,
    mut nav: ResMut<VisualNav>,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut screenshot_manager: ResMut<ScreenshotManager>,
    mut app_exit: EventWriter<AppExit>,
) {
    if !tour.enabled {
        return;
    }
    if tour.wait_frames > 0 {
        tour.wait_frames -= 1;
        return;
    }
    if nav.dirty {
        return;
    }

    match tour.stage {
        ScreenshotTourStage::MainMenu => {
            request_tour_screenshot(&tour, &windows, &mut screenshot_manager, "01_main_menu.png");
            tour.stage = ScreenshotTourStage::StartRun;
            tour.wait_frames = 18;
        }
        ScreenshotTourStage::StartRun => {
            start_new_run(&mut state, &mut nav);
            tour.stage = ScreenshotTourStage::Route;
            tour.wait_frames = 18;
        }
        ScreenshotTourStage::Route => {
            request_tour_screenshot(&tour, &windows, &mut screenshot_manager, "02_route.png");
            tour.stage = ScreenshotTourStage::OpenRoster;
            tour.wait_frames = 18;
        }
        ScreenshotTourStage::OpenRoster => {
            nav.previous_screen = GameScreen::Route;
            nav.screen = GameScreen::Roster;
            nav.message = "Manage the company roster.".to_string();
            nav.dirty = true;
            tour.stage = ScreenshotTourStage::Roster;
            tour.wait_frames = 18;
        }
        ScreenshotTourStage::Roster => {
            request_tour_screenshot(&tour, &windows, &mut screenshot_manager, "03_roster.png");
            tour.stage = ScreenshotTourStage::OpenFightPreview;
            tour.wait_frames = 18;
        }
        ScreenshotTourStage::OpenFightPreview => {
            nav.screen = GameScreen::Route;
            let Some(node_id) = first_available_fight_node(&state.view) else {
                warn!("screenshot tour could not find an available fight node");
                tour.stage = ScreenshotTourStage::ForceRunOver;
                tour.wait_frames = 18;
                return;
            };
            apply_view_result(
                state.app.choose_node(node_id),
                &mut state,
                &mut nav,
                "Encounter selected.",
            );
            tour.stage = ScreenshotTourStage::FightPreview;
            tour.wait_frames = 18;
        }
        ScreenshotTourStage::FightPreview => {
            request_tour_screenshot(
                &tour,
                &windows,
                &mut screenshot_manager,
                "04_fight_preview.png",
            );
            tour.stage = ScreenshotTourStage::StartCombat;
            tour.wait_frames = 18;
        }
        ScreenshotTourStage::StartCombat => {
            apply_view_result(
                state.app.start_fight(),
                &mut state,
                &mut nav,
                "Fight started.",
            );
            tour.stage = ScreenshotTourStage::Combat;
            tour.wait_frames = 24;
        }
        ScreenshotTourStage::Combat => {
            request_tour_screenshot(&tour, &windows, &mut screenshot_manager, "05_combat.png");
            tour.stage = ScreenshotTourStage::FinishCombat;
            tour.wait_frames = 18;
        }
        ScreenshotTourStage::FinishCombat => {
            state.view.phase = "reward_review".to_string();
            state.view.live_fight = None;
            state.view.last_reward = Some(SquadReward {
                gold: 18,
                xp_per_survivor: 24,
                deaths: Vec::new(),
                level_ups: Vec::new(),
            });
            state.view.recruit_offer.clear();
            nav.screen = GameScreen::RewardReview;
            nav.message = "Review the after-action rewards.".to_string();
            nav.dirty = true;
            tour.stage = ScreenshotTourStage::Reward;
            tour.wait_frames = 24;
        }
        ScreenshotTourStage::Reward => {
            request_tour_screenshot(&tour, &windows, &mut screenshot_manager, "06_rewards.png");
            tour.stage = ScreenshotTourStage::ForceRunOver;
            tour.wait_frames = 18;
        }
        ScreenshotTourStage::ForceRunOver => {
            state.view.phase = "run_over".to_string();
            state.view.terminal = Some("Screenshot tour complete.".to_string());
            state.view.live_fight = None;
            state.view.pending_fight = None;
            nav.screen = GameScreen::RunOver;
            nav.dirty = true;
            tour.stage = ScreenshotTourStage::RunOver;
            tour.wait_frames = 18;
        }
        ScreenshotTourStage::RunOver => {
            request_tour_screenshot(&tour, &windows, &mut screenshot_manager, "07_run_over.png");
            tour.stage = ScreenshotTourStage::Done;
            tour.wait_frames = 60;
        }
        ScreenshotTourStage::Done => {
            app_exit.send(AppExit);
        }
    }
}

fn request_tour_screenshot(
    tour: &ScreenshotTour,
    windows: &Query<Entity, With<PrimaryWindow>>,
    screenshot_manager: &mut ScreenshotManager,
    file_name: &str,
) {
    if let Err(err) = fs::create_dir_all(&tour.output_dir) {
        warn!(
            "screenshot tour could not create {}: {err}",
            tour.output_dir
        );
        return;
    }
    let Ok(window) = windows.get_single() else {
        warn!("screenshot tour could not find the primary window");
        return;
    };
    let path = format!("{}/{}", tour.output_dir, file_name);
    if let Err(err) = screenshot_manager.save_screenshot_to_disk(window, &path) {
        warn!("screenshot tour request failed for {path}: {err:?}");
    } else {
        info!("Screenshot tour requested {path}");
    }
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

    let screen_changed = nav.rendered_screen != Some(nav.screen);
    let board_missing = board_entities.is_empty();
    let units_missing = unit_entities.is_empty();

    for entity in &screen_entities {
        commands.entity(entity).despawn_recursive();
    }
    if screen_changed {
        for entity in &board_entities {
            commands.entity(entity).despawn_recursive();
        }
        for entity in &unit_entities {
            commands.entity(entity).despawn_recursive();
        }
    }

    if nav.screen == GameScreen::Combat && (screen_changed || board_missing || units_missing) {
        if let Some(fight) = state.view.live_fight.as_ref() {
            let geometry = BoardGeometry::new(fight.grid);
            commands.insert_resource(geometry);
            if screen_changed || board_missing {
                board::spawn_board(&mut commands, geometry);
            }
            if screen_changed || units_missing {
                units::spawn_units(&mut commands, geometry, fight);
            }
        }
    }

    match nav.screen {
        GameScreen::MainMenu => spawn_main_menu(&mut commands, &state.view),
        GameScreen::Route => spawn_route_shell(&mut commands, &state.view, &nav),
        GameScreen::FightPreview => {
            if let Some(root) =
                fight_preview::spawn_fight_preview(&mut commands, &state.view, default_font())
            {
                commands.entity(root).insert(ScreenEntity);
            }
            if let Some(root) =
                fight_preview::spawn_formation_board(&mut commands, &state.view, default_font())
            {
                commands.entity(root).insert(ScreenEntity);
            }
        }
        GameScreen::Combat | GameScreen::RewardReview => {}
        GameScreen::Roster => spawn_roster_frame(&mut commands, &state.view, &nav),
        GameScreen::RunOver => spawn_run_over_panel(&mut commands, &state.view),
    }

    nav.rendered_screen = Some(nav.screen);
    nav.dirty = false;
}

fn spawn_main_menu(commands: &mut Commands, view: &SquadBattlerView) {
    commands
        .spawn((
            ScreenEntity,
            NodeBundle {
                style: full_screen_style(),
                background_color: Color::rgba(0.025, 0.019, 0.014, 0.72).into(),
                z_index: ZIndex::Global(15),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(screen_panel(
                Val::Px(620.0),
                Val::Auto,
                FlexDirection::Column,
            ))
            .with_children(|panel| {
                spawn_ui_text(panel, view.title.clone(), 38.0, title_color());
                spawn_ui_text(
                    panel,
                    "Build a company, choose a route, and watch the squads clash on the grid."
                        .to_string(),
                    18.0,
                    body_color(),
                );
                spawn_screen_button(panel, "Roll Company", ScreenAction::NewRun, 238.0);
                spawn_ui_text(
                    panel,
                    "The browser demo remains available as a debug surface.".to_string(),
                    13.0,
                    muted_color(),
                );
            });
        });
}

fn spawn_route_shell(commands: &mut Commands, view: &SquadBattlerView, nav: &VisualNav) {
    commands
        .spawn((
            ScreenEntity,
            NodeBundle {
                style: full_screen_style(),
                background_color: Color::NONE.into(),
                z_index: ZIndex::Global(12),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(16.0),
                    right: Val::Px(16.0),
                    top: Val::Px(14.0),
                    height: Val::Px(58.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(18.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: panel_color().into(),
                border_color: BorderColor(border_color()),
                ..default()
            })
            .with_children(|bar| {
                spawn_ui_text(
                    bar,
                    format!("Depth {}   Gold {}", view.depth, view.gold),
                    22.0,
                    title_color(),
                );
                spawn_ui_text(
                    bar,
                    phase_title(&view.phase).to_string(),
                    18.0,
                    body_color(),
                );
                spawn_screen_button(bar, "Roster", ScreenAction::OpenRoster, 132.0);
            });

            root.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(16.0),
                    top: Val::Px(88.0),
                    bottom: Val::Px(16.0),
                    width: Val::Px(286.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    overflow: Overflow::clip_y(),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: panel_color().into(),
                border_color: BorderColor(border_color()),
                ..default()
            })
            .with_children(|panel| {
                spawn_ui_text(panel, "Company".to_string(), 22.0, title_color());
                for member in &view.squad.active {
                    spawn_member_summary(panel, member);
                }
                if view.squad.bench.is_empty() {
                    spawn_ui_text(panel, "Bench empty".to_string(), 13.0, muted_color());
                } else {
                    spawn_ui_text(
                        panel,
                        format!("Bench: {}/{}", view.squad.bench.len(), view.squad.max_bench),
                        13.0,
                        muted_color(),
                    );
                }
            });

            root.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(322.0),
                    right: Val::Px(18.0),
                    bottom: Val::Px(16.0),
                    min_height: Val::Px(50.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(16.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: panel_color().into(),
                border_color: BorderColor(border_color()),
                ..default()
            })
            .with_children(|bar| {
                spawn_ui_text(bar, nav.message.clone(), 16.0, Color::rgb(0.94, 0.84, 0.68));
                spawn_ui_text(
                    bar,
                    "Click a glowing node, or use arrows and Enter.".to_string(),
                    14.0,
                    muted_color(),
                );
            });
        });
}

fn spawn_roster_frame(commands: &mut Commands, _view: &SquadBattlerView, nav: &VisualNav) {
    commands
        .spawn((
            ScreenEntity,
            NodeBundle {
                style: full_screen_style(),
                background_color: Color::rgba(0.015, 0.011, 0.008, 0.55).into(),
                z_index: ZIndex::Global(18),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(18.0),
                    top: Val::Px(18.0),
                    width: Val::Px(360.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    padding: UiRect::all(Val::Px(14.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: panel_color().into(),
                border_color: BorderColor(border_color()),
                ..default()
            })
            .with_children(|panel| {
                spawn_ui_text(panel, "Manage Company".to_string(), 26.0, title_color());
                spawn_ui_text(panel, nav.message.clone(), 15.0, body_color());
                spawn_screen_button(panel, "Back", ScreenAction::CloseRoster, 132.0);
            });
        });
}

fn spawn_run_over_panel(commands: &mut Commands, view: &SquadBattlerView) {
    commands
        .spawn((
            ScreenEntity,
            NodeBundle {
                style: full_screen_style(),
                background_color: Color::rgba(0.025, 0.019, 0.014, 0.82).into(),
                z_index: ZIndex::Global(24),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(screen_panel(
                Val::Px(560.0),
                Val::Auto,
                FlexDirection::Column,
            ))
            .with_children(|panel| {
                spawn_ui_text(panel, "Run Over".to_string(), 34.0, title_color());
                spawn_ui_text(
                    panel,
                    view.terminal
                        .clone()
                        .unwrap_or_else(|| "The route is complete.".to_string()),
                    18.0,
                    body_color(),
                );
                spawn_screen_button(panel, "New Run", ScreenAction::NewRun, 188.0);
            });
        });
}

fn spawn_member_summary(parent: &mut ChildBuilder, member: &SquadMemberView) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            background_color: Color::rgba(0.18, 0.125, 0.075, 0.86).into(),
            border_color: BorderColor(member_border_color(member)),
            ..default()
        })
        .with_children(|card| {
            spawn_ui_text(
                card,
                format!("{}  L{}", member.name, member.level),
                15.0,
                Color::rgb(0.96, 0.90, 0.76),
            );
            spawn_ui_text(
                card,
                format!(
                    "{} {}  HP {}/{}",
                    member.rarity.label(),
                    member.role.label(),
                    member.hp.max(0),
                    member.max_hp
                ),
                12.0,
                muted_color(),
            );
            card.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Px(6.0),
                    ..default()
                },
                background_color: Color::rgb(0.11, 0.07, 0.045).into(),
                ..default()
            })
            .with_children(|bar| {
                bar.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(member_hp_pct(member) * 100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    background_color: member_health_color(member).into(),
                    ..default()
                });
            });
        });
}

fn spawn_screen_button(parent: &mut ChildBuilder, label: &str, action: ScreenAction, width: f32) {
    parent
        .spawn((
            action,
            ButtonBundle {
                style: Style {
                    width: Val::Px(width),
                    height: Val::Px(42.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(12.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: Color::rgb(0.55, 0.17, 0.10).into(),
                border_color: BorderColor(Color::rgb(0.91, 0.58, 0.28)),
                ..default()
            },
        ))
        .with_children(|button| {
            spawn_ui_text(button, label.to_string(), 17.0, Color::rgb(1.0, 0.91, 0.72));
        });
}

fn spawn_ui_text(parent: &mut ChildBuilder, value: String, size: f32, color: Color) {
    parent.spawn(TextBundle {
        text: Text::from_section(
            value,
            TextStyle {
                font: default_font(),
                font_size: size,
                color,
            },
        ),
        style: Style {
            max_width: Val::Percent(100.0),
            ..default()
        },
        ..default()
    });
}

fn screen_panel(width: Val, height: Val, direction: FlexDirection) -> NodeBundle {
    NodeBundle {
        style: Style {
            width,
            height,
            flex_direction: direction,
            row_gap: Val::Px(16.0),
            padding: UiRect::all(Val::Px(24.0)),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        background_color: panel_color().into(),
        border_color: BorderColor(border_color()),
        ..default()
    }
}

fn full_screen_style() -> Style {
    Style {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        right: Val::Px(0.0),
        top: Val::Px(0.0),
        bottom: Val::Px(0.0),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

fn default_font() -> Handle<Font> {
    Handle::<Font>::default()
}

fn panel_color() -> Color {
    Color::rgba(0.08, 0.048, 0.026, 0.91)
}

fn border_color() -> Color {
    Color::rgba(0.93, 0.65, 0.28, 0.58)
}

fn title_color() -> Color {
    Color::rgb(0.98, 0.82, 0.44)
}

fn body_color() -> Color {
    Color::rgb(0.89, 0.78, 0.62)
}

fn muted_color() -> Color {
    Color::rgb(0.64, 0.55, 0.44)
}

fn phase_title(phase: &str) -> &'static str {
    match phase {
        "choose_node" => "Choose Route",
        "fight_preview" => "Scouting",
        "combat_playback" => "Combat",
        "reward_review" => "Rewards",
        "run_over" => "Run Over",
        _ => "Company",
    }
}

fn member_border_color(member: &SquadMemberView) -> Color {
    if member.level_up_available {
        return Color::rgb(0.98, 0.78, 0.30);
    }
    match member.status {
        crate::squad_battler::roster::SquadMemberStatus::Ready => {
            Color::rgba(0.65, 0.47, 0.24, 0.75)
        }
        crate::squad_battler::roster::SquadMemberStatus::Downed => {
            Color::rgba(0.70, 0.31, 0.21, 0.85)
        }
        crate::squad_battler::roster::SquadMemberStatus::Dead => {
            Color::rgba(0.26, 0.19, 0.16, 0.85)
        }
    }
}

fn member_hp_pct(member: &SquadMemberView) -> f32 {
    if member.max_hp <= 0 {
        return 0.0;
    }
    (member.hp.max(0) as f32 / member.max_hp as f32).clamp(0.0, 1.0)
}

fn member_health_color(member: &SquadMemberView) -> Color {
    let pct = member_hp_pct(member);
    if pct <= 0.33 {
        Color::rgb(0.75, 0.22, 0.16)
    } else if pct <= 0.66 {
        Color::rgb(0.88, 0.55, 0.23)
    } else {
        Color::rgb(0.58, 0.73, 0.38)
    }
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

fn first_available_fight_node(view: &SquadBattlerView) -> Option<usize> {
    view.available_nodes.iter().copied().find(|node_id| {
        view.route
            .iter()
            .find(|node| node.id == *node_id)
            .is_some_and(|node| {
                matches!(
                    node.kind,
                    SquadNodeKind::Fight | SquadNodeKind::Elite | SquadNodeKind::Boss
                )
            })
    })
}
