//! Durable character documents. Catalog positions are process-local, never save identifiers.
use crate::game_logic::{ArmorCatalog, NpcPresetCatalog, PlayerConfig, ShieldCatalog, WeaponCatalog};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterDocument {
    pub schema_version: u32,
    pub player: Value,
}

impl CharacterDocument {
    pub fn capture(
        player: &PlayerConfig,
        weapons: &WeaponCatalog,
        armor: &ArmorCatalog,
        shields: &ShieldCatalog,
        npcs: &NpcPresetCatalog,
    ) -> Result<Self, String> {
        let mut value = serde_json::to_value(player).map_err(|e| e.to_string())?;
        let p = value.as_object_mut().ok_or("Invalid character")?;
        p.insert("weapon_id".into(), Value::from(weapons.get(player.weapon_id).ok_or("Unknown weapon")?.name.clone()));
        p.insert("offhand_weapon_id".into(), match player.offhand_weapon_id {
            Some(id) => Value::from(weapons.get(id).ok_or("Unknown offhand weapon")?.name.clone()),
            None => Value::Null,
        });
        p.insert("armor_id".into(), Value::from(armor.get(player.armor_id).ok_or("Unknown armor")?.label.clone()));
        p.insert("shield_id".into(), Value::from(shields.get(player.shield_id).ok_or("Unknown shield")?.label.clone()));
        p.insert("npc_preset".into(), match player.npc_preset {
            Some(id) => Value::from(npcs.get(id).ok_or("Unknown NPC preset")?.name.clone()),
            None => Value::Null,
        });
        // Selection and temporary profile overrides are not character attributes.
        p.remove("fighter_preset");
        p.remove("active_weapon_style_ids");
        Ok(Self { schema_version: 1, player: value })
    }

    pub fn restore(
        &self,
        weapons: &WeaponCatalog,
        armor: &ArmorCatalog,
        shields: &ShieldCatalog,
        npcs: &NpcPresetCatalog,
    ) -> Result<PlayerConfig, String> {
        if self.schema_version != 1 { return Err("Unsupported character document version".into()); }
        let mut value = self.player.clone();
        let p = value.as_object_mut().ok_or("Invalid character")?;
        fn resolve(value: &Value, names: impl Iterator<Item = String>, nullable: bool) -> Result<Value, String> {
            if nullable && value.is_null() { return Ok(Value::Null); }
            let name = value.as_str().ok_or("Expected a catalog name")?;
            let matches: Vec<_> = names.enumerate().filter(|(_, n)| n == name).collect();
            if matches.len() != 1 { return Err(format!("Missing or ambiguous catalog entry: {name}")); }
            Ok(serde_json::json!({ "index": matches[0].0, "_tag": null }))
        }
        for field in ["weapon_id", "offhand_weapon_id"] {
            let id = resolve(p.get(field).ok_or("Missing weapon reference")?, weapons.entries().iter().map(|w| w.name.clone()), field == "offhand_weapon_id")?;
            p.insert(field.into(), id);
        }
        let id = resolve(p.get("armor_id").ok_or("Missing armor")?, armor.entries().iter().map(|a| a.label.clone()), false)?;
        p.insert("armor_id".into(), id);
        let id = resolve(p.get("shield_id").ok_or("Missing shield")?, shields.entries().iter().map(|s| s.label.clone()), false)?;
        p.insert("shield_id".into(), id);
        let id = resolve(p.get("npc_preset").ok_or("Missing NPC reference")?, npcs.entries().iter().map(|n| n.name.clone()), true)?;
        p.insert("npc_preset".into(), id);
        p.insert("fighter_preset".into(), Value::Null);
        p.insert("active_weapon_style_ids".into(), Value::Null);
        let restored: PlayerConfig = serde_json::from_value(value).map_err(|e| format!("Invalid character: {e}"))?;
        let supported = Self::capture(&restored, weapons, armor, shields, npcs)?;
        // Serde normally ignores unknown fields. Refuse to load newer documents
        // if saving them from this client would silently erase those fields.
        fn check_fields(input: &Value, output: &Value) -> Result<(), String> {
            match (input, output) {
                (Value::Object(input), Value::Object(output)) => {
                    for (key, value) in input {
                        let supported = output.get(key).ok_or_else(|| format!("Unsupported character field: {key}. Update the app before editing this character."))?;
                        check_fields(value, supported)?;
                    }
                }
                (Value::Array(input), Value::Array(output)) => {
                    for (value, supported) in input.iter().zip(output) { check_fields(value, supported)?; }
                }
                _ => {}
            }
            Ok(())
        }
        check_fields(&self.player, &supported.player)?;
        Ok(restored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::catalog::Catalog, data, game_logic::WeaponId};

    #[test]
    fn complete_round_trip_survives_catalog_reordering() {
        let (weapons, armor, shields) = data::load_catalogs().unwrap();
        let npcs = Catalog::new(vec![]);
        let mut player = PlayerConfig::new("Cloud character", WeaponId::new(1));
        player.level = 9;
        player.intelligence = 17;
        player.offhand_weapon_id = Some(WeaponId::new(2));
        player.move_speed = 13.5;
        player.called_shot = true;
        player.proficiencies = vec!["tracking".into()];
        player.environment.temperature_c = -17;
        player.environment.bright_light = true;
        player.misc_modifiers.hp_bonus = 23;
        player.misc_modifiers.all_roll_bonus = 4;
        player.race_applied = true;
        player.knockback_step = 7;
        player.magic.learn_spell("echo_strike");
        player.magic.spell_parameters.insert("duration".into(), 3);
        player.magic.body_weight_lbs = 175;
        let doc = CharacterDocument::capture(&player, &weapons, &armor, &shields, &npcs).unwrap();
        let json = serde_json::to_string(&doc).unwrap();
        let doc: CharacterDocument = serde_json::from_str(&json).unwrap();
        let restored = doc.restore(&weapons, &armor, &shields, &npcs).unwrap();
        assert!(player == restored);
        let mut entries = weapons.entries().to_vec();
        entries.reverse();
        let reordered = Catalog::new(entries);
        let restored = doc.restore(&reordered, &armor, &shields, &npcs).unwrap();
        assert_eq!(weapons.get(player.weapon_id).unwrap().name, reordered.get(restored.weapon_id).unwrap().name);
        assert_eq!(doc, CharacterDocument::capture(&restored, &reordered, &armor, &shields, &npcs).unwrap());
    }

    #[test]
    fn missing_catalog_entries_and_future_versions_fail_without_substitution() {
        let (weapons, armor, shields) = data::load_catalogs().unwrap();
        let npcs = Catalog::new(vec![]);
        let player = PlayerConfig::new("Original", WeaponId::new(0));
        let mut doc = CharacterDocument::capture(&player, &weapons, &armor, &shields, &npcs).unwrap();
        doc.player["weapon_id"] = Value::from("Removed weapon");
        assert!(doc.restore(&weapons, &armor, &shields, &npcs).err().unwrap().contains("catalog entry"));
        doc.schema_version = 2;
        assert!(doc.restore(&weapons, &armor, &shields, &npcs).is_err());
    }

    #[test]
    fn unknown_nested_data_is_not_silently_discarded() {
        let (weapons, armor, shields) = data::load_catalogs().unwrap();
        let npcs = Catalog::new(vec![]);
        let player = PlayerConfig::new("Future character", WeaponId::new(0));
        let mut doc = CharacterDocument::capture(&player, &weapons, &armor, &shields, &npcs).unwrap();
        doc.player["magic"]["future_spell_data"] = Value::Bool(true);
        assert!(doc.restore(&weapons, &armor, &shields, &npcs).err().unwrap().contains("future_spell_data"));
    }
}
