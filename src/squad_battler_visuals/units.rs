use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::squad_battler::combat::{BattleUnitStatus, BattleUnitView, GridPos, SquadCombatView};

use super::app::VisualGameState;
use super::assets;
use super::board::{BoardGeometry, TILE_WORLD_SIZE};

const UNIT_SPRITE_SIZE: f32 = TILE_WORLD_SIZE * 0.84;
const UNIT_SHADOW_WIDTH: f32 = TILE_WORLD_SIZE * 0.60;
const UNIT_SHADOW_HEIGHT: f32 = TILE_WORLD_SIZE * 0.20;
const HEALTH_BAR_WIDTH: f32 = TILE_WORLD_SIZE * 0.62;
const HEALTH_BAR_HEIGHT: f32 = 5.0;

const PLAYER_SPRITES: &[&str] = &[
    "sprites/squad/kenney_tiny_dungeon/hero_mage.png",
    "sprites/squad/kenney_tiny_dungeon/hero_scout.png",
    "sprites/squad/kenney_tiny_dungeon/hero_brawler.png",
    "sprites/squad/kenney_tiny_dungeon/hero_knight.png",
    "sprites/squad/kenney_tiny_dungeon/hero_archer.png",
    "sprites/squad/kenney_tiny_dungeon/hero_guard.png",
    "sprites/squad/kenney_tiny_dungeon/hero_captain.png",
];

const ENEMY_SPRITES: &[&str] = &[
    "sprites/squad/kenney_tiny_dungeon/enemy_slime.png",
    "sprites/squad/kenney_tiny_dungeon/enemy_imp.png",
    "sprites/squad/kenney_tiny_dungeon/enemy_bat.png",
    "sprites/squad/kenney_tiny_dungeon/enemy_ghost.png",
];

#[derive(Component)]
pub struct UnitToken {
    pub id: String,
}

#[derive(Component)]
pub struct TargetWorldPosition(pub Vec3);

#[derive(Component)]
pub(crate) struct HealthFill {
    id: String,
}

pub(crate) fn spawn_units(
    commands: &mut Commands,
    geometry: BoardGeometry,
    fight: &SquadCombatView,
    asset_server: &AssetServer,
) {
    for unit in &fight.combatants {
        spawn_unit(commands, geometry, unit, asset_server);
    }
}

pub(crate) fn sync_unit_targets(
    mut commands: Commands,
    geometry: Res<BoardGeometry>,
    asset_server: Res<AssetServer>,
    state: Res<VisualGameState>,
    mut tokens: Query<
        (Entity, &UnitToken, &mut TargetWorldPosition, &mut Sprite),
        Without<HealthFill>,
    >,
    mut health_fills: Query<(&HealthFill, &mut Sprite, &mut Transform)>,
) {
    let Some(fight) = state.view.live_fight.as_ref() else {
        return;
    };
    let units_by_id = fight
        .combatants
        .iter()
        .map(|unit| (unit.id.as_str(), unit))
        .collect::<HashMap<_, _>>();
    let mut present_ids = HashSet::new();

    for (entity, token, mut target, mut sprite) in &mut tokens {
        if let Some(unit) = units_by_id.get(token.id.as_str()) {
            target.0 = geometry.grid_to_world(GridPos::new(unit.x, unit.y), 1.0);
            sprite.color = unit_tint(unit);
            present_ids.insert(token.id.clone());
        } else {
            commands.entity(entity).despawn_recursive();
        }
    }

    for unit in &fight.combatants {
        if !present_ids.contains(&unit.id) {
            spawn_unit(&mut commands, *geometry, unit, &asset_server);
        }
    }

    for (fill, mut sprite, mut transform) in &mut health_fills {
        let Some(unit) = units_by_id.get(fill.id.as_str()) else {
            continue;
        };
        let width = (HEALTH_BAR_WIDTH * health_pct(unit)).max(1.0);
        sprite.color = health_color(unit);
        sprite.custom_size = Some(Vec2::new(width, HEALTH_BAR_HEIGHT));
        transform.translation.x = -HEALTH_BAR_WIDTH * 0.5 + width * 0.5;
    }
}

pub(crate) fn animate_unit_motion(
    time: Res<Time>,
    mut tokens: Query<(&TargetWorldPosition, &mut Transform), With<UnitToken>>,
) {
    let blend = (time.delta_seconds() * 7.5).min(1.0);
    for (target, mut transform) in &mut tokens {
        transform.translation = transform.translation.lerp(target.0, blend);
        if transform.translation.distance(target.0) < 0.15 {
            transform.translation = target.0;
        }
    }
}

fn spawn_unit(
    commands: &mut Commands,
    geometry: BoardGeometry,
    unit: &BattleUnitView,
    asset_server: &AssetServer,
) {
    let pos = geometry.grid_to_world(GridPos::new(unit.x, unit.y), 1.0);
    let hp_width = HEALTH_BAR_WIDTH * health_pct(unit);
    commands
        .spawn((
            UnitToken {
                id: unit.id.clone(),
            },
            TargetWorldPosition(pos),
            SpriteBundle {
                texture: asset_server.load(unit_sprite_path(unit)),
                sprite: Sprite {
                    color: unit_tint(unit),
                    custom_size: Some(Vec2::splat(UNIT_SPRITE_SIZE)),
                    ..default()
                },
                transform: Transform::from_translation(pos),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: shadow_color(unit),
                    custom_size: Some(Vec2::new(UNIT_SHADOW_WIDTH, UNIT_SHADOW_HEIGHT)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, -TILE_WORLD_SIZE * 0.26, -0.1),
                ..default()
            });
            parent.spawn(Text2dBundle {
                text: Text::from_section(
                    unit_label(unit),
                    TextStyle {
                        font: Handle::<Font>::default(),
                        font_size: 15.0,
                        color: label_color(unit),
                    },
                )
                .with_justify(JustifyText::Center),
                transform: Transform::from_xyz(0.0, TILE_WORLD_SIZE * 0.32, 0.35),
                ..default()
            });
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: assets::health_back_color(),
                    custom_size: Some(Vec2::new(HEALTH_BAR_WIDTH, HEALTH_BAR_HEIGHT + 2.0)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, -TILE_WORLD_SIZE * 0.42, 0.2),
                ..default()
            });
            parent.spawn((
                HealthFill {
                    id: unit.id.clone(),
                },
                SpriteBundle {
                    sprite: Sprite {
                        color: health_color(unit),
                        custom_size: Some(Vec2::new(hp_width.max(1.0), HEALTH_BAR_HEIGHT)),
                        ..default()
                    },
                    transform: Transform::from_xyz(
                        -HEALTH_BAR_WIDTH * 0.5 + hp_width * 0.5,
                        -TILE_WORLD_SIZE * 0.42,
                        0.3,
                    ),
                    ..default()
                },
            ));
        });
}

fn unit_sprite_path(unit: &BattleUnitView) -> &'static str {
    let set = if unit.team_id == 0 {
        PLAYER_SPRITES
    } else {
        ENEMY_SPRITES
    };
    set[stable_index(&unit.id, set.len())]
}

fn stable_index(value: &str, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let hash = value.bytes().fold(0_u32, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(byte as u32)
    });
    hash as usize % len
}

fn unit_tint(unit: &BattleUnitView) -> Color {
    if unit.status == BattleUnitStatus::Downed {
        Color::rgba(0.54, 0.50, 0.46, 0.86)
    } else {
        Color::WHITE
    }
}

fn shadow_color(unit: &BattleUnitView) -> Color {
    if unit.status == BattleUnitStatus::Downed {
        Color::rgba(0.14, 0.11, 0.10, 0.52)
    } else if unit.team_id == 0 {
        Color::rgba(0.94, 0.67, 0.22, 0.34)
    } else {
        Color::rgba(0.76, 0.23, 0.16, 0.34)
    }
}

fn health_pct(unit: &BattleUnitView) -> f32 {
    if unit.max_hp <= 0 {
        return 0.0;
    }
    (unit.hp.max(0) as f32 / unit.max_hp as f32).clamp(0.0, 1.0)
}

fn health_color(unit: &BattleUnitView) -> Color {
    let pct = health_pct(unit);
    if pct <= 0.33 {
        assets::health_low_color()
    } else if pct <= 0.66 {
        assets::health_mid_color()
    } else {
        assets::health_high_color()
    }
}

fn label_color(unit: &BattleUnitView) -> Color {
    match unit.team_id {
        0 => Color::rgb(1.0, 0.92, 0.68),
        _ => Color::rgb(1.0, 0.78, 0.66),
    }
}

fn unit_label(unit: &BattleUnitView) -> String {
    if unit.team_id == 0 {
        initials(&unit.name)
    } else {
        enemy_label(&unit.id).unwrap_or_else(|| initials(&unit.name))
    }
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

fn enemy_label(id: &str) -> Option<String> {
    id.rsplit_once('-')
        .and_then(|(_, index)| index.parse::<usize>().ok())
        .map(|index| format!("E{}", index + 1))
}
