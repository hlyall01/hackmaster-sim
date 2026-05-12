use bevy::prelude::*;

use crate::squad_battler::combat::{
    BattleUnitStatus, BattleUnitView, InitiativeView, SquadCombatView,
};
use crate::squad_battler::state::{FightCommand, SquadBattlerView};

use super::app::VisualGameState;

const PANEL_BG: Color = Color::rgba(0.055, 0.041, 0.03, 0.86);
const PANEL_BORDER: Color = Color::rgba(0.93, 0.68, 0.28, 0.58);
const PANEL_MUTED: Color = Color::rgba(0.17, 0.12, 0.08, 0.90);
const TEXT_MAIN: Color = Color::rgb(0.96, 0.90, 0.78);
const TEXT_MUTED: Color = Color::rgb(0.72, 0.64, 0.52);
const PLAYER_ACCENT: Color = Color::rgb(0.96, 0.75, 0.31);
const ENEMY_ACCENT: Color = Color::rgb(0.78, 0.24, 0.16);
const READY_ACCENT: Color = Color::rgb(0.62, 0.78, 0.43);
const DOWNED_ACCENT: Color = Color::rgb(0.22, 0.17, 0.13);

#[derive(Component)]
pub struct SquadBattlerHudRoot;

#[derive(Component)]
pub struct SquadBattlerHudButton {
    pub action: HudAction,
}

#[derive(Clone, Copy, Debug)]
pub enum HudAction {
    Fight(FightCommand),
}

#[derive(Clone, Debug)]
pub struct HudControl {
    pub label: &'static str,
    pub action: HudAction,
    pub enabled: bool,
}

pub fn spawn_hud(commands: &mut Commands, view: &SquadBattlerView) {
    commands
        .spawn((
            SquadBattlerHudRoot,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::all(Val::Px(16.0)),
                    ..default()
                },
                background_color: Color::NONE.into(),
                z_index: ZIndex::Global(20),
                ..default()
            },
        ))
        .with_children(|root| {
            spawn_top_bar(root, view);
            spawn_middle_panels(root, view.live_fight.as_ref());
            spawn_bottom_bar(root, view.live_fight.as_ref());
        });
}

pub fn sync_hud(
    mut commands: Commands,
    state: Res<VisualGameState>,
    roots: Query<Entity, With<SquadBattlerHudRoot>>,
    mut last_signature: Local<Option<String>>,
) {
    let signature = hud_signature(&state.view);
    if last_signature.as_ref() == Some(&signature) && !roots.is_empty() {
        return;
    }

    for entity in &roots {
        commands.entity(entity).despawn_recursive();
    }
    spawn_hud(&mut commands, &state.view);
    *last_signature = Some(signature);
}

pub fn controls_for_view(view: &SquadBattlerView) -> Vec<HudControl> {
    controls_for_fight(view.live_fight.as_ref())
}

fn controls_for_fight(fight: Option<&SquadCombatView>) -> Vec<HudControl> {
    let has_active_fight = fight.is_some_and(|fight| !fight.done);
    let running = fight.is_some_and(|fight| fight.running);

    vec![
        HudControl {
            label: if running { "Pause" } else { "Play" },
            action: HudAction::Fight(if running {
                FightCommand::Pause
            } else {
                FightCommand::Play
            }),
            enabled: has_active_fight,
        },
        HudControl {
            label: "Step",
            action: HudAction::Fight(FightCommand::Step),
            enabled: has_active_fight && !running,
        },
        HudControl {
            label: "Next",
            action: HudAction::Fight(FightCommand::SkipToNextInitiative),
            enabled: has_active_fight,
        },
        HudControl {
            label: "Finish",
            action: HudAction::Fight(FightCommand::Finish),
            enabled: has_active_fight,
        },
    ]
}

fn spawn_top_bar(parent: &mut ChildBuilder, view: &SquadBattlerView) {
    parent
        .spawn(panel_node(
            Val::Percent(100.0),
            Val::Px(58.0),
            FlexDirection::Row,
        ))
        .with_children(|bar| {
            bar.spawn(text_node(
                format!("Depth {}", view.depth),
                22.0,
                PLAYER_ACCENT,
            ));
            bar.spawn(text_node(
                format!("Gold {}", view.gold),
                22.0,
                PLAYER_ACCENT,
            ));
            bar.spawn(text_node(view.phase.clone(), 20.0, TEXT_MAIN));
            let timer = view
                .live_fight
                .as_ref()
                .map(|fight| format!("{}/{}s", fight.elapsed_seconds, fight.max_seconds))
                .unwrap_or_else(|| "--/--s".to_string());
            bar.spawn(text_node(timer, 20.0, TEXT_MAIN));
            let state = view
                .live_fight
                .as_ref()
                .map(fight_state_label)
                .unwrap_or("Route");
            bar.spawn(text_node(state.to_string(), 18.0, TEXT_MUTED));
        });
}

fn spawn_middle_panels(parent: &mut ChildBuilder, fight: Option<&SquadCombatView>) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                margin: UiRect::vertical(Val::Px(14.0)),
                ..default()
            },
            background_color: Color::NONE.into(),
            ..default()
        })
        .with_children(|row| {
            spawn_status_panel(row, "Squad", PLAYER_ACCENT, side_units(fight, 0));
            spawn_status_panel(row, "Enemies", ENEMY_ACCENT, side_units(fight, 1));
        });
}

fn spawn_status_panel(
    parent: &mut ChildBuilder,
    title: &str,
    accent: Color,
    units: Vec<&BattleUnitView>,
) {
    parent
        .spawn(panel_node(Val::Px(300.0), Val::Auto, FlexDirection::Column))
        .with_children(|panel| {
            panel.spawn(text_node(title.to_string(), 22.0, accent));
            if units.is_empty() {
                panel.spawn(text_node("No combatants".to_string(), 15.0, TEXT_MUTED));
                return;
            }
            for unit in units {
                spawn_unit_status(panel, unit, accent);
            }
        });
}

fn spawn_unit_status(parent: &mut ChildBuilder, unit: &BattleUnitView, accent: Color) {
    let pct = health_pct(unit);
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                padding: UiRect::all(Val::Px(8.0)),
                margin: UiRect::top(Val::Px(8.0)),
                ..default()
            },
            background_color: PANEL_MUTED.into(),
            ..default()
        })
        .with_children(|card| {
            card.spawn(text_node(
                format!("{}  {}", unit.name, status_label(unit)),
                15.0,
                TEXT_MAIN,
            ));
            card.spawn(text_node(
                format!(
                    "{}/{} HP  {}  {}",
                    unit.hp.max(0),
                    unit.max_hp,
                    unit.weapon,
                    unit.intent
                ),
                13.0,
                TEXT_MUTED,
            ));
            card.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Px(6.0),
                    ..default()
                },
                background_color: DOWNED_ACCENT.into(),
                ..default()
            })
            .with_children(|bar| {
                bar.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(pct * 100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    background_color: health_color(unit, accent).into(),
                    ..default()
                });
            });
        });
}

fn spawn_bottom_bar(parent: &mut ChildBuilder, fight: Option<&SquadCombatView>) {
    parent
        .spawn(panel_node(
            Val::Percent(100.0),
            Val::Px(104.0),
            FlexDirection::Row,
        ))
        .with_children(|bar| {
            bar.spawn(NodeBundle {
                style: Style {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    overflow: Overflow::clip_x(),
                    ..default()
                },
                background_color: Color::NONE.into(),
                ..default()
            })
            .with_children(|strip| {
                if let Some(fight) = fight {
                    spawn_initiative_strip(strip, fight);
                } else {
                    strip.spawn(text_node("No active fight".to_string(), 16.0, TEXT_MUTED));
                }
            });
            spawn_controls(bar, fight);
        });
}

fn spawn_initiative_strip(parent: &mut ChildBuilder, fight: &SquadCombatView) {
    for item in fight.initiative.iter().take(10) {
        let accent = if item.ready {
            READY_ACCENT
        } else if item.team_id == 0 {
            PLAYER_ACCENT
        } else {
            ENEMY_ACCENT
        };
        spawn_initiative_chip(parent, item, accent);
    }
}

fn spawn_initiative_chip(parent: &mut ChildBuilder, item: &InitiativeView, accent: Color) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Px(112.0),
                height: Val::Px(70.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(7.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            background_color: PANEL_MUTED.into(),
            border_color: accent.into(),
            ..default()
        })
        .with_children(|chip| {
            chip.spawn(text_node(item.name.clone(), 14.0, TEXT_MAIN));
            let timing = if item.ready {
                "Ready".to_string()
            } else {
                format!("{:.1}s", item.next_action_in_seconds.max(0.0))
            };
            chip.spawn(text_node(timing, 13.0, accent));
        });
}

fn spawn_controls(parent: &mut ChildBuilder, fight: Option<&SquadCombatView>) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Px(318.0),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                row_gap: Val::Px(8.0),
                ..default()
            },
            background_color: Color::NONE.into(),
            ..default()
        })
        .with_children(|controls| {
            for control in controls_for_fight(fight) {
                spawn_control_button(controls, control);
            }
        });
}

fn spawn_control_button(parent: &mut ChildBuilder, control: HudControl) {
    let bg = if control.enabled {
        Color::rgba(0.39, 0.22, 0.11, 0.94)
    } else {
        Color::rgba(0.10, 0.075, 0.055, 0.86)
    };
    let fg = if control.enabled {
        TEXT_MAIN
    } else {
        TEXT_MUTED
    };
    parent
        .spawn((
            SquadBattlerHudButton {
                action: control.action,
            },
            ButtonBundle {
                style: Style {
                    width: Val::Px(72.0),
                    height: Val::Px(32.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: bg.into(),
                border_color: PANEL_BORDER.into(),
                ..default()
            },
        ))
        .with_children(|button| {
            button.spawn(text_node(control.label.to_string(), 13.0, fg));
        });
}

fn panel_node(width: Val, height: Val, direction: FlexDirection) -> NodeBundle {
    NodeBundle {
        style: Style {
            width,
            height,
            flex_direction: direction,
            align_items: AlignItems::Center,
            column_gap: Val::Px(18.0),
            row_gap: Val::Px(8.0),
            padding: UiRect::all(Val::Px(12.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        background_color: PANEL_BG.into(),
        border_color: PANEL_BORDER.into(),
        ..default()
    }
}

fn text_node(value: String, size: f32, color: Color) -> TextBundle {
    TextBundle::from_section(
        value,
        TextStyle {
            font_size: size,
            color,
            ..default()
        },
    )
}

fn side_units(fight: Option<&SquadCombatView>, team_id: u8) -> Vec<&BattleUnitView> {
    fight
        .map(|fight| {
            fight
                .combatants
                .iter()
                .filter(|unit| unit.team_id == team_id)
                .collect()
        })
        .unwrap_or_default()
}

fn fight_state_label(fight: &SquadCombatView) -> &'static str {
    if fight.done {
        "Done"
    } else if fight.running {
        "Running"
    } else {
        "Paused"
    }
}

fn status_label(unit: &BattleUnitView) -> &'static str {
    match unit.status {
        BattleUnitStatus::Alive => "Alive",
        BattleUnitStatus::Downed => "Down",
    }
}

fn health_pct(unit: &BattleUnitView) -> f32 {
    if unit.max_hp <= 0 {
        0.0
    } else {
        (unit.hp.max(0) as f32 / unit.max_hp as f32).clamp(0.0, 1.0)
    }
}

fn health_color(unit: &BattleUnitView, accent: Color) -> Color {
    if unit.status == BattleUnitStatus::Downed {
        DOWNED_ACCENT
    } else if health_pct(unit) <= 0.33 {
        ENEMY_ACCENT
    } else {
        accent
    }
}

fn hud_signature(view: &SquadBattlerView) -> String {
    let fight_signature = view
        .live_fight
        .as_ref()
        .map(|fight| {
            let units = fight
                .combatants
                .iter()
                .map(|unit| {
                    format!(
                        "{}:{}:{}:{}:{}",
                        unit.id, unit.hp, unit.max_hp, unit.status as u8, unit.intent
                    )
                })
                .collect::<Vec<_>>()
                .join("|");
            let initiative = fight
                .initiative
                .iter()
                .map(|item| {
                    format!(
                        "{}:{:.1}:{}",
                        item.combatant_id, item.next_action_in_seconds, item.ready
                    )
                })
                .collect::<Vec<_>>()
                .join("|");
            format!(
                "{}:{}:{}:{:?}:{}:{}",
                fight.elapsed_seconds,
                fight.max_seconds,
                fight.running,
                fight.winner_team,
                units,
                initiative
            )
        })
        .unwrap_or_default();

    format!(
        "{}:{}:{}:{}:{}",
        view.depth,
        view.gold,
        view.phase,
        view.log.len(),
        fight_signature
    )
}
