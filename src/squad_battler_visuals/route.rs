use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::collections::HashSet;

use crate::squad_battler::encounters::{SquadNodeKind, SquadRouteNode};
use crate::squad_battler::state::SquadBattlerView;

use super::app::VisualGameState;

const ROUTE_NODE_SIZE: f32 = 46.0;
const ROUTE_NODE_INNER_SIZE: f32 = 30.0;
const ROUTE_NODE_SELECTED_SIZE: f32 = 58.0;
const ROUTE_PICK_RADIUS: f32 = 31.0;
const ROUTE_FLOOR_SPACING: f32 = 104.0;
const ROUTE_LANE_SPACING: f32 = 74.0;
const ROUTE_Z: f32 = 8.0;
const ROUTE_ORIGIN: Vec2 = Vec2::new(-460.0, 185.0);

#[derive(Clone, Copy, Debug, Default, Resource)]
pub struct SelectedRouteNode {
    pub node_id: Option<usize>,
    pub requested_node_id: Option<usize>,
}

impl SelectedRouteNode {
    pub fn take_requested(&mut self) -> Option<usize> {
        self.requested_node_id.take()
    }
}

#[derive(Component)]
pub struct RouteNodeEntity {
    pub node_id: usize,
    pub available: bool,
}

#[derive(Component)]
pub struct RouteNodeSelection {
    pub selected: bool,
}

#[derive(Component)]
struct RouteMapEntity;

#[derive(Component)]
struct RouteNodeFill {
    node_id: usize,
    base_color: Color,
}

#[derive(Component)]
struct RouteNodeSelectedRing {
    node_id: usize,
}

#[derive(Default, Resource)]
pub struct RouteMapSignature(Vec<RouteNodeSignature>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RouteNodeSignature {
    id: usize,
    floor: u32,
    lane: u32,
    kind: SquadNodeKind,
    completed: bool,
    available: bool,
}

pub struct RouteMapPlugin;

impl Plugin for RouteMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedRouteNode>()
            .init_resource::<RouteMapSignature>()
            .add_systems(
                Update,
                (
                    sync_route_map,
                    clear_unavailable_route_selection,
                    select_route_node_with_mouse,
                    select_route_node_with_keyboard,
                    sync_route_node_selection,
                )
                    .chain(),
            );
    }
}

pub fn spawn_route_map(commands: &mut Commands, view: &SquadBattlerView) {
    let available = view.available_nodes.iter().copied().collect::<HashSet<_>>();
    spawn_route_map_nodes(commands, &view.route, &available);
}

fn sync_route_map(
    mut commands: Commands,
    state: Res<VisualGameState>,
    mut signature: ResMut<RouteMapSignature>,
    route_entities: Query<Entity, (With<RouteMapEntity>, Without<Parent>)>,
) {
    let available = state
        .view
        .available_nodes
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let next_signature = build_signature(&state.view.route, &available);
    if signature.0 == next_signature {
        return;
    }

    for entity in &route_entities {
        commands.entity(entity).despawn_recursive();
    }
    spawn_route_map_nodes(&mut commands, &state.view.route, &available);
    signature.0 = next_signature;
}

fn clear_unavailable_route_selection(
    state: Res<VisualGameState>,
    mut selected: ResMut<SelectedRouteNode>,
) {
    if selected
        .node_id
        .is_some_and(|id| !state.view.available_nodes.contains(&id))
    {
        selected.node_id = state.view.available_nodes.first().copied();
    }
    if selected
        .requested_node_id
        .is_some_and(|id| !state.view.available_nodes.contains(&id))
    {
        selected.requested_node_id = None;
    }
}

fn select_route_node_with_mouse(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    nodes: Query<(&RouteNodeEntity, &GlobalTransform)>,
    mut selected: ResMut<SelectedRouteNode>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor_world) = cursor_world_position(&windows, &cameras) else {
        return;
    };
    let Some(node_id) = nodes
        .iter()
        .filter(|(node, _)| node.available)
        .filter_map(|(node, transform)| {
            let pos = transform.translation().truncate();
            let distance = pos.distance(cursor_world);
            (distance <= ROUTE_PICK_RADIUS).then_some((node.node_id, distance))
        })
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(node_id, _)| node_id)
    else {
        return;
    };

    selected.node_id = Some(node_id);
    selected.requested_node_id = Some(node_id);
}

fn select_route_node_with_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    nodes: Query<(&RouteNodeEntity, &Transform)>,
    mut selected: ResMut<SelectedRouteNode>,
) {
    let mut available = nodes
        .iter()
        .filter(|(node, _)| node.available)
        .map(|(node, transform)| (node.node_id, transform.translation.truncate()))
        .collect::<Vec<_>>();
    if available.is_empty() {
        selected.node_id = None;
        selected.requested_node_id = None;
        return;
    }
    available.sort_by_key(|(node_id, _)| *node_id);

    if selected.node_id.is_none_or(|id| {
        !available
            .iter()
            .any(|(available_id, _)| *available_id == id)
    }) {
        selected.node_id = Some(available[0].0);
    }

    if keys.just_pressed(KeyCode::Tab) {
        selected.node_id = cycle_selection(&available, selected.node_id, 1);
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        selected.node_id = nearest_directional_selection(&available, selected.node_id, Vec2::X)
            .or_else(|| cycle_selection(&available, selected.node_id, 1));
    }
    if keys.just_pressed(KeyCode::ArrowLeft) {
        selected.node_id = nearest_directional_selection(&available, selected.node_id, -Vec2::X)
            .or_else(|| cycle_selection(&available, selected.node_id, -1));
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        selected.node_id = nearest_directional_selection(&available, selected.node_id, Vec2::Y)
            .or_else(|| cycle_selection(&available, selected.node_id, -1));
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        selected.node_id = nearest_directional_selection(&available, selected.node_id, -Vec2::Y)
            .or_else(|| cycle_selection(&available, selected.node_id, 1));
    }
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        selected.requested_node_id = selected.node_id;
    }
}

fn sync_route_node_selection(
    selected: Res<SelectedRouteNode>,
    mut nodes: Query<(&RouteNodeEntity, &mut RouteNodeSelection, &mut Sprite)>,
    mut fills: Query<(&RouteNodeFill, &mut Sprite)>,
    mut rings: Query<(&RouteNodeSelectedRing, &mut Visibility)>,
) {
    for (node, mut selection, mut sprite) in &mut nodes {
        selection.selected = selected.node_id == Some(node.node_id);
        sprite.color = node_outer_color(node.available, selection.selected);
    }
    for (fill, mut sprite) in &mut fills {
        sprite.color = if selected.node_id == Some(fill.node_id) {
            Color::rgb(1.0, 0.92, 0.58)
        } else {
            fill.base_color
        };
    }
    for (ring, mut visibility) in &mut rings {
        *visibility = if selected.node_id == Some(ring.node_id) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn spawn_route_map_nodes(
    commands: &mut Commands,
    route: &[SquadRouteNode],
    available: &HashSet<usize>,
) {
    if route.is_empty() {
        return;
    }

    let max_lane = route.iter().map(|node| node.lane).max().unwrap_or(0);
    let lane_center = max_lane as f32 * 0.5;

    for node in route {
        let position = route_node_position(node, lane_center);
        let available = available.contains(&node.id);
        let completed = node.completed;

        commands
            .spawn((
                RouteMapEntity,
                RouteNodeEntity {
                    node_id: node.id,
                    available,
                },
                RouteNodeSelection { selected: false },
                SpriteBundle {
                    sprite: Sprite {
                        color: node_outer_color(available, false),
                        custom_size: Some(Vec2::splat(ROUTE_NODE_SIZE)),
                        ..default()
                    },
                    transform: Transform::from_translation(position),
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    RouteMapEntity,
                    RouteNodeSelectedRing { node_id: node.id },
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::rgba(1.0, 0.84, 0.28, 0.45),
                            custom_size: Some(Vec2::splat(ROUTE_NODE_SELECTED_SIZE)),
                            ..default()
                        },
                        visibility: Visibility::Hidden,
                        transform: Transform::from_xyz(0.0, 0.0, -0.1),
                        ..default()
                    },
                ));
                parent.spawn((
                    RouteMapEntity,
                    RouteNodeFill {
                        node_id: node.id,
                        base_color: node_inner_color(node.kind, available, completed),
                    },
                    SpriteBundle {
                        sprite: Sprite {
                            color: node_inner_color(node.kind, available, completed),
                            custom_size: Some(Vec2::splat(ROUTE_NODE_INNER_SIZE)),
                            ..default()
                        },
                        transform: Transform::from_xyz(0.0, 0.0, 0.1),
                        ..default()
                    },
                ));
            });
    }
}

fn build_signature(
    route: &[SquadRouteNode],
    available: &HashSet<usize>,
) -> Vec<RouteNodeSignature> {
    route
        .iter()
        .map(|node| RouteNodeSignature {
            id: node.id,
            floor: node.floor,
            lane: node.lane,
            kind: node.kind,
            completed: node.completed,
            available: available.contains(&node.id),
        })
        .collect()
}

fn route_node_position(node: &SquadRouteNode, lane_center: f32) -> Vec3 {
    Vec3::new(
        ROUTE_ORIGIN.x + node.floor as f32 * ROUTE_FLOOR_SPACING,
        ROUTE_ORIGIN.y - (node.lane as f32 - lane_center) * ROUTE_LANE_SPACING,
        ROUTE_Z,
    )
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

fn cycle_selection(
    available: &[(usize, Vec2)],
    selected: Option<usize>,
    direction: isize,
) -> Option<usize> {
    let selected_index = selected
        .and_then(|selected| available.iter().position(|(id, _)| *id == selected))
        .unwrap_or(0);
    let len = available.len() as isize;
    let next = (selected_index as isize + direction).rem_euclid(len) as usize;
    Some(available[next].0)
}

fn nearest_directional_selection(
    available: &[(usize, Vec2)],
    selected: Option<usize>,
    direction: Vec2,
) -> Option<usize> {
    let selected_pos = selected
        .and_then(|selected| available.iter().find(|(id, _)| *id == selected))
        .map(|(_, pos)| *pos)?;
    available
        .iter()
        .filter(|(id, _)| Some(*id) != selected)
        .filter_map(|(id, pos)| {
            let delta = *pos - selected_pos;
            (delta.dot(direction) > 1.0).then_some((*id, delta.length_squared()))
        })
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(id, _)| id)
}

fn node_outer_color(available: bool, selected: bool) -> Color {
    if selected {
        Color::rgb(1.0, 0.82, 0.24)
    } else if available {
        Color::rgb(0.9, 0.64, 0.2)
    } else {
        Color::rgba(0.24, 0.18, 0.14, 0.82)
    }
}

fn node_inner_color(kind: SquadNodeKind, available: bool, completed: bool) -> Color {
    if completed {
        return Color::rgb(0.38, 0.56, 0.34);
    }
    if !available {
        return Color::rgb(0.17, 0.13, 0.11);
    }
    match kind {
        SquadNodeKind::Fight => Color::rgb(0.72, 0.25, 0.18),
        SquadNodeKind::Recruit => Color::rgb(0.32, 0.62, 0.54),
        SquadNodeKind::Event => Color::rgb(0.48, 0.42, 0.74),
        SquadNodeKind::Elite => Color::rgb(0.78, 0.34, 0.16),
        SquadNodeKind::Boss => Color::rgb(0.72, 0.16, 0.12),
        SquadNodeKind::Rest => Color::rgb(0.36, 0.58, 0.32),
    }
}
