use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::autobattler::render::{
    body_visual_for_race_id, weapon_visual_for_group, RenderAssets, RenderConfig, WEAPON_GROUPS,
};
use crate::autobattler::screenshot::ScreenshotState;
use crate::autobattler::state::{AppScreen, AutobattlerState, SpriteReviewStage, SpriteReviewState};

#[derive(Component)]
pub struct ReviewBody;

#[derive(Component)]
pub struct ReviewWeapon;

fn review_grid_layout(
    window: &Window,
    count: usize,
    columns: usize,
    margin: Vec2,
) -> (Vec<Vec2>, Vec2) {
    if count == 0 {
        return (Vec::new(), Vec2::ZERO);
    }
    let columns = columns.max(1).min(count);
    let rows = (count + columns - 1) / columns;
    let available_width = (window.width() - margin.x * 2.0).max(1.0);
    let available_height = (window.height() - margin.y * 2.0).max(1.0);
    let cell_w = available_width / columns as f32;
    let cell_h = available_height / rows as f32;
    let start_x = -window.width() * 0.5 + margin.x + cell_w * 0.5;
    let start_y = window.height() * 0.5 - margin.y - cell_h * 0.5;
    let mut positions = Vec::with_capacity(count);
    for idx in 0..count {
        let col = idx % columns;
        let row = idx / columns;
        positions.push(Vec2::new(
            start_x + col as f32 * cell_w,
            start_y - row as f32 * cell_h,
        ));
    }
    (positions, Vec2::new(cell_w, cell_h))
}

pub fn sprite_review_system(
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    assets: Res<RenderAssets>,
    render_config: Res<RenderConfig>,
    state: Res<AutobattlerState>,
    mut screenshots: ResMut<ScreenshotState>,
    review: Option<ResMut<SpriteReviewState>>,
    review_bodies: Query<Entity, With<ReviewBody>>,
    review_weapons: Query<Entity, (With<ReviewWeapon>, Without<Parent>)>,
) {
    let Some(mut review) = review else {
        return;
    };
    if !matches!(state.app.screen, AppScreen::SpriteReview) {
        return;
    }
    let Ok(window) = windows.get_single() else {
        return;
    };

    if review.needs_refresh {
        review.needs_refresh = false;
        review.frames_since_refresh = 0;
        review.awaiting_capture = false;

        for entity in review_bodies.iter() {
            commands.entity(entity).despawn_recursive();
        }
        for entity in review_weapons.iter() {
            commands.entity(entity).despawn_recursive();
        }

        match review.stage {
            SpriteReviewStage::Weapons => {
                let Some(race_id) = review.current_race() else {
                    return;
                };
                let (positions, cell_size) = review_grid_layout(
                    window,
                    WEAPON_GROUPS.len(),
                    7,
                    Vec2::new(60.0, 80.0),
                );
                let ground_offset = cell_size.y * 0.25;
                let body_visual = body_visual_for_race_id(race_id, &assets, &render_config, false);
                let base_weapon_y =
                    body_visual.size.y * render_config.weapon_anchor_y - body_visual.ground_offset;
                let weapon_z = render_config.weapon_z - render_config.person_z;
                for (idx, group) in WEAPON_GROUPS.iter().enumerate() {
                    let Some(pos) = positions.get(idx) else {
                        continue;
                    };
                    let ground_y = pos.y - ground_offset;
                    let body_entity = commands
                        .spawn((
                            SpriteBundle {
                                sprite: Sprite {
                                    color: body_visual.color,
                                    custom_size: Some(body_visual.size),
                                    anchor: body_visual.anchor,
                                    ..Default::default()
                                },
                                texture: body_visual.texture.clone(),
                                transform: Transform::from_translation(Vec3::new(
                                    pos.x,
                                    ground_y + body_visual.ground_offset,
                                    render_config.person_z,
                                )),
                                ..Default::default()
                            },
                            ReviewBody,
                        ))
                        .id();
                    let weapon_visual = weapon_visual_for_group(*group, &assets, &render_config);
                    if !weapon_visual.show {
                        continue;
                    }
                    let weapon_texture = weapon_visual.texture.clone();
                    let weapon_entity = commands
                        .spawn((
                            SpriteBundle {
                                sprite: Sprite {
                                    color: weapon_visual.color,
                                    custom_size: Some(weapon_visual.size),
                                    anchor: weapon_visual.anchor,
                                    ..Default::default()
                                },
                                texture: weapon_texture,
                                transform: Transform::from_translation(Vec3::new(
                                    weapon_visual.offset.x,
                                    base_weapon_y + weapon_visual.offset.y,
                                    weapon_z,
                                ))
                                .with_rotation(Quat::from_rotation_z(
                                    weapon_visual.rotation_deg.to_radians(),
                                )),
                                ..Default::default()
                            },
                            ReviewWeapon,
                        ))
                        .id();
                    commands.entity(body_entity).add_child(weapon_entity);
                }
            }
            SpriteReviewStage::Pained => {
                let count = review.races.len();
                let (positions, cell_size) =
                    review_grid_layout(window, count, 6, Vec2::new(60.0, 80.0));
                let ground_offset = cell_size.y * 0.25;
                for (idx, race_id) in review.races.iter().enumerate() {
                    let Some(pos) = positions.get(idx) else {
                        continue;
                    };
                    let visual = body_visual_for_race_id(race_id, &assets, &render_config, true);
                    let ground_y = pos.y - ground_offset;
                    commands.spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                color: visual.color,
                                custom_size: Some(visual.size),
                                anchor: visual.anchor,
                                ..Default::default()
                            },
                            texture: visual.texture,
                            transform: Transform::from_translation(Vec3::new(
                                pos.x,
                                ground_y + visual.ground_offset,
                                render_config.person_z,
                            )),
                            ..Default::default()
                        },
                        ReviewBody,
                    ));
                }
            }
        }
    } else {
        review.frames_since_refresh = review.frames_since_refresh.saturating_add(1);
    }

    if review.awaiting_capture || review.frames_since_refresh < 4 {
        return;
    }
    let path = match review.stage {
        SpriteReviewStage::Weapons => review
            .current_race()
            .map(|race_id| format!("screenshots/sprite_review_{race_id}_weapons.png"))
            .unwrap_or_else(|| "screenshots/sprite_review_unknown.png".to_string()),
        SpriteReviewStage::Pained => "screenshots/sprite_review_pained.png".to_string(),
    };
    screenshots.requested_path = Some(path);
    screenshots.requested = true;
    review.awaiting_capture = true;
}
