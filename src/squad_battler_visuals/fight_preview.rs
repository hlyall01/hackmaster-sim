use bevy::prelude::*;

use crate::squad_battler::roster::{SquadMemberStatus, SquadMemberView};
use crate::squad_battler::state::{EnemyView, SquadBattlerView};

const PANEL_WIDTH: f32 = 760.0;
const ROSTER_COLUMN_WIDTH: f32 = 340.0;
const CARD_HEIGHT: f32 = 78.0;
const BUTTON_HEIGHT: f32 = 48.0;

#[derive(Component)]
pub struct FightPreviewRoot;

#[derive(Component)]
pub struct ActiveSquadPreview;

#[derive(Component)]
pub struct PendingEnemiesPreview;

#[derive(Component)]
pub struct StartFightHint;

#[derive(Component)]
pub struct BackHint;

pub fn spawn_fight_preview(
    commands: &mut Commands,
    view: &SquadBattlerView,
    font: Handle<Font>,
) -> Option<Entity> {
    let pending = view.pending_fight.as_ref()?;
    let root = commands
        .spawn((
            FightPreviewRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    padding: UiRect::all(Val::Px(28.0)),
                    ..default()
                },
                background_color: Color::rgba(0.04, 0.03, 0.025, 0.84).into(),
                z_index: ZIndex::Global(50),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(PANEL_WIDTH),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(18.0),
                    padding: UiRect::all(Val::Px(24.0)),
                    ..default()
                },
                background_color: Color::rgb(0.12, 0.075, 0.045).into(),
                ..default()
            })
            .with_children(|panel| {
                panel.spawn(text(
                    format!("{} Fight", pending.tier),
                    font.clone(),
                    32.0,
                    Color::rgb(0.98, 0.84, 0.48),
                ));
                panel.spawn(text(
                    format!(
                        "{} active allies against {} hostile combatants",
                        view.squad.active.len(),
                        pending.enemy_count
                    ),
                    font.clone(),
                    18.0,
                    Color::rgb(0.84, 0.74, 0.62),
                ));

                panel
                    .spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(20.0),
                            ..default()
                        },
                        background_color: Color::NONE.into(),
                        ..default()
                    })
                    .with_children(|columns| {
                        spawn_active_squad_column(columns, &view.squad.active, font.clone());
                        spawn_enemy_column(columns, &pending.enemies, font.clone());
                    });

                panel
                    .spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::FlexEnd,
                            column_gap: Val::Px(12.0),
                            ..default()
                        },
                        background_color: Color::NONE.into(),
                        ..default()
                    })
                    .with_children(|actions| {
                        spawn_back_button(actions, font.clone());
                        spawn_start_button(actions, font.clone());
                    });
            });
        })
        .id();

    Some(root)
}

pub fn despawn_fight_preview(commands: &mut Commands, roots: impl IntoIterator<Item = Entity>) {
    for root in roots {
        commands.entity(root).despawn_recursive();
    }
}

pub fn start_fight_requested(
    interactions: &Query<&Interaction, (Changed<Interaction>, With<StartFightHint>)>,
) -> bool {
    interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
}

pub fn back_requested(
    interactions: &Query<&Interaction, (Changed<Interaction>, With<BackHint>)>,
) -> bool {
    interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
}

fn spawn_active_squad_column(
    parent: &mut ChildBuilder,
    active: &[SquadMemberView],
    font: Handle<Font>,
) {
    parent
        .spawn((ActiveSquadPreview, column_container("Active Squad")))
        .with_children(|column| {
            column.spawn(section_heading("Active Squad", font.clone()));
            if active.is_empty() {
                column.spawn(empty_text("No active allies assigned.", font));
                return;
            }
            for member in active {
                spawn_member_card(column, member, font.clone());
            }
        });
}

fn spawn_enemy_column(parent: &mut ChildBuilder, enemies: &[EnemyView], font: Handle<Font>) {
    parent
        .spawn((PendingEnemiesPreview, column_container("Pending Enemies")))
        .with_children(|column| {
            column.spawn(section_heading("Pending Enemies", font.clone()));
            if enemies.is_empty() {
                column.spawn(empty_text("Enemy squad unknown.", font));
                return;
            }
            for enemy in enemies {
                spawn_enemy_card(column, enemy, font.clone());
            }
        });
}

fn spawn_member_card(parent: &mut ChildBuilder, member: &SquadMemberView, font: Handle<Font>) {
    let hp_pct = hp_pct(member.hp, member.max_hp);
    parent
        .spawn(card(Color::rgb(0.16, 0.12, 0.075)))
        .with_children(|card| {
            card.spawn(text(
                format!("{}  Lv {}", member.name, member.level),
                font.clone(),
                18.0,
                member_name_color(member.status),
            ));
            card.spawn(text(
                format!(
                    "{} {} | HP {}/{}",
                    member.rarity.label(),
                    member.role.label(),
                    member.hp.max(0),
                    member.max_hp
                ),
                font.clone(),
                14.0,
                Color::rgb(0.86, 0.79, 0.66),
            ));
            card.spawn(health_back()).with_children(|bar| {
                bar.spawn(health_fill(hp_pct, health_color(hp_pct)));
            });
        });
}

fn spawn_enemy_card(parent: &mut ChildBuilder, enemy: &EnemyView, font: Handle<Font>) {
    parent
        .spawn(card(Color::rgb(0.18, 0.075, 0.055)))
        .with_children(|card| {
            card.spawn(text(
                format!("{}  Lv {}", enemy.name, enemy.level),
                font.clone(),
                18.0,
                Color::rgb(0.98, 0.54, 0.42),
            ));
            card.spawn(text(
                "Hostile combatant".to_string(),
                font,
                14.0,
                Color::rgb(0.88, 0.72, 0.64),
            ));
        });
}

fn spawn_start_button(parent: &mut ChildBuilder, font: Handle<Font>) {
    parent
        .spawn((
            StartFightHint,
            ButtonBundle {
                style: button_style(184.0),
                background_color: Color::rgb(0.68, 0.2, 0.13).into(),
                ..default()
            },
        ))
        .with_children(|button| {
            button.spawn(text("Start Fight", font, 18.0, Color::rgb(1.0, 0.92, 0.76)));
        });
}

fn spawn_back_button(parent: &mut ChildBuilder, font: Handle<Font>) {
    parent
        .spawn((
            BackHint,
            ButtonBundle {
                style: button_style(124.0),
                background_color: Color::rgb(0.20, 0.16, 0.12).into(),
                ..default()
            },
        ))
        .with_children(|button| {
            button.spawn(text("Back", font, 18.0, Color::rgb(0.9, 0.82, 0.68)));
        });
}

fn column_container(_label: &'static str) -> NodeBundle {
    NodeBundle {
        style: Style {
            width: Val::Px(ROSTER_COLUMN_WIDTH),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            padding: UiRect::all(Val::Px(12.0)),
            ..default()
        },
        background_color: Color::rgba(0.055, 0.04, 0.032, 0.72).into(),
        ..default()
    }
}

fn section_heading(label: &'static str, font: Handle<Font>) -> TextBundle {
    text(label, font, 18.0, Color::rgb(0.98, 0.84, 0.48))
}

fn empty_text(label: &'static str, font: Handle<Font>) -> TextBundle {
    text(label, font, 14.0, Color::rgb(0.76, 0.68, 0.58))
}

fn card(color: Color) -> NodeBundle {
    NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            min_height: Val::Px(CARD_HEIGHT),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(5.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        background_color: color.into(),
        ..default()
    }
}

fn health_back() -> NodeBundle {
    NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Px(6.0),
            ..default()
        },
        background_color: Color::rgb(0.05, 0.03, 0.025).into(),
        ..default()
    }
}

fn health_fill(pct: f32, color: Color) -> NodeBundle {
    NodeBundle {
        style: Style {
            width: Val::Percent((pct * 100.0).max(2.0)),
            height: Val::Percent(100.0),
            ..default()
        },
        background_color: color.into(),
        ..default()
    }
}

fn button_style(width: f32) -> Style {
    Style {
        width: Val::Px(width),
        height: Val::Px(BUTTON_HEIGHT),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

fn text(value: impl Into<String>, font: Handle<Font>, font_size: f32, color: Color) -> TextBundle {
    TextBundle::from_section(
        value,
        TextStyle {
            font,
            font_size,
            color,
        },
    )
}

fn member_name_color(status: SquadMemberStatus) -> Color {
    match status {
        SquadMemberStatus::Ready => Color::rgb(0.96, 0.82, 0.42),
        SquadMemberStatus::Downed => Color::rgb(0.82, 0.44, 0.28),
        SquadMemberStatus::Dead => Color::rgb(0.46, 0.4, 0.36),
    }
}

fn hp_pct(hp: i32, max_hp: i32) -> f32 {
    if max_hp <= 0 {
        return 0.0;
    }
    (hp.max(0) as f32 / max_hp as f32).clamp(0.0, 1.0)
}

fn health_color(pct: f32) -> Color {
    if pct <= 0.33 {
        Color::rgb(0.74, 0.19, 0.13)
    } else if pct <= 0.66 {
        Color::rgb(0.93, 0.68, 0.28)
    } else {
        Color::rgb(0.62, 0.78, 0.43)
    }
}
