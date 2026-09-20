//! Daily Ego generation, with gains rounded to the nearest five before capping.
//! Reachable uncapped amounts differ by multiples of five. Both choices preserve
//! their ordering on this grid, and maximum Ego dominates every uncapped amount.
//! Choosing the larger result today therefore also maximizes the final amount.

pub const MAX_EGO_GENERATION_DAYS: u32 = 10_000;

#[derive(Clone, Copy, Debug)]
pub struct EgoGenerationInput {
    pub maximum: f64,
    pub current: f64,
    pub days: u32,
    pub wisdom: u32,
    pub charisma: u32,
}

impl EgoGenerationInput {
    pub fn use_fighter_preset(&mut self, preset: &super::FighterPreset) {
        self.wisdom = u32::from(preset.wisdom);
        self.charisma = u32::from(preset.charisma);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EgoChoice {
    Wisdom,
    Charisma,
    Either,
}

#[derive(Clone, Debug)]
pub struct EgoGenerationDay {
    pub day: u32,
    pub starting: f64,
    pub wisdom_gain: f64,
    pub charisma_gain: f64,
    pub choice: EgoChoice,
    pub ending: f64,
}

#[derive(Clone, Debug)]
pub struct EgoGenerationPlan {
    pub final_ego: f64,
    pub total_gain: f64,
    pub days: Vec<EgoGenerationDay>,
}

pub fn calculate_ego_generation(
    input: EgoGenerationInput,
) -> Result<EgoGenerationPlan, &'static str> {
    if !input.maximum.is_finite() || input.maximum < 0.0 {
        return Err("Maximum Ego must be a finite, nonnegative number.");
    }
    if !input.current.is_finite() || input.current < 0.0 || input.current > input.maximum {
        return Err("Current Ego must be between zero and Maximum Ego.");
    }
    if input.days > MAX_EGO_GENERATION_DAYS {
        return Err("Choose at most 10,000 days.");
    }
    let mut current = input.current;
    let mut days = Vec::with_capacity(input.days as usize);
    for day in 1..=input.days {
        let wisdom_gain =
            ((input.maximum - current) * (0.02 * input.wisdom as f64) / 5.0).round() * 5.0;
        let charisma_gain = (current * (0.02 * input.charisma as f64) / 5.0).round() * 5.0;
        let wisdom = (current + wisdom_gain).min(input.maximum);
        let charisma = (current + charisma_gain).min(input.maximum);
        let choice = if wisdom > charisma {
            EgoChoice::Wisdom
        } else if charisma > wisdom {
            EgoChoice::Charisma
        } else {
            EgoChoice::Either
        };
        let ending = wisdom.max(charisma);
        days.push(EgoGenerationDay {
            day,
            starting: current,
            wisdom_gain: wisdom - current,
            charisma_gain: charisma - current,
            choice,
            ending,
        });
        current = ending;
    }
    Ok(EgoGenerationPlan {
        final_ego: current,
        total_gain: current - input.current,
        days,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ego_preset_refresh_preserves_the_scenario_and_uses_updated_attributes() {
        let mut preset = crate::test_support::fighter("Halberd fixture");
        let mut input = EgoGenerationInput {
            maximum: 1000.0,
            current: 500.0,
            days: 4,
            wisdom: 12,
            charisma: 9,
        };
        preset.wisdom = 13;
        preset.charisma = 11;
        input.use_fighter_preset(&preset);
        assert_eq!((input.wisdom, input.charisma), (13, 11));
        assert_eq!((input.maximum, input.current, input.days), (1000.0, 500.0, 4));
        let plan = calculate_ego_generation(input).unwrap();
        assert_eq!(plan.days[0].wisdom_gain, 130.0);
        assert_eq!(plan.days[0].charisma_gain, 110.0);
    }

    #[test]
    fn ego_arthur_switches_from_restoration_to_growth() {
        let plan = calculate_ego_generation(EgoGenerationInput {
            maximum: 1000.0,
            current: 500.0,
            days: 4,
            wisdom: 12,
            charisma: 9,
        })
        .unwrap();
        assert_eq!(plan.days[0].choice, EgoChoice::Wisdom);
        assert_eq!(plan.days[0].ending, 620.0);
        assert_eq!(plan.days[1].choice, EgoChoice::Charisma);
        assert_eq!(plan.days[1].ending, 730.0);
        assert_eq!(plan.final_ego, 1000.0);
        assert_eq!(plan.total_gain, 500.0);
    }

    #[test]
    fn ego_greedy_matches_every_possible_daily_sequence() {
        for wisdom in [0, 1, 12, 50, 60] {
            for charisma in [0, 1, 9, 50, 60] {
                for current in [0.0, 25.5, 50.0, 99.9, 100.0] {
                    let input = EgoGenerationInput {
                        maximum: 100.0,
                        current,
                        days: 8,
                        wisdom,
                        charisma,
                    };
                    let plan = calculate_ego_generation(input).unwrap();
                    let mut best = current;
                    for sequence in 0..(1 << input.days) {
                        let mut ego = current;
                        for day in 0..input.days {
                            let gain = if sequence & (1 << day) == 0 {
                                (100.0 - ego) * 0.02 * wisdom as f64
                            } else {
                                ego * 0.02 * charisma as f64
                            };
                            ego = (ego + (gain / 5.0).round() * 5.0).min(100.0);
                        }
                        best = best.max(ego);
                    }
                    assert!((plan.final_ego - best).abs() < 1e-9, "{input:?}");
                    assert!(
                        plan.days
                            .iter()
                            .all(|day| day.ending >= day.starting && day.ending <= input.maximum)
                    );
                }
            }
        }
    }

    #[test]
    fn ego_rounds_daily_gains_to_five_before_applying_the_cap() {
        let input = EgoGenerationInput {
            maximum: 103.0,
            current: 28.0,
            days: 1,
            wisdom: 5,
            charisma: 0,
        };
        let plan = calculate_ego_generation(input).unwrap();
        // 10% of 75 missing is 7.5: halfway rounds up to a gain of 10.
        assert_eq!(plan.days[0].wisdom_gain, 10.0);
        assert_eq!(plan.final_ego, 38.0);
        let plan = calculate_ego_generation(EgoGenerationInput {
            current: 100.0,
            charisma: 9,
            ..input
        })
        .unwrap();
        assert_eq!(plan.days[0].charisma_gain, 3.0);
        assert_eq!(plan.final_ego, 103.0);
        let plan = calculate_ego_generation(EgoGenerationInput {
            current: 100.0,
            ..input
        })
        .unwrap();
        assert_eq!(plan.final_ego, 100.0);
    }

    #[test]
    fn ego_handles_zero_days_zero_capacity_ties_and_invalid_inputs() {
        let input = EgoGenerationInput {
            maximum: 100.0,
            current: 50.0,
            days: 0,
            wisdom: 12,
            charisma: 12,
        };
        let plan = calculate_ego_generation(input).unwrap();
        assert_eq!(plan.final_ego, 50.0);
        assert_eq!(plan.total_gain, 0.0);
        assert!(plan.days.is_empty());
        let plan = calculate_ego_generation(EgoGenerationInput { days: 1, ..input }).unwrap();
        assert_eq!(plan.days[0].choice, EgoChoice::Either);
        let plan = calculate_ego_generation(EgoGenerationInput {
            maximum: 0.0,
            current: 0.0,
            days: 3,
            ..input
        })
        .unwrap();
        assert_eq!(plan.final_ego, 0.0);
        for current in [-1.0, 101.0, f64::NAN, f64::INFINITY] {
            assert!(calculate_ego_generation(EgoGenerationInput { current, ..input }).is_err());
        }
        for maximum in [-1.0, f64::NAN, f64::INFINITY] {
            assert!(calculate_ego_generation(EgoGenerationInput { maximum, ..input }).is_err());
        }
        assert!(
            calculate_ego_generation(EgoGenerationInput {
                days: MAX_EGO_GENERATION_DAYS + 1,
                ..input
            })
            .is_err()
        );
    }
}
