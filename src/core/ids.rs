//! Typed identifiers for catalog entries.
//!
//! IDs should stay stable across data formats and UI selection indices.

use std::marker::PhantomData;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Id<Tag> {
    index: usize,
    _tag: PhantomData<Tag>,
}

impl<Tag> Id<Tag> {
    pub const fn new(index: usize) -> Self {
        Self {
            index,
            _tag: PhantomData,
        }
    }

    pub const fn index(&self) -> usize {
        self.index
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WeaponTag {}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArmorTag {}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShieldTag {}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NpcPresetTag {}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FighterPresetTag {}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TalentTag {}

pub type WeaponId = Id<WeaponTag>;
pub type ArmorId = Id<ArmorTag>;
pub type ShieldId = Id<ShieldTag>;
pub type NpcPresetId = Id<NpcPresetTag>;
pub type FighterPresetId = Id<FighterPresetTag>;
pub type TalentId = Id<TalentTag>;
