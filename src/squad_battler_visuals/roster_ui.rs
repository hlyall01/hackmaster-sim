use bevy::prelude::*;

use crate::squad_battler::roster::{SquadMemberStatus, SquadMemberView, SquadView};

use super::app::VisualGameState;

const PANEL_WIDTH: f32 = 340.0;
const CARD_HEIGHT: f32 = 82.0;
const BUTTON_HEIGHT: f32 = 24.0;

#[derive(Clone, Copy, Component, Debug, PartialEq, Eq)]
pub enum RosterSection {
    Active,
    Bench,
}

#[derive(Component)]
pub struct RosterUiRoot;

#[derive(Component)]
pub struct RosterUiList {
    pub section: RosterSection,
}

#[derive(Component)]
pub struct RosterMemberCard {
    pub section: RosterSection,
    pub member_id: String,
}

#[derive(Component)]
pub struct RosterMemberSelectButton {
    pub section: RosterSection,
    pub member_id: String,
}

#[derive(Clone, Component, Debug, PartialEq, Eq)]
pub enum RosterActionButton {
    Promote { bench_member_id: String },
    SwapWithSelectedActive { bench_member_id: String },
    Dismiss { bench_member_id: String },
}

#[derive(Clone, Debug, Event, PartialEq, Eq)]
pub struct RosterMemberSelected {
    pub section: RosterSection,
    pub member_id: String,
}

#[derive(Clone, Debug, Event, PartialEq, Eq)]
pub enum RosterActionRequested {
    Promote {
        bench_member_id: String,
    },
    Swap {
        active_member_id: String,
        bench_member_id: String,
    },
    Dismiss {
        bench_member_id: String,
    },
}

#[derive(Default, Resource)]
pub struct RosterUiSelection {
    pub active_member_id: Option<String>,
    pub bench_member_id: Option<String>,
}

#[derive(Default, Resource)]
pub struct RosterUiVisible(pub bool);

#[derive(Default)]
pub struct RosterUiPlugin;

impl Plugin for RosterUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RosterUiSelection>()
            .init_resource::<RosterUiVisible>()
            .add_event::<RosterMemberSelected>()
            .add_event::<RosterActionRequested>()
            .add_systems(Update, (sync_roster_ui, handle_roster_ui_interactions));
    }
}

pub fn spawn_roster_ui(
    commands: &mut Commands,
    squad: &SquadView,
    selection: &RosterUiSelection,
) -> Entity {
    let font = Handle::<Font>::default();
    commands
        .spawn((
            RosterUiRoot,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    right: Val::Px(14.0),
                    top: Val::Px(14.0),
                    width: Val::Px(PANEL_WIDTH),
                    max_height: Val::Percent(96.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::rgba(0.08, 0.055, 0.04, 0.88)),
                ..default()
            },
        ))
        .with_children(|parent| {
            spawn_text(
                parent,
                font.clone(),
                "Roster",
                22.0,
                Color::rgb(0.98, 0.83, 0.52),
            );
            spawn_roster_section(
                parent,
                font.clone(),
                RosterSection::Active,
                "Active Squad",
                &squad.active,
                squad.max_active,
                selection,
            );
            spawn_roster_section(
                parent,
                font,
                RosterSection::Bench,
                "Bench",
                &squad.bench,
                squad.max_bench,
                selection,
            );
        })
        .id()
}

pub fn sync_roster_ui(
    mut commands: Commands,
    state: Res<VisualGameState>,
    visible: Res<RosterUiVisible>,
    selection: Res<RosterUiSelection>,
    roots: Query<Entity, With<RosterUiRoot>>,
) {
    if !visible.0 {
        for root in &roots {
            commands.entity(root).despawn_recursive();
        }
        return;
    }

    if !state.is_changed() && !selection.is_changed() && !visible.is_changed() && !roots.is_empty()
    {
        return;
    }

    for root in &roots {
        commands.entity(root).despawn_recursive();
    }
    spawn_roster_ui(&mut commands, &state.view.squad, &selection);
}

pub fn handle_roster_ui_interactions(
    mut selection: ResMut<RosterUiSelection>,
    mut selected_events: EventWriter<RosterMemberSelected>,
    mut action_events: EventWriter<RosterActionRequested>,
    select_buttons: Query<
        (&Interaction, &RosterMemberSelectButton),
        (Changed<Interaction>, With<Button>),
    >,
    action_buttons: Query<
        (&Interaction, &RosterActionButton),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, button) in &select_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button.section {
            RosterSection::Active => selection.active_member_id = Some(button.member_id.clone()),
            RosterSection::Bench => selection.bench_member_id = Some(button.member_id.clone()),
        }
        selected_events.send(RosterMemberSelected {
            section: button.section,
            member_id: button.member_id.clone(),
        });
    }

    for (interaction, button) in &action_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button {
            RosterActionButton::Promote { bench_member_id } => {
                action_events.send(RosterActionRequested::Promote {
                    bench_member_id: bench_member_id.clone(),
                });
            }
            RosterActionButton::SwapWithSelectedActive { bench_member_id } => {
                if let Some(active_member_id) = selection.active_member_id.clone() {
                    action_events.send(RosterActionRequested::Swap {
                        active_member_id,
                        bench_member_id: bench_member_id.clone(),
                    });
                }
            }
            RosterActionButton::Dismiss { bench_member_id } => {
                action_events.send(RosterActionRequested::Dismiss {
                    bench_member_id: bench_member_id.clone(),
                });
            }
        }
    }
}

fn spawn_roster_section(
    parent: &mut ChildBuilder,
    font: Handle<Font>,
    section: RosterSection,
    title: &str,
    members: &[SquadMemberView],
    max_members: usize,
    selection: &RosterUiSelection,
) {
    parent
        .spawn((
            RosterUiList { section },
            NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|list| {
            spawn_text(
                list,
                font.clone(),
                &format!("{title} {}/{}", members.len(), max_members),
                15.0,
                Color::rgb(0.82, 0.72, 0.58),
            );
            if members.is_empty() {
                spawn_text(list, font, "Empty", 13.0, Color::rgb(0.55, 0.49, 0.42));
                return;
            }
            for member in members {
                spawn_member_card(list, font.clone(), section, member, selection);
            }
        });
}

fn spawn_member_card(
    parent: &mut ChildBuilder,
    font: Handle<Font>,
    section: RosterSection,
    member: &SquadMemberView,
    selection: &RosterUiSelection,
) {
    let selected = match section {
        RosterSection::Active => selection.active_member_id.as_deref() == Some(member.id.as_str()),
        RosterSection::Bench => selection.bench_member_id.as_deref() == Some(member.id.as_str()),
    };
    let border = if selected {
        Color::rgb(0.95, 0.68, 0.25)
    } else {
        Color::rgba(0.28, 0.21, 0.14, 0.9)
    };

    parent
        .spawn((
            RosterMemberCard {
                section,
                member_id: member.id.clone(),
            },
            NodeBundle {
                style: Style {
                    min_height: Val::Px(CARD_HEIGHT),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(5.0),
                    padding: UiRect::all(Val::Px(7.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: BackgroundColor(card_color(member.status)),
                border_color: BorderColor(border),
                ..default()
            },
        ))
        .with_children(|card| {
            card.spawn((
                RosterMemberSelectButton {
                    section,
                    member_id: member.id.clone(),
                },
                ButtonBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(28.0),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        padding: UiRect::horizontal(Val::Px(6.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(select_button_color(selected)),
                    ..default()
                },
            ))
            .with_children(|button| {
                spawn_text(
                    button,
                    font.clone(),
                    &format!("{}  L{}", member.name, member.level),
                    13.0,
                    Color::rgb(0.96, 0.91, 0.8),
                );
                spawn_text(
                    button,
                    font.clone(),
                    status_label(member.status),
                    11.0,
                    status_color(member.status),
                );
            });

            spawn_text(
                card,
                font.clone(),
                &format!(
                    "{} | {} HP {}/{}",
                    member.role.label(),
                    member.weapon,
                    member.hp.max(0),
                    member.max_hp
                ),
                11.0,
                Color::rgb(0.74, 0.68, 0.58),
            );
            if !member.stats.is_empty() || member.wound_total > 0 {
                let wounds = if member.wound_total > 0 {
                    format!(" | Wounds {}", member.wound_total)
                } else {
                    String::new()
                };
                spawn_text(
                    card,
                    font.clone(),
                    &format!("{}{}", member.stats.join("  "), wounds),
                    10.0,
                    Color::rgb(0.62, 0.58, 0.51),
                );
            }

            if section == RosterSection::Bench {
                spawn_bench_actions(card, font, member);
            }
        });
}

fn spawn_bench_actions(parent: &mut ChildBuilder, font: Handle<Font>, member: &SquadMemberView) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(5.0),
                ..default()
            },
            ..default()
        })
        .with_children(|row| {
            spawn_action_button(
                row,
                font.clone(),
                "Promote",
                RosterActionButton::Promote {
                    bench_member_id: member.id.clone(),
                },
            );
            spawn_action_button(
                row,
                font.clone(),
                "Swap",
                RosterActionButton::SwapWithSelectedActive {
                    bench_member_id: member.id.clone(),
                },
            );
            spawn_action_button(
                row,
                font,
                "Dismiss",
                RosterActionButton::Dismiss {
                    bench_member_id: member.id.clone(),
                },
            );
        });
}

fn spawn_action_button(
    parent: &mut ChildBuilder,
    font: Handle<Font>,
    label: &str,
    action: RosterActionButton,
) {
    parent
        .spawn((
            action,
            ButtonBundle {
                style: Style {
                    height: Val::Px(BUTTON_HEIGHT),
                    padding: UiRect::horizontal(Val::Px(7.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(Color::rgb(0.23, 0.17, 0.12)),
                ..default()
            },
        ))
        .with_children(|button| {
            spawn_text(button, font, label, 10.0, Color::rgb(0.92, 0.84, 0.68));
        });
}

fn spawn_text(
    parent: &mut ChildBuilder,
    font: Handle<Font>,
    value: &str,
    font_size: f32,
    color: Color,
) {
    parent.spawn(TextBundle::from_section(
        value.to_string(),
        TextStyle {
            font,
            font_size,
            color,
        },
    ));
}

fn card_color(status: SquadMemberStatus) -> Color {
    match status {
        SquadMemberStatus::Ready => Color::rgba(0.14, 0.1, 0.07, 0.92),
        SquadMemberStatus::Downed => Color::rgba(0.12, 0.105, 0.09, 0.92),
        SquadMemberStatus::Dead => Color::rgba(0.08, 0.075, 0.07, 0.92),
    }
}

fn select_button_color(selected: bool) -> Color {
    if selected {
        Color::rgb(0.32, 0.22, 0.12)
    } else {
        Color::rgb(0.18, 0.13, 0.09)
    }
}

fn status_label(status: SquadMemberStatus) -> &'static str {
    match status {
        SquadMemberStatus::Ready => "Ready",
        SquadMemberStatus::Downed => "Down",
        SquadMemberStatus::Dead => "Dead",
    }
}

fn status_color(status: SquadMemberStatus) -> Color {
    match status {
        SquadMemberStatus::Ready => Color::rgb(0.62, 0.78, 0.43),
        SquadMemberStatus::Downed => Color::rgb(0.93, 0.68, 0.28),
        SquadMemberStatus::Dead => Color::rgb(0.74, 0.19, 0.13),
    }
}
