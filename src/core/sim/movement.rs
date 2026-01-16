//! Movement and range helpers.

use super::types::WeaponProfile;

#[derive(Clone, Copy)]
struct RangeBands {
    band_0: f32,
    band_4: f32,
    band_6: f32,
    band_8: f32,
}

impl RangeBands {
    fn from_array(values: [f32; 4]) -> Self {
        Self {
            band_0: values[0],
            band_4: values[1],
            band_6: values[2],
            band_8: values[3],
        }
    }
}

fn ranged_bands_for_weapon_name(name: &str) -> Option<RangeBands> {
    match name {
        "Shortbow" | "Recurve bow" => Some(RangeBands {
            band_0: 50.0,
            band_4: 80.0,
            band_6: 120.0,
            band_8: 150.0,
        }),
        "Longbow" => Some(RangeBands {
            band_0: 60.0,
            band_4: 120.0,
            band_6: 160.0,
            band_8: 210.0,
        }),
        "Warbow" => Some(RangeBands {
            band_0: 80.0,
            band_4: 160.0,
            band_6: 230.0,
            band_8: 300.0,
        }),
        "Light crossbow" => Some(RangeBands {
            band_0: 60.0,
            band_4: 100.0,
            band_6: 140.0,
            band_8: 180.0,
        }),
        "Heavy crossbow" => Some(RangeBands {
            band_0: 80.0,
            band_4: 140.0,
            band_6: 190.0,
            band_8: 250.0,
        }),
        "Hand crossbow" => Some(RangeBands {
            band_0: 40.0,
            band_4: 70.0,
            band_6: 100.0,
            band_8: 120.0,
        }),
        "Arbalest" => Some(RangeBands {
            band_0: 120.0,
            band_4: 220.0,
            band_6: 320.0,
            band_8: 400.0,
        }),
        "Sling" => Some(RangeBands {
            band_0: 40.0,
            band_4: 80.0,
            band_6: 120.0,
            band_8: 160.0,
        }),
        "Throwing axe" => Some(RangeBands {
            band_0: 20.0,
            band_4: 30.0,
            band_6: 40.0,
            band_8: 60.0,
        }),
        "Throwing knife" => Some(RangeBands {
            band_0: 20.0,
            band_4: 30.0,
            band_6: 40.0,
            band_8: 50.0,
        }),
        "Dart" => Some(RangeBands {
            band_0: 10.0,
            band_4: 20.0,
            band_6: 30.0,
            band_8: 40.0,
        }),
        "Javelin" => Some(RangeBands {
            band_0: 30.0,
            band_4: 50.0,
            band_6: 70.0,
            band_8: 100.0,
        }),
        "Pilum" => Some(RangeBands {
            band_0: 30.0,
            band_4: 40.0,
            band_6: 60.0,
            band_8: 80.0,
        }),
        "Bola" | "Lasso" => Some(RangeBands {
            band_0: 10.0,
            band_4: 20.0,
            band_6: 30.0,
            band_8: 50.0,
        }),
        "Net" => Some(RangeBands {
            band_0: 10.0,
            band_4: 15.0,
            band_6: 0.0,
            band_8: 0.0,
        }),
        _ => None,
    }
}

fn ranged_bands_for_weapon(weapon: &WeaponProfile) -> Option<RangeBands> {
    weapon
        .range_bands_feet
        .map(RangeBands::from_array)
        .or_else(|| ranged_bands_for_weapon_name(&weapon.name))
}

pub(crate) fn range_modifier_for_weapon_with_scale(
    weapon: &WeaponProfile,
    distance: f32,
    scale: f32,
) -> Option<i32> {
    let bands = ranged_bands_for_weapon(weapon)?;
    let max_range = bands
        .band_8
        .max(bands.band_6)
        .max(bands.band_4)
        .max(bands.band_0);
    let scale = if scale > 0.0 { scale } else { 1.0 };
    let effective_distance = distance * scale;
    if max_range > 0.0 && effective_distance > max_range {
        return None;
    }
    if effective_distance <= bands.band_0 {
        Some(0)
    } else if effective_distance <= bands.band_4 {
        Some(-4)
    } else if effective_distance <= bands.band_6 && bands.band_6 > 0.0 {
        Some(-6)
    } else if effective_distance <= bands.band_8 && bands.band_8 > 0.0 {
        Some(-8)
    } else {
        None
    }
}


pub fn max_range_for_bands(bands: [f32; 4]) -> f32 {
    let bands = RangeBands::from_array(bands);
    bands
        .band_8
        .max(bands.band_6)
        .max(bands.band_4)
        .max(bands.band_0)
}

pub fn max_range_for_weapon_name(name: &str) -> Option<f32> {
    ranged_bands_for_weapon_name(name).map(|bands| {
        bands
            .band_8
            .max(bands.band_6)
            .max(bands.band_4)
            .max(bands.band_0)
    })
}

pub fn range_bands_for_weapon_name(name: &str) -> Option<[f32; 4]> {
    ranged_bands_for_weapon_name(name).map(|bands| {
        [bands.band_0, bands.band_4, bands.band_6, bands.band_8]
    })
}

pub(crate) fn max_range_for_weapon(weapon: &WeaponProfile) -> Option<f32> {
    ranged_bands_for_weapon(weapon).map(|bands| {
        bands
            .band_8
            .max(bands.band_6)
            .max(bands.band_4)
            .max(bands.band_0)
    })
}
