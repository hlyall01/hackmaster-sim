use crate::core::rng::SimRng;
use rand::Rng;
use std::ops::RangeInclusive;

#[derive(Clone, Debug)]
pub struct LootItemEntry {
    pub name: String,
    pub weight: u32,
}

#[derive(Clone, Debug)]
pub struct LootTable {
    pub gold_range: RangeInclusive<u32>,
    pub xp_per_level: u32,
    pub item_table: Vec<LootItemEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LootRoll {
    pub gold: u32,
    pub xp: u32,
    pub items: Vec<String>,
}

impl LootTable {
    pub fn roll(&self, level: u8, rng: &mut SimRng) -> LootRoll {
        let min_gold = *self.gold_range.start();
        let max_gold = *self.gold_range.end();
        let (min_gold, max_gold) = if min_gold <= max_gold {
            (min_gold, max_gold)
        } else {
            (max_gold, min_gold)
        };
        let gold = if min_gold == max_gold {
            min_gold
        } else {
            rng.gen_range(min_gold..=max_gold)
        };
        let xp = self.xp_per_level.saturating_mul(level as u32);

        let total_weight: u32 = self
            .item_table
            .iter()
            .filter(|entry| entry.weight > 0)
            .map(|entry| entry.weight)
            .sum();

        let item = if total_weight == 0 {
            None
        } else {
            let mut roll = rng.gen_range(0..total_weight);
            let mut selected: Option<&LootItemEntry> = None;
            for entry in self.item_table.iter().filter(|entry| entry.weight > 0) {
                if roll < entry.weight {
                    selected = Some(entry);
                    break;
                }
                roll -= entry.weight;
            }
            selected
        };

        LootRoll {
            gold,
            xp,
            items: item
                .map(|entry| vec![entry.name.clone()])
                .unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loot_roll_is_deterministic_for_seed() {
        let table = LootTable {
            gold_range: 10..=20,
            xp_per_level: 5,
            item_table: vec![
                LootItemEntry {
                    name: "Potion".to_string(),
                    weight: 1,
                },
                LootItemEntry {
                    name: "Gem".to_string(),
                    weight: 2,
                },
            ],
        };

        let mut rng_a = SimRng::from_seed(7);
        let mut rng_b = SimRng::from_seed(7);
        let roll_a = table.roll(3, &mut rng_a);
        let roll_b = table.roll(3, &mut rng_b);
        assert_eq!(roll_a, roll_b);
    }

    #[test]
    fn loot_roll_handles_empty_items() {
        let table = LootTable {
            gold_range: 5..=5,
            xp_per_level: 1,
            item_table: Vec::new(),
        };

        let mut rng = SimRng::from_seed(1);
        let roll = table.roll(1, &mut rng);
        assert!(roll.items.is_empty());
    }
}
