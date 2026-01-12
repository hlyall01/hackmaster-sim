//! Data adapters for loading catalogs and presets from JSON.

mod armor;
mod fighter_presets;
mod materials;
mod npc_presets;
mod weapons;

use crate::game_logic::{ArmorCatalog, ShieldCatalog, WeaponCatalog};

pub use armor::load_armor_catalog;
pub use fighter_presets::{load_fighter_presets, save_fighter_presets};
pub use npc_presets::load_npc_presets;
pub use weapons::{load_shield_catalog, load_weapon_catalog};

pub fn load_catalogs() -> Result<(WeaponCatalog, ArmorCatalog, ShieldCatalog), String> {
    let weapons = load_weapon_catalog("data/weapons.json")?;
    let armor = load_armor_catalog("data/armor.json")?;
    let shields = load_shield_catalog("data/weapons.json")?;
    let _materials = materials::load_materials("data/materials.json")?;
    Ok((weapons, armor, shields))
}
