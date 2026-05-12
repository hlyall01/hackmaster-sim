use bevy::prelude::*;

use crate::squad_battler::combat::{BattleGrid, GridPos};

use super::assets;

pub const TILE_WORLD_SIZE: f32 = 72.0;
pub const BOARD_CAMERA_MARGIN: f32 = 116.0;

#[derive(Clone, Copy, Resource)]
pub struct BoardGeometry {
    pub grid: BattleGrid,
    pub tile_size: f32,
}

impl BoardGeometry {
    pub fn new(grid: BattleGrid) -> Self {
        Self {
            grid,
            tile_size: TILE_WORLD_SIZE,
        }
    }

    pub fn board_size(self) -> Vec2 {
        Vec2::new(
            self.grid.width as f32 * self.tile_size,
            self.grid.height as f32 * self.tile_size,
        )
    }

    pub fn grid_to_world(self, pos: GridPos, z: f32) -> Vec3 {
        let size = self.board_size();
        Vec3::new(
            -size.x * 0.5 + self.tile_size * 0.5 + pos.x as f32 * self.tile_size,
            size.y * 0.5 - self.tile_size * 0.5 - pos.y as f32 * self.tile_size,
            z,
        )
    }
}

pub fn spawn_board(commands: &mut Commands, geometry: BoardGeometry) {
    let size = geometry.board_size();
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: assets::table_color(),
            custom_size: Some(size + Vec2::splat(50.0)),
            ..default()
        },
        transform: Transform::from_xyz(0.0, 0.0, -4.0),
        ..default()
    });
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: assets::board_edge_color(),
            custom_size: Some(size + Vec2::splat(18.0)),
            ..default()
        },
        transform: Transform::from_xyz(0.0, 0.0, -3.0),
        ..default()
    });

    for y in 0..geometry.grid.height {
        for x in 0..geometry.grid.width {
            let is_light = (x + y) % 2 == 0;
            commands.spawn(SpriteBundle {
                sprite: Sprite {
                    color: if is_light {
                        assets::tile_light_color()
                    } else {
                        assets::tile_dark_color()
                    },
                    custom_size: Some(Vec2::splat(geometry.tile_size - 2.0)),
                    ..default()
                },
                transform: Transform::from_translation(
                    geometry.grid_to_world(GridPos::new(x, y), -2.0),
                ),
                ..default()
            });
        }
    }

    let line_color = assets::grid_line_color();
    for x in 0..=geometry.grid.width {
        let line_x = -size.x * 0.5 + x as f32 * geometry.tile_size;
        commands.spawn(SpriteBundle {
            sprite: Sprite {
                color: line_color,
                custom_size: Some(Vec2::new(2.0, size.y)),
                ..default()
            },
            transform: Transform::from_xyz(line_x, 0.0, -1.0),
            ..default()
        });
    }
    for y in 0..=geometry.grid.height {
        let line_y = size.y * 0.5 - y as f32 * geometry.tile_size;
        commands.spawn(SpriteBundle {
            sprite: Sprite {
                color: line_color,
                custom_size: Some(Vec2::new(size.x, 2.0)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, line_y, -1.0),
            ..default()
        });
    }
}
