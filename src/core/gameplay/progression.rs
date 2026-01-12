use crate::core::types::PlayerProfile;

#[derive(Clone, Debug)]
pub struct XpCurve {
    pub base: u32,
    pub per_level: u32,
}

impl XpCurve {
    pub fn xp_for_level(&self, level: u8) -> u32 {
        if level <= 1 {
            0
        } else {
            let steps = level.saturating_sub(2) as u32;
            self.base
                .saturating_add(self.per_level.saturating_mul(steps))
        }
    }

    pub fn xp_for_next_level(&self, level: u8) -> u32 {
        self.xp_for_level(level.saturating_add(1))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LevelUpResult {
    pub levels_gained: u8,
}

pub fn apply_xp(profile: &mut PlayerProfile, curve: &XpCurve, gained_xp: u32) -> LevelUpResult {
    apply_xp_with(profile, curve, gained_xp, |_profile, _new_level| {})
}

pub fn apply_xp_with<F: FnMut(&mut PlayerProfile, u8)>(
    profile: &mut PlayerProfile,
    curve: &XpCurve,
    gained_xp: u32,
    mut on_level_up: F,
) -> LevelUpResult {
    profile.xp = profile.xp.saturating_add(gained_xp);

    let mut levels_gained: u8 = 0;
    while profile.level < u8::MAX {
        let next_level = profile.level.saturating_add(1);
        let next_level_xp = curve.xp_for_level(next_level);
        if profile.xp < next_level_xp {
            break;
        }
        profile.level = next_level;
        levels_gained = levels_gained.saturating_add(1);
        on_level_up(profile, profile.level);
    }

    LevelUpResult { levels_gained }
}
