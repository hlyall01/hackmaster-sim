use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::board::{BOARD_CAMERA_MARGIN, BoardGeometry};

pub fn spawn_camera(commands: &mut Commands) {
    commands.spawn(Camera2dBundle::default());
}

pub fn fit_camera_to_board(
    geometry: Res<BoardGeometry>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cameras: Query<&mut OrthographicProjection, With<Camera>>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    let size = geometry.board_size() + Vec2::splat(BOARD_CAMERA_MARGIN * 2.0);
    let required_scale = (size.x / window.width().max(1.0))
        .max(size.y / window.height().max(1.0))
        .max(0.55);

    for mut projection in &mut cameras {
        projection.scale = required_scale;
    }
}
