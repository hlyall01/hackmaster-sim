use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::squad_battler::combat::{BattleUnitStatus, BattleUnitView, GridPos, SquadCombatView};

use super::app::VisualGameState;
use super::assets;
use super::board::{BoardGeometry, TILE_WORLD_SIZE};

const TOKEN_OUTER_SIZE: f32 = TILE_WORLD_SIZE * 0.72;
const TOKEN_INNER_SIZE: f32 = TILE_WORLD_SIZE * 0.52;
const HEALTH_BAR_WIDTH: f32 = TILE_WORLD_SIZE * 0.62;
const HEALTH_BAR_HEIGHT: f32 = 5.0;

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
) {
    for unit in &fight.combatants {
        spawn_unit(commands, geometry, unit);
    }
}

pub(crate) fn sync_unit_targets(
    mut commands: Commands,
    geometry: Res<BoardGeometry>,
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
            sprite.color = outer_color(unit);
            present_ids.insert(token.id.clone());
        } else {
            commands.entity(entity).despawn_recursive();
        }
    }

    for unit in &fight.combatants {
        if !present_ids.contains(&unit.id) {
            spawn_unit(&mut commands, *geometry, unit);
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

fn spawn_unit(commands: &mut Commands, geometry: BoardGeometry, unit: &BattleUnitView) {
    let pos = geometry.grid_to_world(GridPos::new(unit.x, unit.y), 1.0);
    let hp_width = HEALTH_BAR_WIDTH * health_pct(unit);
    commands
        .spawn((
            UnitToken {
                id: unit.id.clone(),
            },
            TargetWorldPosition(pos),
            SpriteBundle {
                sprite: Sprite {
                    color: outer_color(unit),
                    custom_size: Some(Vec2::splat(TOKEN_OUTER_SIZE)),
                    ..default()
                },
                transform: Transform::from_translation(pos),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: inner_color(unit),
                    custom_size: Some(Vec2::splat(TOKEN_INNER_SIZE)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 0.1),
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

fn outer_color(unit: &BattleUnitView) -> Color {
    if unit.status == BattleUnitStatus::Downed {
        return assets::downed_color();
    }
    match unit.team_id {
        0 => assets::player_outer_color(),
        _ => assets::enemy_outer_color(),
    }
}

fn inner_color(unit: &BattleUnitView) -> Color {
    if unit.status == BattleUnitStatus::Downed {
        return assets::downed_color();
    }
    match unit.team_id {
        0 => assets::player_inner_color(),
        _ => assets::enemy_inner_color(),
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
