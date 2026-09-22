//! Wound recovery shared by the simulator tools.

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Wound {
    pub damage: u32,
    pub healing_progress_steps: u32,
}

pub fn heal_wounds(wounds: &mut Vec<Wound>, rest_days: u32, fast_healer: bool, resting: bool) {
    let mut rest_steps = rest_days.saturating_mul(4);
    if !resting {
        rest_steps /= 2;
    }
    for wound in wounds.iter_mut() {
        if wound.damage == 0 {
            continue;
        }

        let mut healing_progress = wound.healing_progress_steps.saturating_add(rest_steps);

        while wound.damage > 0 {
            let required_steps = required_healing_steps(wound.damage, fast_healer);
            if healing_progress < required_steps {
                break;
            }
            healing_progress = healing_progress.saturating_sub(required_steps);
            wound.damage -= 1;
        }

        if wound.damage == 0 {
            wound.healing_progress_steps = 0;
        } else {
            wound.healing_progress_steps = healing_progress;
        }
    }
    wounds.retain(|wound| wound.damage > 0);
}

pub fn required_healing_steps(damage: u32, fast_healer: bool) -> u32 {
    if fast_healer {
        if damage == 1 {
            1
        } else {
            damage.saturating_sub(1).saturating_mul(2)
        }
    } else {
        damage.saturating_mul(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heals_wounds_with_rest_days() {
        let mut wounds = vec![Wound {
            damage: 7,
            healing_progress_steps: 0,
        }];

        heal_wounds(&mut wounds, 1, false, true);
        assert_eq!(
            wounds,
            vec![Wound {
                damage: 7,
                healing_progress_steps: 4
            }]
        );
    }

    #[test]
    fn halves_healing_without_rest() {
        let mut wounds = vec![Wound {
            damage: 7,
            healing_progress_steps: 0,
        }];

        heal_wounds(&mut wounds, 1, false, false);
        assert_eq!(
            wounds,
            vec![Wound {
                damage: 7,
                healing_progress_steps: 2
            }]
        );
    }

    #[test]
    fn fast_healer_recovers_wounds_faster() {
        let mut normal = vec![Wound {
            damage: 3,
            healing_progress_steps: 0,
        }];
        let mut fast = normal.clone();

        heal_wounds(&mut normal, 1, false, true);
        heal_wounds(&mut fast, 1, true, true);

        assert_eq!(
            normal,
            vec![Wound {
                damage: 3,
                healing_progress_steps: 4
            }]
        );
        assert_eq!(
            fast,
            vec![Wound {
                damage: 2,
                healing_progress_steps: 0
            }]
        );
    }
}
