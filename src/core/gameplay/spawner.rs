use crate::core::ids::NpcPresetId;
use crate::core::rng::SimRng;
use crate::core::types::EnemyProfile;
use rand::Rng;

#[derive(Clone, Debug)]
pub struct EnemySpawnEntry {
    pub preset_id: NpcPresetId,
    pub min_level: u8,
    pub max_level: u8,
    pub weight: u32,
}

impl EnemySpawnEntry {
    pub fn matches_level(&self, level: u8) -> bool {
        level >= self.min_level && level <= self.max_level
    }
}

#[derive(Clone, Debug, Default)]
pub struct EnemySpawner {
    entries: Vec<EnemySpawnEntry>,
}

impl EnemySpawner {
    pub fn new(entries: Vec<EnemySpawnEntry>) -> Self {
        Self { entries }
    }

    pub fn entries(&self) -> &[EnemySpawnEntry] {
        &self.entries
    }

    pub fn push(&mut self, entry: EnemySpawnEntry) {
        self.entries.push(entry);
    }

    pub fn spawn_for_level(&self, level: u8, rng: &mut SimRng) -> Option<EnemyProfile> {
        let total_weight: u32 = self
            .entries
            .iter()
            .filter(|entry| entry.weight > 0 && entry.matches_level(level))
            .map(|entry| entry.weight)
            .sum();

        if total_weight == 0 {
            return None;
        }

        let mut roll = rng.gen_range(0..total_weight);
        for entry in self
            .entries
            .iter()
            .filter(|entry| entry.weight > 0 && entry.matches_level(level))
        {
            if roll < entry.weight {
                return Some(EnemyProfile {
                    level,
                    preset_id: entry.preset_id,
                });
            }
            roll -= entry.weight;
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawner_filters_by_level() {
        let spawner = EnemySpawner::new(vec![
            EnemySpawnEntry {
                preset_id: NpcPresetId::new(0),
                min_level: 1,
                max_level: 1,
                weight: 10,
            },
            EnemySpawnEntry {
                preset_id: NpcPresetId::new(1),
                min_level: 3,
                max_level: 5,
                weight: 10,
            },
        ]);

        let mut rng = SimRng::from_seed(1);
        let profile = spawner.spawn_for_level(4, &mut rng).expect("spawn");
        assert_eq!(profile.preset_id, NpcPresetId::new(1));
        assert_eq!(profile.level, 4);
    }

    #[test]
    fn spawner_is_deterministic_for_seed() {
        let spawner = EnemySpawner::new(vec![
            EnemySpawnEntry {
                preset_id: NpcPresetId::new(0),
                min_level: 1,
                max_level: 3,
                weight: 1,
            },
            EnemySpawnEntry {
                preset_id: NpcPresetId::new(1),
                min_level: 1,
                max_level: 3,
                weight: 3,
            },
        ]);

        let mut rng_a = SimRng::from_seed(42);
        let mut rng_b = SimRng::from_seed(42);
        let a = spawner.spawn_for_level(2, &mut rng_a);
        let b = spawner.spawn_for_level(2, &mut rng_b);
        assert_eq!(a, b);
    }
}
