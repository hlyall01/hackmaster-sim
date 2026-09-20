use bevy::prelude::*;

use crate::squad_battler::rewards::{RecruitDestination, SquadReward};
use crate::squad_battler::roster::{SquadMemberView, SquadView};
use crate::squad_battler::state::SquadBattlerView;

use super::app::VisualGameState;

const PANEL_WIDTH: f32 = 390.0;
const CARD_GAP: f32 = 8.0;

#[derive(Component)]
pub struct RewardScreenRoot;

#[derive(Clone, Debug, Event)]
pub enum RewardUiEvent {
    ClaimReward,
    Continue,
    RecruitChoice {
        candidate_id: String,
        destination: RecruitDestination,
        replace_member_id: Option<String>,
    },
}

#[derive(Clone, Debug, Component)]
pub struct RewardActionButton {
    pub event: RewardUiEvent,
}

#[derive(Default, Resource)]
pub struct RewardScreenVisible(pub bool);

pub struct RewardScreenPlugin;

impl Plugin for RewardScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RewardScreenVisible>()
            .add_event::<RewardUiEvent>()
            .add_systems(Update, (sync_reward_screen, emit_reward_ui_events));
    }
}

pub fn sync_reward_screen(
    mut commands: Commands,
    state: Option<Res<VisualGameState>>,
    visible: Res<RewardScreenVisible>,
    roots: Query<Entity, With<RewardScreenRoot>>,
) {
    let Some(state) = state else {
        return;
    };

    if !visible.0 {
        for entity in &roots {
            commands.entity(entity).despawn_recursive();
        }
        return;
    }

    if !state.is_changed() && !visible.is_changed() && !roots.is_empty() {
        return;
    }

    for entity in &roots {
        commands.entity(entity).despawn_recursive();
    }

    if should_show_reward_screen(&state.view) {
        spawn_reward_screen(&mut commands, &state.view);
    }
}

pub fn emit_reward_ui_events(
    mut events: EventWriter<RewardUiEvent>,
    buttons: Query<(&Interaction, &RewardActionButton), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, button) in &buttons {
        if *interaction == Interaction::Pressed {
            events.send(button.event.clone());
        }
    }
}

pub fn spawn_reward_screen(commands: &mut Commands, view: &SquadBattlerView) -> Entity {
    commands
        .spawn((
            RewardScreenRoot,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    right: Val::Px(18.0),
                    top: Val::Px(18.0),
                    bottom: Val::Px(18.0),
                    width: Val::Px(PANEL_WIDTH),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    overflow: Overflow::clip_y(),
                    ..default()
                },
                background_color: panel_color().into(),
                z_index: ZIndex::Global(20),
                ..default()
            },
        ))
        .with_children(|parent| {
            spawn_header(parent, view);
            if let Some(reward) = &view.last_reward {
                spawn_reward_summary(parent, reward);
            }
            if view.recruit_offer.is_empty() {
                spawn_continue_controls(parent, view);
            } else {
                spawn_recruit_offer(parent, view);
            }
        })
        .id()
}

pub fn despawn_reward_screen(
    commands: &mut Commands,
    roots: &Query<Entity, With<RewardScreenRoot>>,
) {
    for entity in roots {
        commands.entity(entity).despawn_recursive();
    }
}

fn should_show_reward_screen(view: &SquadBattlerView) -> bool {
    view.last_reward.is_some() || !view.recruit_offer.is_empty()
}

fn spawn_header(parent: &mut ChildBuilder, view: &SquadBattlerView) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            spawn_text(parent, "After-action", 24.0, title_color());
            spawn_text(
                parent,
                &format!(
                    "Depth {} - Gold {} - {}",
                    view.depth,
                    view.gold,
                    phase_label(&view.phase)
                ),
                14.0,
                muted_text_color(),
            );
        });
}

fn spawn_reward_summary(parent: &mut ChildBuilder, reward: &SquadReward) {
    spawn_card(parent, |parent| {
        spawn_text(parent, "Spoils", 18.0, title_color());
        spawn_text(
            parent,
            &format!(
                "Gold +{} - XP per survivor +{}",
                reward.gold, reward.xp_per_survivor
            ),
            15.0,
            body_text_color(),
        );
        spawn_text(
            parent,
            &format!("Deaths: {}", list_or_none(&reward.deaths)),
            14.0,
            muted_text_color(),
        );
        spawn_text(
            parent,
            &format!("Level ups: {}", list_or_none(&reward.level_ups)),
            14.0,
            muted_text_color(),
        );
    });
}

fn spawn_continue_controls(parent: &mut ChildBuilder, view: &SquadBattlerView) {
    spawn_card(parent, |parent| {
        spawn_text(parent, continue_hint(view), 14.0, muted_text_color());
        spawn_button(
            parent,
            "Claim",
            Some(RewardUiEvent::ClaimReward),
            primary_button_color(),
        );
        spawn_button(
            parent,
            "Continue",
            Some(RewardUiEvent::Continue),
            secondary_button_color(),
        );
    });
}

fn spawn_recruit_offer(parent: &mut ChildBuilder, view: &SquadBattlerView) {
    spawn_text(
        parent,
        &format!("Recruit Offer - {} available", view.recruit_offer.len()),
        18.0,
        title_color(),
    );

    let active_full = view.squad.active.len() >= view.squad.max_active;
    let bench_full = view.squad.bench.len() >= view.squad.max_bench;

    for candidate in &view.recruit_offer {
        spawn_recruit_card(parent, candidate, &view.squad, active_full, bench_full);
    }
}

fn spawn_recruit_card(
    parent: &mut ChildBuilder,
    candidate: &SquadMemberView,
    squad: &SquadView,
    active_full: bool,
    bench_full: bool,
) {
    spawn_card(parent, |parent| {
        spawn_text(
            parent,
            &format!(
                "{}  {}/{}",
                candidate.name,
                candidate.hp,
                candidate.max_hp.max(1)
            ),
            17.0,
            title_color(),
        );
        spawn_text(
            parent,
            &format!(
                "{} {} - Lv {} - {}",
                candidate.rarity.label(),
                candidate.role.label(),
                candidate.level,
                candidate.weapon
            ),
            14.0,
            body_text_color(),
        );
        if !candidate.stats.is_empty() {
            spawn_text(
                parent,
                &candidate.stats.join(" - "),
                13.0,
                muted_text_color(),
            );
        }

        spawn_button_row(parent, |parent| {
            spawn_button(
                parent,
                if active_full { "Active full" } else { "Active" },
                (!active_full).then(|| RewardUiEvent::RecruitChoice {
                    candidate_id: candidate.id.clone(),
                    destination: RecruitDestination::Active,
                    replace_member_id: None,
                }),
                if active_full {
                    disabled_button_color()
                } else {
                    primary_button_color()
                },
            );
            spawn_button(
                parent,
                if bench_full { "Bench full" } else { "Bench" },
                (!bench_full).then(|| RewardUiEvent::RecruitChoice {
                    candidate_id: candidate.id.clone(),
                    destination: RecruitDestination::Bench,
                    replace_member_id: None,
                }),
                if bench_full {
                    disabled_button_color()
                } else {
                    secondary_button_color()
                },
            );
            spawn_button(
                parent,
                "Decline",
                Some(RewardUiEvent::RecruitChoice {
                    candidate_id: candidate.id.clone(),
                    destination: RecruitDestination::Decline,
                    replace_member_id: None,
                }),
                danger_button_color(),
            );
        });

        spawn_replace_controls(parent, candidate, &squad.active);
    });
}

fn spawn_replace_controls(
    parent: &mut ChildBuilder,
    candidate: &SquadMemberView,
    active: &[SquadMemberView],
) {
    if active.is_empty() {
        spawn_text(
            parent,
            "No active member to replace.",
            13.0,
            muted_text_color(),
        );
        return;
    }

    spawn_text(parent, "Replace active member", 13.0, muted_text_color());
    for member in active {
        spawn_button(
            parent,
            &format!("Replace {}", member.name),
            Some(RewardUiEvent::RecruitChoice {
                candidate_id: candidate.id.clone(),
                destination: RecruitDestination::Replace,
                replace_member_id: Some(member.id.clone()),
            }),
            replace_button_color(),
        );
    }
}

fn spawn_card(parent: &mut ChildBuilder, children: impl FnOnce(&mut ChildBuilder)) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(CARD_GAP),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            background_color: card_color().into(),
            ..default()
        })
        .with_children(children);
}

fn spawn_button_row(parent: &mut ChildBuilder, children: impl FnOnce(&mut ChildBuilder)) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(6.0),
                ..default()
            },
            ..default()
        })
        .with_children(children);
}

fn spawn_button(
    parent: &mut ChildBuilder,
    label: &str,
    event: Option<RewardUiEvent>,
    color: Color,
) {
    let mut entity = parent.spawn(ButtonBundle {
        style: Style {
            min_width: Val::Px(88.0),
            min_height: Val::Px(34.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(Val::Px(10.0)),
            ..default()
        },
        background_color: color.into(),
        ..default()
    });
    if let Some(event) = event {
        entity.insert(RewardActionButton { event });
    }
    entity.with_children(|parent| {
        spawn_text(parent, label, 13.0, Color::WHITE);
    });
}

fn spawn_text(parent: &mut ChildBuilder, value: &str, font_size: f32, color: Color) {
    parent.spawn(TextBundle::from_section(
        value.to_string(),
        TextStyle {
            font_size,
            color,
            ..default()
        },
    ));
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn continue_hint(view: &SquadBattlerView) -> &'static str {
    if view.terminal.is_some() || view.phase == "run_over" {
        "Run complete. Claim or continue to dismiss this panel."
    } else {
        "Rewards are ready. Claim or continue to return to route selection."
    }
}

fn phase_label(phase: &str) -> &str {
    match phase {
        "new_run" => "New run",
        "route_select" => "Route select",
        "fight_preview" => "Fight preview",
        "combat_playback" => "Combat",
        "reward_review" => "Reward review",
        "run_over" => "Run over",
        _ => phase,
    }
}

fn panel_color() -> Color {
    Color::rgba(0.08, 0.055, 0.035, 0.93)
}

fn card_color() -> Color {
    Color::rgba(0.18, 0.105, 0.055, 0.92)
}

fn title_color() -> Color {
    Color::rgb(1.0, 0.86, 0.52)
}

fn body_text_color() -> Color {
    Color::rgb(0.95, 0.88, 0.76)
}

fn muted_text_color() -> Color {
    Color::rgb(0.78, 0.68, 0.55)
}

fn primary_button_color() -> Color {
    Color::rgb(0.56, 0.32, 0.12)
}

fn secondary_button_color() -> Color {
    Color::rgb(0.29, 0.23, 0.17)
}

fn replace_button_color() -> Color {
    Color::rgb(0.33, 0.25, 0.14)
}

fn danger_button_color() -> Color {
    Color::rgb(0.42, 0.12, 0.09)
}

fn disabled_button_color() -> Color {
    Color::rgb(0.16, 0.13, 0.11)
}
