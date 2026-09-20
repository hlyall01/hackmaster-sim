use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::squad_battler::combat::{BattleUnitStatus, BattleUnitView, GridPos, SquadCombatView};

use super::app::VisualGameState;
use super::assets;
use super::board::{BoardGeometry, TILE_WORLD_SIZE};

const UNIT_SPRITE_SIZE: f32 = TILE_WORLD_SIZE * 1.75;
const UNIT_SHADOW_WIDTH: f32 = TILE_WORLD_SIZE * 0.60;
const UNIT_SHADOW_HEIGHT: f32 = TILE_WORLD_SIZE * 0.20;
const HEALTH_BAR_WIDTH: f32 = TILE_WORLD_SIZE * 0.62;
const HEALTH_BAR_HEIGHT: f32 = 5.0;

const FOOZLE_FRAME_SIZE: f32 = 64.0;
const FOOZLE_COLUMNS: usize = 21;
const FOOZLE_ROWS: usize = 77;

const FOOZLE_SPRITES: &[&str] = &[
    "sprites/squad/foozle/Legend_MainCharacter_Green_Spritesheet_All_Animations.png",
    "sprites/squad/foozle/Legend_MainCharacter_Purple_Spritesheet_All_Animations.png",
    "sprites/squad/foozle/Legend_MainCharacter_Red_Spritesheet_All_Animations.png",
];

const PRIORITY_ATTACK_CLIP: u8 = 50;
const PRIORITY_DEATH_CLIP: u8 = 100;

#[derive(Component)]
pub struct UnitToken {
    pub id: String,
}

#[derive(Component)]
pub struct UnitVisual {
    pub id: String,
}

#[derive(Component)]
pub struct UnitSprite {
    pub id: String,
    pub status: BattleUnitStatus,
}

#[derive(Resource, Clone)]
pub(crate) struct UnitSpriteAtlas {
    handle: Handle<TextureAtlasLayout>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnitClipKind {
    Idle,
    Walk,
    Run,
    SwordAttack,
    HeavyAttack,
    RangedAttack,
    MagicAttack,
    Death,
    DeathLoop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnitDirection {
    Down,
    Up,
    Left,
    Right,
}

#[derive(Component)]
pub struct UnitAnimation {
    clip: UnitClipKind,
    direction: UnitDirection,
    facing: UnitDirection,
    frame_index: usize,
    frame_timer: Timer,
}

#[derive(Component)]
pub struct UnitAnimationOverride {
    kind: UnitClipKind,
    direction: UnitDirection,
    priority: u8,
    timer: Timer,
}

#[derive(Component)]
pub struct TargetWorldPosition(pub Vec3);

#[derive(Component)]
pub(crate) struct HealthFill {
    id: String,
}

pub(crate) fn setup_unit_sprite_atlas(
    mut commands: Commands,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.insert_resource(UnitSpriteAtlas {
        handle: texture_atlas_layouts.add(foozle_texture_atlas_layout()),
    });
}

pub(crate) fn spawn_units(
    commands: &mut Commands,
    geometry: BoardGeometry,
    fight: &SquadCombatView,
    asset_server: &AssetServer,
    atlas: &UnitSpriteAtlas,
) {
    for unit in &fight.combatants {
        spawn_unit(commands, geometry, unit, asset_server, atlas);
    }
}

pub(crate) fn sync_unit_targets(
    mut commands: Commands,
    geometry: Res<BoardGeometry>,
    asset_server: Res<AssetServer>,
    atlas: Res<UnitSpriteAtlas>,
    state: Res<VisualGameState>,
    mut tokens: Query<(Entity, &UnitToken, &mut TargetWorldPosition)>,
    mut unit_sprites: Query<(&mut UnitSprite, &mut Sprite), Without<HealthFill>>,
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

    for (entity, token, mut target) in &mut tokens {
        if let Some(unit) = units_by_id.get(token.id.as_str()) {
            if !present_ids.insert(token.id.clone()) {
                commands.entity(entity).despawn_recursive();
                continue;
            }
            target.0 = geometry.grid_to_world(GridPos::new(unit.x, unit.y), 1.0);
        } else {
            commands.entity(entity).despawn_recursive();
        }
    }

    for unit in &fight.combatants {
        if !present_ids.contains(&unit.id) {
            spawn_unit(&mut commands, *geometry, unit, &asset_server, &atlas);
        }
    }

    for (mut unit_sprite, mut sprite) in &mut unit_sprites {
        let Some(unit) = units_by_id.get(unit_sprite.id.as_str()) else {
            continue;
        };
        unit_sprite.status = unit.status;
        sprite.color = unit_tint(unit);
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

pub(crate) fn animate_unit_sprites(
    mut commands: Commands,
    time: Res<Time>,
    roots: Query<(&UnitToken, &Transform, &TargetWorldPosition)>,
    mut sprites: Query<(
        Entity,
        &UnitSprite,
        &mut UnitAnimation,
        &mut TextureAtlas,
        &mut Sprite,
        Option<&mut UnitAnimationOverride>,
    )>,
) {
    for (entity, unit_sprite, mut animation, mut atlas, mut sprite, override_clip) in &mut sprites {
        let root = roots
            .iter()
            .find(|(token, _, _)| token.id == unit_sprite.id)
            .map(|(_, transform, target)| (transform.translation, target.0));
        let Some((current, target)) = root else {
            continue;
        };

        let movement_delta = (target - current).truncate();
        let moving = movement_delta.length() > 0.5;
        if moving {
            if let Some(direction) = direction_from_delta(movement_delta) {
                animation.facing = direction;
            }
        }

        if let Some(mut override_clip) = override_clip {
            override_clip.timer.tick(time.delta());
            let clip = foozle_clip(override_clip.kind, override_clip.direction);
            sprite.flip_x = override_clip.direction.flip_x();
            let frame = one_shot_frame(clip, override_clip.timer.fraction());
            atlas.index = frame;
            if override_clip.timer.finished() {
                commands.entity(entity).remove::<UnitAnimationOverride>();
            }
            continue;
        }

        let base_clip = if unit_sprite.status == BattleUnitStatus::Downed {
            UnitClipKind::DeathLoop
        } else if moving {
            if movement_delta.length() > TILE_WORLD_SIZE * 1.25 {
                UnitClipKind::Run
            } else {
                UnitClipKind::Walk
            }
        } else {
            UnitClipKind::Idle
        };
        let direction = if unit_sprite.status == BattleUnitStatus::Downed {
            animation.facing
        } else {
            animation.facing
        };

        if animation.clip != base_clip || animation.direction != direction {
            animation.clip = base_clip;
            animation.direction = direction;
            animation.frame_index = 0;
            animation.frame_timer =
                Timer::from_seconds(frame_seconds(base_clip), TimerMode::Repeating);
        }

        animation.frame_timer.tick(time.delta());
        if animation.frame_timer.just_finished() {
            let clip = foozle_clip(animation.clip, animation.direction);
            animation.frame_index = (animation.frame_index + 1) % clip.len;
        }
        let clip = foozle_clip(animation.clip, animation.direction);
        sprite.flip_x = animation.direction.flip_x();
        atlas.index = clip.start + animation.frame_index.min(clip.len.saturating_sub(1));
    }
}

pub(crate) fn trigger_attack_animation(
    commands: &mut Commands,
    sprite_entity: Entity,
    actor_position: Vec3,
    target_position: Vec3,
    weapon: Option<&str>,
) {
    let direction = direction_from_delta((target_position - actor_position).truncate())
        .unwrap_or(UnitDirection::Right);
    insert_animation_override(
        commands,
        sprite_entity,
        UnitAnimationOverride {
            kind: attack_clip_for_weapon(weapon.unwrap_or_default()),
            direction,
            priority: PRIORITY_ATTACK_CLIP,
            timer: Timer::from_seconds(0.36, TimerMode::Once),
        },
    );
}

pub(crate) fn trigger_death_animation(
    commands: &mut Commands,
    sprite_entity: Entity,
    attacker_position: Vec3,
    target_position: Vec3,
) {
    let direction = direction_from_delta((attacker_position - target_position).truncate())
        .unwrap_or(UnitDirection::Down);
    insert_animation_override(
        commands,
        sprite_entity,
        UnitAnimationOverride {
            kind: UnitClipKind::Death,
            direction,
            priority: PRIORITY_DEATH_CLIP,
            timer: Timer::from_seconds(1.05, TimerMode::Once),
        },
    );
}

fn insert_animation_override(
    commands: &mut Commands,
    sprite_entity: Entity,
    requested: UnitAnimationOverride,
) {
    commands.add(move |world: &mut World| {
        let Some(existing) = world.get::<UnitAnimationOverride>(sprite_entity) else {
            if let Some(mut entity) = world.get_entity_mut(sprite_entity) {
                entity.insert(requested);
            }
            return;
        };
        if existing.priority <= requested.priority {
            if let Some(mut entity) = world.get_entity_mut(sprite_entity) {
                entity.insert(requested);
            }
        }
    });
}

fn spawn_unit(
    commands: &mut Commands,
    geometry: BoardGeometry,
    unit: &BattleUnitView,
    asset_server: &AssetServer,
    atlas: &UnitSpriteAtlas,
) {
    let pos = geometry.grid_to_world(GridPos::new(unit.x, unit.y), 1.0);
    let hp_width = HEALTH_BAR_WIDTH * health_pct(unit);
    let facing = initial_facing(unit);
    let initial_kind = base_clip_for_unit(unit, false);
    let initial_clip = foozle_clip(initial_kind, facing);
    commands
        .spawn((
            UnitToken {
                id: unit.id.clone(),
            },
            TargetWorldPosition(pos),
            SpatialBundle {
                transform: Transform::from_translation(pos),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    UnitVisual {
                        id: unit.id.clone(),
                    },
                    SpatialBundle::default(),
                ))
                .with_children(|visual| {
                    visual.spawn((
                        UnitSprite {
                            id: unit.id.clone(),
                            status: unit.status,
                        },
                        UnitAnimation {
                            clip: initial_kind,
                            direction: facing,
                            facing,
                            frame_index: 0,
                            frame_timer: Timer::from_seconds(
                                frame_seconds(initial_kind),
                                TimerMode::Repeating,
                            ),
                        },
                        SpriteSheetBundle {
                            texture: asset_server.load(unit_sprite_path(unit)),
                            atlas: TextureAtlas {
                                layout: atlas.handle.clone(),
                                index: initial_clip.start,
                            },
                            sprite: Sprite {
                                color: unit_tint(unit),
                                flip_x: facing.flip_x(),
                                custom_size: Some(Vec2::splat(UNIT_SPRITE_SIZE)),
                                ..default()
                            },
                            transform: Transform::from_xyz(0.0, 0.0, 0.1),
                            ..default()
                        },
                    ));
                    visual.spawn(SpriteBundle {
                        sprite: Sprite {
                            color: shadow_color(unit),
                            custom_size: Some(Vec2::new(UNIT_SHADOW_WIDTH, UNIT_SHADOW_HEIGHT)),
                            ..default()
                        },
                        transform: Transform::from_xyz(0.0, -TILE_WORLD_SIZE * 0.26, -0.1),
                        ..default()
                    });
                    visual.spawn(Text2dBundle {
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
                    visual.spawn(SpriteBundle {
                        sprite: Sprite {
                            color: assets::health_back_color(),
                            custom_size: Some(Vec2::new(HEALTH_BAR_WIDTH, HEALTH_BAR_HEIGHT + 2.0)),
                            ..default()
                        },
                        transform: Transform::from_xyz(0.0, -TILE_WORLD_SIZE * 0.42, 0.2),
                        ..default()
                    });
                    visual.spawn((
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
        });
}

fn unit_sprite_path(unit: &BattleUnitView) -> &'static str {
    if unit.team_id == 1 {
        return FOOZLE_SPRITES[2];
    }
    FOOZLE_SPRITES[stable_index(&unit.id, FOOZLE_SPRITES.len().saturating_sub(1))]
}

#[derive(Clone, Copy)]
struct FoozleClip {
    start: usize,
    len: usize,
}

fn foozle_texture_atlas_layout() -> TextureAtlasLayout {
    TextureAtlasLayout::from_grid(
        Vec2::splat(FOOZLE_FRAME_SIZE),
        FOOZLE_COLUMNS,
        FOOZLE_ROWS,
        None,
        None,
    )
}

fn foozle_clip(kind: UnitClipKind, direction: UnitDirection) -> FoozleClip {
    let row = match direction.atlas_direction() {
        AtlasDirection::Down => match kind {
            UnitClipKind::Idle => 1,
            UnitClipKind::Walk | UnitClipKind::Run => 3,
            UnitClipKind::SwordAttack => 7,
            UnitClipKind::HeavyAttack => 19,
            UnitClipKind::RangedAttack => 11,
            UnitClipKind::MagicAttack => 20,
            UnitClipKind::Death => 24,
            UnitClipKind::DeathLoop => 25,
        },
        AtlasDirection::Side => match kind {
            UnitClipKind::Idle => 27,
            UnitClipKind::Walk | UnitClipKind::Run => 29,
            UnitClipKind::SwordAttack => 33,
            UnitClipKind::HeavyAttack => 44,
            UnitClipKind::RangedAttack => 39,
            UnitClipKind::MagicAttack => 45,
            UnitClipKind::Death | UnitClipKind::DeathLoop => 50,
        },
        AtlasDirection::Up => match kind {
            UnitClipKind::Idle => 52,
            UnitClipKind::Walk | UnitClipKind::Run => 54,
            UnitClipKind::SwordAttack => 58,
            UnitClipKind::HeavyAttack => 69,
            UnitClipKind::RangedAttack => 64,
            UnitClipKind::MagicAttack => 70,
            UnitClipKind::Death => 75,
            UnitClipKind::DeathLoop => 76,
        },
    };
    let offset = match (kind, direction.atlas_direction()) {
        (UnitClipKind::DeathLoop, AtlasDirection::Side) => 11,
        _ => 0,
    };
    let len = match (kind, direction.atlas_direction()) {
        (UnitClipKind::Death, AtlasDirection::Down) => 21,
        (UnitClipKind::Death, AtlasDirection::Side) => 12,
        (UnitClipKind::Death, AtlasDirection::Up) => 19,
        (UnitClipKind::DeathLoop, AtlasDirection::Side) => 1,
        (UnitClipKind::DeathLoop, _) => 8,
        (UnitClipKind::Walk | UnitClipKind::Run, _) => 8,
        (UnitClipKind::RangedAttack, AtlasDirection::Down) => 4,
        (UnitClipKind::RangedAttack, AtlasDirection::Side) => 4,
        (UnitClipKind::RangedAttack, AtlasDirection::Up) => 4,
        _ => 4,
    };
    FoozleClip {
        start: row * FOOZLE_COLUMNS + offset,
        len,
    }
}

fn one_shot_frame(clip: FoozleClip, fraction: f32) -> usize {
    let frame = (fraction.clamp(0.0, 0.999) * clip.len as f32).floor() as usize;
    clip.start + frame.min(clip.len.saturating_sub(1))
}

fn frame_seconds(kind: UnitClipKind) -> f32 {
    match kind {
        UnitClipKind::Idle => 0.16,
        UnitClipKind::Walk => 0.09,
        UnitClipKind::Run => 0.06,
        UnitClipKind::SwordAttack
        | UnitClipKind::HeavyAttack
        | UnitClipKind::RangedAttack
        | UnitClipKind::MagicAttack => 0.07,
        UnitClipKind::Death => 0.055,
        UnitClipKind::DeathLoop => 0.14,
    }
}

fn initial_facing(unit: &BattleUnitView) -> UnitDirection {
    if unit.team_id == 0 {
        UnitDirection::Right
    } else {
        UnitDirection::Left
    }
}

fn base_clip_for_unit(unit: &BattleUnitView, moving: bool) -> UnitClipKind {
    if unit.status == BattleUnitStatus::Downed {
        UnitClipKind::DeathLoop
    } else if moving {
        UnitClipKind::Walk
    } else {
        UnitClipKind::Idle
    }
}

fn attack_clip_for_weapon(weapon: &str) -> UnitClipKind {
    let weapon = weapon.to_ascii_lowercase();
    if weapon.contains("axe")
        || weapon.contains("hatchet")
        || weapon.contains("hammer")
        || weapon.contains("mace")
    {
        UnitClipKind::HeavyAttack
    } else if weapon.contains("bow") || weapon.contains("crossbow") || weapon.contains("sling") {
        UnitClipKind::RangedAttack
    } else if weapon.contains("wand") || weapon.contains("staff") || weapon.contains("spell") {
        UnitClipKind::MagicAttack
    } else {
        UnitClipKind::SwordAttack
    }
}

fn direction_from_delta(delta: Vec2) -> Option<UnitDirection> {
    if delta.length_squared() <= f32::EPSILON {
        return None;
    }
    if delta.x.abs() > delta.y.abs() {
        if delta.x >= 0.0 {
            Some(UnitDirection::Right)
        } else {
            Some(UnitDirection::Left)
        }
    } else if delta.y >= 0.0 {
        Some(UnitDirection::Up)
    } else {
        Some(UnitDirection::Down)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AtlasDirection {
    Down,
    Side,
    Up,
}

impl UnitDirection {
    fn atlas_direction(self) -> AtlasDirection {
        match self {
            UnitDirection::Down => AtlasDirection::Down,
            UnitDirection::Up => AtlasDirection::Up,
            UnitDirection::Left | UnitDirection::Right => AtlasDirection::Side,
        }
    }

    fn flip_x(self) -> bool {
        self == UnitDirection::Left
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::squad_battler::combat::{BattleGrid, InitiativeView};
    use crate::squad_battler::state::SquadBattlerApp;
    use bevy::asset::AssetPlugin;
    use bevy::ecs::system::CommandQueue;

    #[test]
    fn visible_unit_elements_are_children_of_the_animated_visual_rig() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Image>();
        app.init_asset::<TextureAtlasLayout>();
        let asset_server = app.world.resource::<AssetServer>().clone();
        let atlas = test_unit_sprite_atlas(&mut app);
        let fight = sample_combat_view();
        let geometry = BoardGeometry::new(fight.grid);

        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &app.world);
            spawn_units(&mut commands, geometry, &fight, &asset_server, &atlas);
        }
        queue.apply(&mut app.world);

        let root = app
            .world
            .query_filtered::<Entity, With<UnitToken>>()
            .single(&app.world);
        let root_children = app
            .world
            .get::<Children>(root)
            .expect("unit root should own visible children");
        let visual_children = root_children
            .iter()
            .copied()
            .filter(|child| app.world.get::<UnitVisual>(*child).is_some())
            .collect::<Vec<_>>();
        let static_visible_children = root_children
            .iter()
            .copied()
            .filter(|child| {
                app.world.get::<Sprite>(*child).is_some()
                    || app.world.get::<Text>(*child).is_some()
                    || app.world.get::<HealthFill>(*child).is_some()
            })
            .collect::<Vec<_>>();

        assert_eq!(
            visual_children.len(),
            1,
            "unit root should own exactly one animated visual rig"
        );
        assert!(
            static_visible_children.is_empty(),
            "unit root has static visible children outside the animated visual rig: {:?}",
            static_visible_children
        );

        let rig_children = app
            .world
            .get::<Children>(visual_children[0])
            .expect("visual rig should own sprite, label, shadow, and health children");
        let body_sprite_children = rig_children
            .iter()
            .copied()
            .filter(|child| app.world.get::<UnitSprite>(*child).is_some())
            .collect::<Vec<_>>();
        assert_eq!(
            body_sprite_children.len(),
            1,
            "visual rig should own exactly one body sprite"
        );
        assert!(
            app.world
                .get::<TextureAtlas>(body_sprite_children[0])
                .is_some(),
            "body sprite should be atlas-driven, not a second static sprite"
        );
        let body_sprite = app
            .world
            .get::<Sprite>(body_sprite_children[0])
            .expect("body entity should render a sprite");
        assert_eq!(
            body_sprite.custom_size,
            Some(Vec2::splat(UNIT_SPRITE_SIZE)),
            "Foozle sprites need a larger draw box because the frames have substantial transparent padding"
        );

        let visible_rig_children = rig_children
            .iter()
            .copied()
            .filter(|child| {
                app.world.get::<Sprite>(*child).is_some()
                    || app.world.get::<Text>(*child).is_some()
                    || app.world.get::<HealthFill>(*child).is_some()
            })
            .collect::<Vec<_>>();
        assert!(
            !visible_rig_children.is_empty(),
            "animated visual rig should own the visible unit elements"
        );
    }

    #[test]
    fn sync_unit_targets_despawns_duplicate_unit_roots() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Image>();
        app.init_asset::<TextureAtlasLayout>();
        let atlas = test_unit_sprite_atlas(&mut app);
        app.insert_resource(atlas.clone());
        app.add_systems(Update, sync_unit_targets);

        let asset_server = app.world.resource::<AssetServer>().clone();
        let fight = sample_combat_view();
        let geometry = BoardGeometry::new(fight.grid);
        let mut game = SquadBattlerApp::new().expect("test app should load catalogs");
        let mut view = game.new_run(Some(1));
        view.live_fight = Some(fight.clone());
        app.insert_resource(geometry);
        app.insert_resource(VisualGameState { app: game, view });

        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &app.world);
            spawn_units(&mut commands, geometry, &fight, &asset_server, &atlas);
            spawn_units(&mut commands, geometry, &fight, &asset_server, &atlas);
        }
        queue.apply(&mut app.world);
        assert_eq!(
            app.world.query::<&UnitToken>().iter(&app.world).count(),
            2,
            "test setup should create two roots with the same unit id"
        );

        app.update();

        assert_eq!(
            app.world.query::<&UnitToken>().iter(&app.world).count(),
            1,
            "sync should keep only one rendered root per combatant id"
        );
        assert_eq!(
            app.world.query::<&UnitVisual>().iter(&app.world).count(),
            1,
            "sync should remove the duplicate visual rig with its root"
        );
        assert_eq!(
            app.world.query::<&UnitSprite>().iter(&app.world).count(),
            1,
            "sync should remove the duplicate body sprite with its root"
        );
    }

    #[test]
    fn downed_units_spawn_on_foozle_death_hold_frame() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Image>();
        app.init_asset::<TextureAtlasLayout>();
        let asset_server = app.world.resource::<AssetServer>().clone();
        let atlas = test_unit_sprite_atlas(&mut app);
        let mut fight = sample_combat_view();
        fight.combatants[0].status = BattleUnitStatus::Downed;
        fight.combatants[0].hp = 0;
        let geometry = BoardGeometry::new(fight.grid);

        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &app.world);
            spawn_units(&mut commands, geometry, &fight, &asset_server, &atlas);
        }
        queue.apply(&mut app.world);

        let atlas = app
            .world
            .query_filtered::<&TextureAtlas, With<UnitSprite>>()
            .single(&app.world);
        assert_eq!(
            atlas.index,
            foozle_clip(UnitClipKind::DeathLoop, UnitDirection::Left).start,
            "downed units should render as a fallen Foozle sprite, not an idle standing sprite"
        );
    }

    #[test]
    fn animation_override_priority_keeps_death_over_attack() {
        let mut app = App::new();
        let sprite_entity = app
            .world
            .spawn(UnitAnimationOverride {
                kind: UnitClipKind::Death,
                direction: UnitDirection::Down,
                priority: PRIORITY_DEATH_CLIP,
                timer: Timer::from_seconds(1.0, TimerMode::Once),
            })
            .id();

        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &app.world);
            trigger_attack_animation(
                &mut commands,
                sprite_entity,
                Vec3::ZERO,
                Vec3::X,
                Some("Sword"),
            );
        }
        queue.apply(&mut app.world);

        let animation = app
            .world
            .get::<UnitAnimationOverride>(sprite_entity)
            .expect("override should still exist");
        assert_eq!(
            animation.kind,
            UnitClipKind::Death,
            "lower-priority attack animation must not replace death animation"
        );
    }

    #[test]
    fn animation_override_priority_allows_death_to_replace_attack() {
        let mut app = App::new();
        let sprite_entity = app
            .world
            .spawn(UnitAnimationOverride {
                kind: UnitClipKind::SwordAttack,
                direction: UnitDirection::Right,
                priority: PRIORITY_ATTACK_CLIP,
                timer: Timer::from_seconds(1.0, TimerMode::Once),
            })
            .id();

        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &app.world);
            trigger_death_animation(
                &mut commands,
                sprite_entity,
                Vec3::new(-1.0, 0.0, 0.0),
                Vec3::ZERO,
            );
        }
        queue.apply(&mut app.world);

        let animation = app
            .world
            .get::<UnitAnimationOverride>(sprite_entity)
            .expect("override should still exist");
        assert_eq!(
            animation.kind,
            UnitClipKind::Death,
            "higher-priority death animation should replace attack animation"
        );
    }

    fn test_unit_sprite_atlas(app: &mut App) -> UnitSpriteAtlas {
        let handle = app
            .world
            .resource_mut::<Assets<TextureAtlasLayout>>()
            .add(foozle_texture_atlas_layout());
        UnitSpriteAtlas { handle }
    }

    fn sample_combat_view() -> SquadCombatView {
        SquadCombatView {
            grid: BattleGrid::default(),
            elapsed_seconds: 0,
            max_seconds: 180,
            running: false,
            done: false,
            winner_team: None,
            combatants: vec![BattleUnitView {
                id: "enemy-1-0".to_string(),
                name: "Test Enemy".to_string(),
                team_id: 1,
                x: 4,
                y: 3,
                hp: 10,
                max_hp: 10,
                status: BattleUnitStatus::Alive,
                weapon: "Claws".to_string(),
                reach_ft: 5.0,
                max_range_ft: None,
                move_tiles: 4,
                initiative: 0.0,
                intent: "Testing".to_string(),
            }],
            initiative: Vec::<InitiativeView>::new(),
            log_tail: Vec::new(),
            events_tail: Vec::new(),
        }
    }
}
