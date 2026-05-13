use bevy::prelude::*;
use std::collections::HashSet;

use crate::squad_battler::combat::{SquadCombatEvent, SquadCombatEventKind};

use super::app::VisualGameState;
use super::units::{self, UnitSprite, UnitToken};

const HIT_FLASH_SECONDS: f32 = 0.24;
const SCALE_PULSE_SECONDS: f32 = 0.28;
const ATTACK_WIGGLE_SECONDS: f32 = 0.24;
const FLOATER_SECONDS: f32 = 0.9;
const ATTACK_WIGGLE_FORWARD_DISTANCE: f32 = 14.0;
const ATTACK_WIGGLE_SIDE_DISTANCE: f32 = 4.5;
const PRIORITY_SCALE_PULSE: u8 = 10;
const PRIORITY_ATTACK_WIGGLE: u8 = 40;

#[derive(SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum CombatFxSet {
    Playback,
    Animation,
}

pub struct CombatFxPlugin;

impl Plugin for CombatFxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CombatFxSeenEvents>()
            .init_resource::<CombatFxTextStyle>()
            .add_systems(
                Update,
                (
                    playback_combat_events
                        .in_set(CombatFxSet::Playback)
                        .after(units::sync_unit_targets),
                    (
                        animate_hit_flashes,
                        (animate_scale_pulses, animate_attack_wiggles).chain(),
                        animate_floating_damage_text,
                    )
                        .in_set(CombatFxSet::Animation)
                        .after(CombatFxSet::Playback)
                        .after(units::animate_unit_motion),
                ),
            );
    }
}

#[derive(Resource, Default)]
pub struct CombatFxSeenEvents {
    seen: HashSet<CombatFxEventKey>,
}

#[derive(Resource)]
pub struct CombatFxTextStyle {
    pub font: Handle<Font>,
    pub font_size: f32,
}

impl Default for CombatFxTextStyle {
    fn default() -> Self {
        Self {
            font: Handle::default(),
            font_size: 26.0,
        }
    }
}

#[derive(Component)]
pub struct HitFlash {
    timer: Timer,
    base_color: Color,
}

#[derive(Component)]
pub struct ScalePulse {
    timer: Timer,
    base_scale: Vec3,
    amount: f32,
}

#[derive(Component)]
pub struct AttackWiggle {
    timer: Timer,
    direction: Vec2,
    applied_offset: Vec2,
    base_rotation: Quat,
}

#[derive(Component)]
pub struct FloatingDamageText {
    timer: Timer,
    velocity: Vec3,
    base_color: Color,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct CombatFxEventKey {
    time: u32,
    kind: u8,
    actor_id: String,
    target_id: Option<String>,
}

pub fn playback_combat_events(
    mut commands: Commands,
    state: Res<VisualGameState>,
    mut seen: ResMut<CombatFxSeenEvents>,
    text_style: Res<CombatFxTextStyle>,
    tokens: Query<(&UnitToken, &GlobalTransform)>,
    sprites: Query<(Entity, &UnitSprite, &GlobalTransform, &Transform, &Sprite)>,
) {
    let Some(fight) = state.view.live_fight.as_ref() else {
        seen.seen.clear();
        return;
    };

    let mut current_tail = HashSet::new();
    for event in &fight.events_tail {
        let key = event_key(event);
        current_tail.insert(key.clone());
        if !seen.seen.insert(key) {
            continue;
        }
        play_event(&mut commands, &text_style, event, &tokens, &sprites);
    }
    seen.seen.retain(|key| current_tail.contains(key));
}

pub fn animate_hit_flashes(
    mut commands: Commands,
    time: Res<Time>,
    mut flashes: Query<(Entity, &mut HitFlash, &mut Sprite)>,
) {
    for (entity, mut flash, mut sprite) in &mut flashes {
        flash.timer.tick(time.delta());
        let pct = flash.timer.fraction();
        let strength = (1.0 - pct).clamp(0.0, 1.0);
        sprite.color = mix_color(flash.base_color, Color::WHITE, strength * 0.72);
        if flash.timer.finished() {
            sprite.color = flash.base_color;
            commands.entity(entity).remove::<HitFlash>();
        }
    }
}

pub fn animate_scale_pulses(
    mut commands: Commands,
    time: Res<Time>,
    mut pulses: Query<(
        Entity,
        &mut ScalePulse,
        &mut Transform,
        Option<&AttackWiggle>,
    )>,
) {
    for (entity, mut pulse, mut transform, attack_wiggle) in &mut pulses {
        pulse.timer.tick(time.delta());
        if highest_transform_priority(attack_wiggle) > PRIORITY_SCALE_PULSE {
            transform.scale = pulse.base_scale;
            if pulse.timer.finished() {
                commands.entity(entity).remove::<ScalePulse>();
            }
            continue;
        }
        let pct = pulse.timer.fraction();
        let wave = (std::f32::consts::PI * pct).sin().max(0.0);
        transform.scale = pulse.base_scale * (1.0 + pulse.amount * wave);
        if pulse.timer.finished() {
            transform.scale = pulse.base_scale;
            commands.entity(entity).remove::<ScalePulse>();
        }
    }
}

pub fn animate_attack_wiggles(
    mut commands: Commands,
    time: Res<Time>,
    mut wiggles: Query<(Entity, &mut AttackWiggle, &mut Transform)>,
) {
    for (entity, mut wiggle, mut transform) in &mut wiggles {
        wiggle.timer.tick(time.delta());
        let pct = wiggle.timer.fraction();
        let thrust = (std::f32::consts::PI * pct).sin().max(0.0);
        let shake = (std::f32::consts::TAU * 3.0 * pct).sin();
        let side = Vec2::new(-wiggle.direction.y, wiggle.direction.x)
            * ATTACK_WIGGLE_SIDE_DISTANCE
            * shake
            * (1.0 - pct);
        let offset = wiggle.direction * ATTACK_WIGGLE_FORWARD_DISTANCE * thrust + side;
        let delta = offset - wiggle.applied_offset;
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
        transform.rotation = wiggle.base_rotation * Quat::from_rotation_z(shake * 0.18);
        wiggle.applied_offset = offset;
        if wiggle.timer.finished() {
            transform.translation.x -= wiggle.applied_offset.x;
            transform.translation.y -= wiggle.applied_offset.y;
            transform.rotation = wiggle.base_rotation;
            commands.entity(entity).remove::<AttackWiggle>();
        }
    }
}

pub fn animate_floating_damage_text(
    mut commands: Commands,
    time: Res<Time>,
    mut floaters: Query<(Entity, &mut FloatingDamageText, &mut Transform, &mut Text)>,
) {
    for (entity, mut floater, mut transform, mut text) in &mut floaters {
        floater.timer.tick(time.delta());
        transform.translation += floater.velocity * time.delta_seconds();
        let alpha = (1.0 - floater.timer.fraction()).clamp(0.0, 1.0);
        let mut color = floater.base_color;
        color.set_a(alpha);
        text.sections[0].style.color = color;
        if floater.timer.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn play_event(
    commands: &mut Commands,
    text_style: &CombatFxTextStyle,
    event: &SquadCombatEvent,
    tokens: &Query<(&UnitToken, &GlobalTransform)>,
    sprites: &Query<(Entity, &UnitSprite, &GlobalTransform, &Transform, &Sprite)>,
) {
    match event.kind {
        SquadCombatEventKind::Attack => {
            if let Some((actor, target)) = actor_and_target(event, tokens, sprites) {
                add_attack_wiggle(commands, actor, target);
            }
        }
        SquadCombatEventKind::Hit => {
            if let Some((target_entity, _, target_global, target_local, target_sprite)) =
                target_sprite(event, sprites)
            {
                add_hit_flash(commands, target_entity, target_sprite.color);
                add_scale_pulse(commands, target_entity, target_local.scale, 0.18);
                if let Some(damage) = event.damage {
                    spawn_damage_text(commands, text_style, target_global.translation(), damage);
                }
            }
        }
        SquadCombatEventKind::Miss => {
            if let Some((actor, target)) = actor_and_target(event, tokens, sprites) {
                add_attack_wiggle(commands, actor, target);
            }
        }
        SquadCombatEventKind::Death => {
            if let Some((target_entity, _, _, target_local, target_sprite)) =
                target_sprite(event, sprites)
            {
                add_hit_flash(commands, target_entity, target_sprite.color);
                add_scale_pulse(commands, target_entity, target_local.scale, 0.24);
            }
        }
        SquadCombatEventKind::Knockback => {
            if let Some((target_entity, _, _, target_local, target_sprite)) =
                target_sprite(event, sprites)
            {
                add_hit_flash(commands, target_entity, target_sprite.color);
                add_scale_pulse(commands, target_entity, target_local.scale, 0.16);
            }
        }
        SquadCombatEventKind::Move | SquadCombatEventKind::Skip | SquadCombatEventKind::Timeout => {
        }
    }
}

fn add_hit_flash(commands: &mut Commands, entity: Entity, base_color: Color) {
    commands.entity(entity).insert(HitFlash {
        timer: Timer::from_seconds(HIT_FLASH_SECONDS, TimerMode::Once),
        base_color,
    });
}

fn add_scale_pulse(commands: &mut Commands, entity: Entity, base_scale: Vec3, amount: f32) {
    commands.entity(entity).insert(ScalePulse {
        timer: Timer::from_seconds(SCALE_PULSE_SECONDS, TimerMode::Once),
        base_scale,
        amount,
    });
}

fn add_attack_wiggle(commands: &mut Commands, actor: FxActor<'_>, target: FxTarget<'_>) {
    let delta = target.position.truncate() - actor.position.truncate();
    let direction = delta.try_normalize().unwrap_or(Vec2::X);
    commands.entity(actor.entity).insert(AttackWiggle {
        timer: Timer::from_seconds(ATTACK_WIGGLE_SECONDS, TimerMode::Once),
        direction,
        applied_offset: Vec2::ZERO,
        base_rotation: actor.local.rotation,
    });
}

fn highest_transform_priority(attack_wiggle: Option<&AttackWiggle>) -> u8 {
    if attack_wiggle.is_some() {
        PRIORITY_ATTACK_WIGGLE
    } else {
        0
    }
}

fn spawn_damage_text(
    commands: &mut Commands,
    text_style: &CombatFxTextStyle,
    target_position: Vec3,
    damage: i32,
) {
    let color = if damage > 0 {
        Color::rgb(1.0, 0.31, 0.21)
    } else {
        Color::rgb(0.72, 0.88, 1.0)
    };
    let text = if damage > 0 {
        format!("-{damage}")
    } else {
        damage.to_string()
    };
    commands.spawn((
        FloatingDamageText {
            timer: Timer::from_seconds(FLOATER_SECONDS, TimerMode::Once),
            velocity: Vec3::new(0.0, 54.0, 0.0),
            base_color: color,
        },
        Text2dBundle {
            text: Text::from_section(
                text,
                TextStyle {
                    font: text_style.font.clone(),
                    font_size: text_style.font_size,
                    color,
                },
            )
            .with_justify(JustifyText::Center),
            transform: Transform::from_translation(target_position + Vec3::new(0.0, 30.0, 5.0)),
            ..default()
        },
    ));
}

struct FxActor<'a> {
    entity: Entity,
    local: &'a Transform,
    position: Vec3,
}

struct FxTarget<'a> {
    position: Vec3,
    _marker: std::marker::PhantomData<&'a ()>,
}

fn actor_and_target<'a>(
    event: &SquadCombatEvent,
    tokens: &'a Query<(&UnitToken, &GlobalTransform)>,
    sprites: &'a Query<(Entity, &UnitSprite, &GlobalTransform, &Transform, &Sprite)>,
) -> Option<(FxActor<'a>, FxTarget<'a>)> {
    let actor = actor_sprite(event, sprites)?;
    let target = target_root(event, tokens)?;
    Some((actor, target))
}

fn actor_sprite<'a>(
    event: &SquadCombatEvent,
    sprites: &'a Query<(Entity, &UnitSprite, &GlobalTransform, &Transform, &Sprite)>,
) -> Option<FxActor<'a>> {
    let (entity, _, global, local, _) = sprite_by_id(&event.actor_id, sprites)?;
    Some(FxActor {
        entity,
        local,
        position: global.translation(),
    })
}

fn target_sprite<'a>(
    event: &SquadCombatEvent,
    sprites: &'a Query<(Entity, &UnitSprite, &GlobalTransform, &Transform, &Sprite)>,
) -> Option<(
    Entity,
    &'a UnitSprite,
    &'a GlobalTransform,
    &'a Transform,
    &'a Sprite,
)> {
    sprite_by_id(event.target_id.as_deref()?, sprites)
}

fn target_root<'a>(
    event: &SquadCombatEvent,
    tokens: &'a Query<(&UnitToken, &GlobalTransform)>,
) -> Option<FxTarget<'a>> {
    let (_, global) = root_by_id(event.target_id.as_deref()?, tokens)?;
    Some(FxTarget {
        position: global.translation(),
        _marker: std::marker::PhantomData,
    })
}

fn root_by_id<'a>(
    id: &str,
    tokens: &'a Query<(&UnitToken, &GlobalTransform)>,
) -> Option<(&'a UnitToken, &'a GlobalTransform)> {
    tokens.iter().find(|(token, _)| token.id == id)
}

fn sprite_by_id<'a>(
    id: &str,
    sprites: &'a Query<(Entity, &UnitSprite, &GlobalTransform, &Transform, &Sprite)>,
) -> Option<(
    Entity,
    &'a UnitSprite,
    &'a GlobalTransform,
    &'a Transform,
    &'a Sprite,
)> {
    sprites
        .iter()
        .find(|(_, unit_sprite, _, _, _)| unit_sprite.id == id)
}

fn event_key(event: &SquadCombatEvent) -> CombatFxEventKey {
    CombatFxEventKey {
        time: event.time,
        kind: event_kind_id(&event.kind),
        actor_id: event.actor_id.clone(),
        target_id: event.target_id.clone(),
    }
}

fn event_kind_id(kind: &SquadCombatEventKind) -> u8 {
    match kind {
        SquadCombatEventKind::Move => 0,
        SquadCombatEventKind::Attack => 1,
        SquadCombatEventKind::Miss => 2,
        SquadCombatEventKind::Hit => 3,
        SquadCombatEventKind::Death => 4,
        SquadCombatEventKind::Knockback => 5,
        SquadCombatEventKind::Skip => 6,
        SquadCombatEventKind::Timeout => 7,
    }
}

fn mix_color(from: Color, to: Color, amount: f32) -> Color {
    let [from_r, from_g, from_b, from_a] = from.as_rgba_f32();
    let [to_r, to_g, to_b, to_a] = to.as_rgba_f32();
    let amount = amount.clamp(0.0, 1.0);
    Color::rgba(
        from_r + (to_r - from_r) * amount,
        from_g + (to_g - from_g) * amount,
        from_b + (to_b - from_b) * amount,
        from_a + (to_a - from_a) * amount,
    )
}
