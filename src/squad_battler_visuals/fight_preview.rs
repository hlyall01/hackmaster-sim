use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::squad_battler::combat::GridPos;
use crate::squad_battler::roster::{SquadMemberStatus, SquadMemberView};
use crate::squad_battler::state::{EnemyView, SquadBattlerView};

const ROSTER_COLUMN_WIDTH: f32 = 300.0;
const CARD_HEIGHT: f32 = 78.0;
const BUTTON_HEIGHT: f32 = 48.0;
const FORMATION_TILE_SIZE: f32 = 50.0;
const FORMATION_Z: f32 = 30.0;

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

#[derive(Component)]
pub struct FightPreviewFormationRoot;

#[derive(Component)]
pub struct FormationCell {
    pub x: i32,
    pub y: i32,
    pub deployment: bool,
}

#[derive(Component)]
pub struct FormationToken {
    pub member_id: String,
}

#[derive(Default, Resource)]
pub struct FormationDragState {
    member_id: Option<String>,
    origin: Vec3,
}

#[derive(Clone, Debug, Event)]
pub struct FormationMoveRequest {
    pub member_id: String,
    pub x: i32,
    pub y: i32,
}

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
                    ..default()
                },
                background_color: Color::rgba(0.04, 0.03, 0.025, 0.42).into(),
                z_index: ZIndex::Global(50),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(top_panel()).with_children(|panel| {
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
            });

            root.spawn(side_panel(Side::Left)).with_children(|panel| {
                spawn_active_squad_column(panel, &view.squad.active, font.clone());
            });

            root.spawn(side_panel(Side::Right)).with_children(|panel| {
                spawn_enemy_column(panel, &pending.enemies, font.clone());
            });

            root.spawn(action_panel()).with_children(|actions| {
                spawn_back_button(actions, font.clone());
                spawn_start_button(actions, font.clone());
            });
        })
        .id();

    Some(root)
}

pub fn spawn_formation_board(
    commands: &mut Commands,
    view: &SquadBattlerView,
    font: Handle<Font>,
) -> Option<Entity> {
    let pending = view.pending_fight.as_ref()?;
    let width = view.grid.width;
    let height = view.grid.height;
    let deployment_columns = view.formation.deployment_columns;
    let board_size = Vec2::new(
        width as f32 * FORMATION_TILE_SIZE,
        height as f32 * FORMATION_TILE_SIZE,
    );
    let root = commands
        .spawn((FightPreviewFormationRoot, SpatialBundle::default()))
        .id();

    commands.entity(root).with_children(|parent| {
        parent.spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgba(0.13, 0.075, 0.038, 0.94),
                custom_size: Some(board_size + Vec2::splat(34.0)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, -18.0, FORMATION_Z - 4.0),
            ..default()
        });

        for y in 0..height {
            for x in 0..width {
                let pos = formation_world_pos(width, height, GridPos::new(x, y));
                let deployment = x < deployment_columns;
                parent.spawn((
                    FormationCell { x, y, deployment },
                    SpriteBundle {
                        sprite: Sprite {
                            color: formation_cell_color(x, y, deployment, width),
                            custom_size: Some(Vec2::splat(FORMATION_TILE_SIZE - 2.0)),
                            ..default()
                        },
                        transform: Transform::from_translation(pos),
                        ..default()
                    },
                ));
            }
        }

        for member in &view.squad.active {
            let pos = formation_slot_for_member(view, &member.id)
                .unwrap_or_else(|| GridPos::new(1, height / 2));
            spawn_player_token(parent, member, pos, width, height, font.clone());
        }

        let center_y = height / 2;
        let enemy_count = pending.enemies.len();
        for (idx, enemy) in pending.enemies.iter().enumerate() {
            let offset = idx as i32 - (enemy_count as i32 - 1) / 2;
            let pos = GridPos::new(width - 2, center_y + offset).clamp(
                crate::squad_battler::combat::BattleGrid {
                    width,
                    height,
                    tile_size_ft: view.grid.tile_size_ft,
                },
            );
            spawn_enemy_token(parent, enemy, idx, pos, width, height, font.clone());
        }
    });

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

pub fn handle_formation_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut drag: ResMut<FormationDragState>,
    mut tokens: Query<(&FormationToken, &mut Transform)>,
    cells: Query<(&FormationCell, &GlobalTransform)>,
    mut events: EventWriter<FormationMoveRequest>,
) {
    let Some(cursor) = cursor_world_position(&windows, &cameras) else {
        return;
    };

    if buttons.just_pressed(MouseButton::Left) {
        let mut closest = None;
        for (token, transform) in &mut tokens {
            let distance = transform.translation.truncate().distance(cursor);
            if distance <= FORMATION_TILE_SIZE * 0.55 {
                let replace = closest
                    .as_ref()
                    .is_none_or(|(_, _, best): &(String, Vec3, f32)| distance < *best);
                if replace {
                    closest = Some((token.member_id.clone(), transform.translation, distance));
                }
            }
        }
        if let Some((member_id, origin, _)) = closest {
            drag.member_id = Some(member_id);
            drag.origin = origin;
        }
    }

    if let Some(member_id) = drag.member_id.as_ref() {
        for (token, mut transform) in &mut tokens {
            if token.member_id == *member_id {
                transform.translation.x = cursor.x;
                transform.translation.y = cursor.y;
                transform.translation.z = FORMATION_Z + 6.0;
            }
        }
    }

    if !buttons.just_released(MouseButton::Left) {
        return;
    }

    let Some(member_id) = drag.member_id.take() else {
        return;
    };
    let target_cell = cells
        .iter()
        .filter(|(cell, _)| cell.deployment)
        .filter_map(|(cell, transform)| {
            let distance = transform.translation().truncate().distance(cursor);
            (distance <= FORMATION_TILE_SIZE * 0.72).then_some((cell.x, cell.y, distance))
        })
        .min_by(|(_, _, a), (_, _, b)| a.total_cmp(b));

    if let Some((x, y, _)) = target_cell {
        events.send(FormationMoveRequest { member_id, x, y });
    } else {
        for (token, mut transform) in &mut tokens {
            if token.member_id == member_id {
                transform.translation = drag.origin;
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
}

fn top_panel() -> NodeBundle {
    NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left: Val::Px(18.0),
            right: Val::Px(18.0),
            top: Val::Px(16.0),
            height: Val::Px(72.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(28.0),
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(Val::Px(18.0)),
            ..default()
        },
        background_color: Color::rgba(0.08, 0.047, 0.027, 0.90).into(),
        ..default()
    }
}

fn side_panel(side: Side) -> NodeBundle {
    let mut style = Style {
        position_type: PositionType::Absolute,
        top: Val::Px(104.0),
        bottom: Val::Px(92.0),
        width: Val::Px(ROSTER_COLUMN_WIDTH + 24.0),
        flex_direction: FlexDirection::Column,
        padding: UiRect::all(Val::Px(12.0)),
        ..default()
    };
    match side {
        Side::Left => style.left = Val::Px(16.0),
        Side::Right => style.right = Val::Px(16.0),
    }
    NodeBundle {
        style,
        background_color: Color::rgba(0.055, 0.04, 0.032, 0.82).into(),
        ..default()
    }
}

fn action_panel() -> NodeBundle {
    NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            right: Val::Px(18.0),
            bottom: Val::Px(18.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(12.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        background_color: Color::rgba(0.08, 0.047, 0.027, 0.90).into(),
        ..default()
    }
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

fn spawn_player_token(
    parent: &mut ChildBuilder,
    member: &SquadMemberView,
    pos: GridPos,
    width: i32,
    height: i32,
    font: Handle<Font>,
) {
    let world = formation_world_pos(width, height, pos);
    let hp_pct = hp_pct(member.hp, member.max_hp);
    parent
        .spawn((
            FormationToken {
                member_id: member.id.clone(),
            },
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.94, 0.67, 0.22),
                    custom_size: Some(Vec2::splat(FORMATION_TILE_SIZE * 0.74)),
                    ..default()
                },
                transform: Transform::from_translation(world + Vec3::new(0.0, 0.0, 3.0)),
                ..default()
            },
        ))
        .with_children(|token| {
            token.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.11, 0.22, 0.25),
                    custom_size: Some(Vec2::splat(FORMATION_TILE_SIZE * 0.52)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 0.1),
                ..default()
            });
            token.spawn(Text2dBundle {
                text: Text::from_section(
                    initials(&member.name),
                    TextStyle {
                        font: font.clone(),
                        font_size: 17.0,
                        color: Color::rgb(1.0, 0.92, 0.68),
                    },
                ),
                transform: Transform::from_xyz(0.0, 2.0, 0.3),
                ..default()
            });
            token.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.05, 0.03, 0.02),
                    custom_size: Some(Vec2::new(FORMATION_TILE_SIZE * 0.62, 5.0)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, -FORMATION_TILE_SIZE * 0.43, 0.2),
                ..default()
            });
            token.spawn(SpriteBundle {
                sprite: Sprite {
                    color: health_color(hp_pct),
                    custom_size: Some(Vec2::new(FORMATION_TILE_SIZE * 0.62 * hp_pct, 4.0)),
                    ..default()
                },
                transform: Transform::from_xyz(
                    -FORMATION_TILE_SIZE * 0.31 + FORMATION_TILE_SIZE * 0.31 * hp_pct,
                    -FORMATION_TILE_SIZE * 0.43,
                    0.3,
                ),
                ..default()
            });
        });
}

fn spawn_enemy_token(
    parent: &mut ChildBuilder,
    _enemy: &EnemyView,
    idx: usize,
    pos: GridPos,
    width: i32,
    height: i32,
    font: Handle<Font>,
) {
    let world = formation_world_pos(width, height, pos);
    parent
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.76, 0.23, 0.16),
                custom_size: Some(Vec2::splat(FORMATION_TILE_SIZE * 0.72)),
                ..default()
            },
            transform: Transform::from_translation(world + Vec3::new(0.0, 0.0, 3.0)),
            ..default()
        })
        .with_children(|token| {
            token.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.22, 0.055, 0.04),
                    custom_size: Some(Vec2::splat(FORMATION_TILE_SIZE * 0.50)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 0.1),
                ..default()
            });
            token.spawn(Text2dBundle {
                text: Text::from_section(
                    format!("E{}", idx + 1),
                    TextStyle {
                        font,
                        font_size: 16.0,
                        color: Color::rgb(1.0, 0.78, 0.66),
                    },
                ),
                transform: Transform::from_xyz(0.0, 2.0, 0.3),
                ..default()
            });
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

fn formation_world_pos(width: i32, height: i32, pos: GridPos) -> Vec3 {
    let board_width = width as f32 * FORMATION_TILE_SIZE;
    let board_height = height as f32 * FORMATION_TILE_SIZE;
    Vec3::new(
        -board_width * 0.5 + FORMATION_TILE_SIZE * 0.5 + pos.x as f32 * FORMATION_TILE_SIZE,
        board_height * 0.5 - FORMATION_TILE_SIZE * 0.5 - pos.y as f32 * FORMATION_TILE_SIZE - 18.0,
        FORMATION_Z,
    )
}

fn formation_cell_color(x: i32, y: i32, deployment: bool, width: i32) -> Color {
    if deployment {
        if (x + y) % 2 == 0 {
            Color::rgb(0.29, 0.24, 0.13)
        } else {
            Color::rgb(0.23, 0.20, 0.12)
        }
    } else if x >= width - 2 {
        if (x + y) % 2 == 0 {
            Color::rgb(0.28, 0.13, 0.09)
        } else {
            Color::rgb(0.22, 0.10, 0.075)
        }
    } else if (x + y) % 2 == 0 {
        Color::rgb(0.18, 0.12, 0.075)
    } else {
        Color::rgb(0.14, 0.095, 0.065)
    }
}

fn formation_slot_for_member(view: &SquadBattlerView, member_id: &str) -> Option<GridPos> {
    view.formation
        .slots
        .iter()
        .find(|slot| slot.member_id == member_id)
        .map(|slot| GridPos::new(slot.x, slot.y))
}

fn initials(name: &str) -> String {
    let mut initials = name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>();
    if initials.is_empty() {
        initials.push('?');
    }
    initials
}

fn cursor_world_position(
    windows: &Query<&Window, With<PrimaryWindow>>,
    cameras: &Query<(&Camera, &GlobalTransform)>,
) -> Option<Vec2> {
    let window = windows.get_single().ok()?;
    let cursor = window.cursor_position()?;
    let (camera, camera_transform) = cameras.get_single().ok()?;
    camera.viewport_to_world_2d(camera_transform, cursor)
}
