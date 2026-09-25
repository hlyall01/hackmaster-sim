//! Compiled catalog effects and their active arena state.
use crate::core::magic::{
    EffectDefinition, EffectRule, EffectTarget, EffectValues, MagicError, MagicLoadout,
    RangePolicy, SpellCatalogEntry,
};
use std::ops::{Deref, DerefMut};

#[derive(Clone, Debug, PartialEq)]
pub struct ConfiguredEffect {
    pub catalog: std::sync::Arc<SpellCatalogEntry>,
    pub id: String,
    pub name: String,
    pub definition: EffectDefinition,
    pub values: EffectValues,
    pub target: Option<usize>,
}
impl Deref for ConfiguredEffect {
    type Target = EffectValues;
    fn deref(&self) -> &EffectValues {
        &self.values
    }
}
impl DerefMut for ConfiguredEffect {
    fn deref_mut(&mut self) -> &mut EffectValues {
        &mut self.values
    }
}
impl ConfiguredEffect {
    pub fn from_catalog(
        spell: &SpellCatalogEntry,
        loadout: &MagicLoadout,
    ) -> Result<(Self, u32), MagicError> {
        let definition = spell.mechanics.clone().ok_or(MagicError::InvalidSpell)?;
        let (values, cost) = spell.resolve_values(loadout)?;
        Ok((
            Self {
                catalog: std::sync::Arc::new(spell.clone()),
                id: spell.id.clone(),
                name: spell.name.clone(),
                definition,
                values,
                target: None,
            },
            cost,
        ))
    }
    pub fn timed_buff(&self) -> Option<super::TemporaryEffect> {
        let EffectRule::TimedBuff {
            movement_defense,
            average_incoming_damage,
        } = &self.definition.effect
        else {
            return None;
        };
        let mut buff = super::TemporaryEffect::new(&self.id, self.duration as i32);
        buff.movement_defense = movement_defense.clone();
        buff.average_damage_radius = average_incoming_damage.then_some(self.radius);
        Some(buff)
    }
    pub fn needs_target(&self) -> bool {
        self.definition.target == EffectTarget::Enemy
    }
    pub fn channelled(&self) -> bool {
        self.definition.effect.channelled()
    }
    pub fn targeting_range(&self, weapon_reach: f32) -> f32 {
        match self.definition.ai_range {
            RangePolicy::SpellRange => self.range,
            RangePolicy::WeaponReach => weapon_reach,
            RangePolicy::SweepDistance => self.speed * self.duration as f32,
            RangePolicy::AnyEnemy => f32::INFINITY,
        }
    }
    pub fn damage_rule(&self) -> &crate::core::magic::DamageRule {
        match &self.definition.effect {
            EffectRule::Damage { damage, .. } | EffectRule::SummonMelee { damage, .. } => damage,
            _ => unreachable!("validated damage effect"),
        }
    }
}
#[derive(Clone, Debug)]
pub struct SweepingField {
    pub spell: ConfiguredEffect,
    pub origin: super::GridPos,
    pub direction: (f32, f32),
    pub started_at: u32,
    pub touched: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct SummonedLimbs {
    pub spell: ConfiguredEffect,
    pub expires_at: u32,
    pub next_attack: u32,
    pub hp: Vec<i32>,
}
#[derive(Clone, Debug)]
pub struct OppressiveField {
    pub spell: ConfiguredEffect,
    pub center: super::GridPos,
    pub radius: f32,
    pub expires_at: u32,
    pub delayed: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct ChargedObject {
    /// Enemy-targeted blasts stay at the impact point; carried charges follow the caster.
    pub center: Option<super::GridPos>,
    pub spell: ConfiguredEffect,
    pub detonates_at: u32,
}
