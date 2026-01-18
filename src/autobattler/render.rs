use std::collections::HashMap;

use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::texture::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;

use crate::autobattler::constants::{ARENA_PADDING, RUN_PANEL_WIDTH, SUMMARY_PANEL_WIDTH};
use crate::autobattler::logic;
use crate::autobattler::screenshot::{create_headless_render_target, HeadlessConfig, HeadlessRenderTarget};
use crate::autobattler::state::{AppScreen, AutobattlerState};
use crate::character::WeaponGroup;
use crate::core::types::RaceSpec;
use crate::game_logic::{NpcPresetCatalog, WeaponCatalog};
use crate::sim;

#[derive(Component)]
pub struct CombatantSprite {
    idx: usize,
}

#[derive(Component)]
pub struct WeaponSprite {
    idx: usize,
}

#[derive(Component)]
pub struct ArenaLine;

#[derive(Resource)]
pub struct RenderAssets {
    white_texture: Handle<Image>,
    stick_texture: Handle<Image>,
    race_textures: HashMap<String, Handle<Image>>,
    race_pained_textures: HashMap<String, Handle<Image>>,
    enemy_textures: HashMap<String, Handle<Image>>,
    enemy_pained_textures: HashMap<String, Handle<Image>>,
    weapon_sprites: HashMap<WeaponGroup, WeaponSpriteAsset>,
}

#[derive(Clone)]
struct WeaponSpriteAsset {
    texture: Handle<Image>,
    size: Vec2,
}

#[derive(Resource, Clone, Copy)]
pub struct RenderConfig {
    pub body_art: BodyArt,
    pub weapon_art: WeaponArt,
    pub person_size: Vec2,
    pub person_z: f32,
    pub weapon_z: f32,
    pub line_z: f32,
    pub line_height: f32,
    pub weapon_anchor_y: f32,
    pub line_color: Color,
    pub weapon_color: Color,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            body_art: BodyArt::Pixel,
            weapon_art: WeaponArt::Pixel,
            person_size: Vec2::new(40.0, 60.0),
            person_z: 2.0,
            weapon_z: 3.0,
            line_z: 1.0,
            line_height: 3.0,
            weapon_anchor_y: 0.30,
            line_color: Color::rgb(0.3, 0.32, 0.35),
            weapon_color: Color::rgb(0.82, 0.82, 0.84),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum BodyArt {
    Pixel,
    StickFigure,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum WeaponArt {
    Pixel,
    Block,
}

#[derive(Clone, Copy)]
struct ArenaLayout {
    left: f32,
    right: f32,
    ground_y: f32,
    scale: f32,
    padding_tiles: i32,
    tile_size_ft: f32,
}

#[derive(Clone)]
pub(crate) struct BodyVisual {
    pub texture: Handle<Image>,
    pub size: Vec2,
    pub color: Color,
    pub anchor: Anchor,
    pub ground_offset: f32,
}

#[derive(Clone)]
pub(crate) struct WeaponVisual {
    pub texture: Handle<Image>,
    pub size: Vec2,
    pub color: Color,
    pub rotation_deg: f32,
    pub offset: Vec2,
    pub anchor: Anchor,
    pub show: bool,
}

impl WeaponVisual {
    fn hidden() -> Self {
        Self {
            texture: Handle::default(),
            size: Vec2::ZERO,
            color: Color::NONE,
            rotation_deg: 0.0,
            offset: Vec2::ZERO,
            anchor: Anchor::CenterLeft,
            show: false,
        }
    }
}

fn arena_layout(
    window: &Window,
    config: &sim::SimConfig,
    left_ui_width: f32,
    right_ui_width: f32,
) -> ArenaLayout {
    let padding = ARENA_PADDING;
    let width = window.width().max(padding * 2.0 + 1.0);
    let height = window.height().max(padding * 2.0 + 1.0);
    let mut left = -width * 0.5 + left_ui_width + padding;
    let mut right = width * 0.5 - right_ui_width - padding;
    if right <= left + 40.0 {
        left = -width * 0.5 + padding;
        right = width * 0.5 - padding;
    }
    let ground_y = -height * 0.2;
    let arena_width = (right - left).max(1.0);
    let tile_size_ft = config.tile_size_ft.max(0.01);
    let start_tiles = (config.start_distance / tile_size_ft).ceil() as i32;
    let padding_tiles = ((config.grid_width - 1 - start_tiles) / 2).max(0);
    let scale = arena_width / config.start_distance.max(1.0);
    ArenaLayout {
        left,
        right,
        ground_y,
        scale,
        padding_tiles,
        tile_size_ft,
    }
}

fn make_person_texture() -> Image {
    let width: u32 = 20;
    let height: u32 = 30;
    let mut data = vec![0u8; (width * height * 4) as usize];

    let mut set_px = |x: i32, y: i32| {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as u32, y as u32);
        if x >= width || y >= height {
            return;
        }
        let idx = ((y * width + x) * 4) as usize;
        data[idx] = 255;
        data[idx + 1] = 255;
        data[idx + 2] = 255;
        data[idx + 3] = 255;
    };

    let draw_line = |x0: i32, y0: i32, x1: i32, y1: i32, set_px: &mut dyn FnMut(i32, i32)| {
        let mut x0 = x0;
        let mut y0 = y0;
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            set_px(x0, y0);
            set_px(x0 + 1, y0);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    };

    let center_x = (width / 2) as i32;
    let head_y = 6;
    let head_r = 4;
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let dx = x - center_x;
            let dy = y - head_y;
            if dx * dx + dy * dy <= head_r * head_r {
                set_px(x, y);
            }
        }
    }

    draw_line(center_x, 10, center_x, 20, &mut set_px);
    draw_line(center_x, 14, center_x - 6, 17, &mut set_px);
    draw_line(center_x, 14, center_x + 6, 17, &mut set_px);
    draw_line(center_x, 20, center_x - 4, 28, &mut set_px);
    draw_line(center_x, 20, center_x + 4, 28, &mut set_px);

    Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

pub const WEAPON_GROUPS: [WeaponGroup; 14] = [
    WeaponGroup::Unarmed,
    WeaponGroup::Axes,
    WeaponGroup::Basic,
    WeaponGroup::Blunt,
    WeaponGroup::Bows,
    WeaponGroup::Crossbows,
    WeaponGroup::Double,
    WeaponGroup::Ensnaring,
    WeaponGroup::Lashes,
    WeaponGroup::LargeSwords,
    WeaponGroup::SmallSwords,
    WeaponGroup::Polearms,
    WeaponGroup::Spears,
    WeaponGroup::Shields,
];

fn load_render_assets(images: &mut Assets<Image>, race_catalog: &[RaceSpec]) -> RenderAssets {
    let white_texture = images.add(Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    let stick_texture = images.add(make_person_texture());
    let mut race_textures = HashMap::new();
    let mut race_pained_textures = HashMap::new();
    for race in race_catalog {
        let path = format!("sprites/races/{}.png", race.id);
        if let Some(bytes) = embedded_race_png(&race.id) {
            let image = image_from_png_bytes(bytes);
            race_textures.insert(race.id.clone(), images.add(image));
        } else {
            eprintln!("Missing embedded sprite for race: {} ({path})", race.id);
        }
        if let Some(bytes) = embedded_race_pained_png(&race.id) {
            let image = image_from_png_bytes(bytes);
            race_pained_textures.insert(race.id.clone(), images.add(image));
        }
    }
    let mut enemy_textures = HashMap::new();
    let mut enemy_pained_textures = HashMap::new();
    if let Some(bytes) = embedded_enemy_png("hobgoblin") {
        let image = image_from_png_bytes(bytes);
        enemy_textures.insert("hobgoblin".to_string(), images.add(image));
    } else {
        eprintln!("Missing embedded sprite for enemy: hobgoblin");
    }
    if let Some(bytes) = embedded_enemy_pained_png("hobgoblin") {
        let image = image_from_png_bytes(bytes);
        enemy_pained_textures.insert("hobgoblin".to_string(), images.add(image));
    }
    let mut weapon_sprites = HashMap::new();
    for group in WEAPON_GROUPS {
        if let Some(bytes) = embedded_weapon_png(group) {
            let image = image_from_png_bytes(bytes);
            let size = image.size_f32();
            let texture = images.add(image);
            weapon_sprites.insert(group, WeaponSpriteAsset { texture, size });
        } else {
            eprintln!("Missing embedded weapon sprite for group: {group:?}");
        }
    }
    RenderAssets {
        white_texture,
        stick_texture,
        race_textures,
        race_pained_textures,
        enemy_textures,
        enemy_pained_textures,
        weapon_sprites,
    }
}

fn image_from_png_bytes(bytes: &'static [u8]) -> Image {
    Image::from_buffer(
        bytes,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::nearest(),
        RenderAssetUsages::default(),
    )
    .expect("Failed to decode embedded png")
}

fn embedded_race_png(id: &str) -> Option<&'static [u8]> {
    match id {
        "armeroci" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/armeroci.png"
        ))),
        "fymblwngen" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/fymblwngen.png"
        ))),
        "ithican" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/ithican.png"
        ))),
        "kanian" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/kanian.png"
        ))),
        "katlakehan" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/katlakehan.png"
        ))),
        "midlander" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/midlander.png"
        ))),
        "pather" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/pather.png"
        ))),
        "vetlander" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/vetlander.png"
        ))),
        "limmtrig" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/limmtrig.png"
        ))),
        "vorova_female" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/vorova_female.png"
        ))),
        "vorova_male" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/vorova_male.png"
        ))),
        _ => None,
    }
}

fn embedded_race_pained_png(id: &str) -> Option<&'static [u8]> {
    match id {
        "armeroci" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/armeroci_pained.png"
        ))),
        "fymblwngen" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/fymblwngen_pained.png"
        ))),
        "ithican" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/ithican_pained.png"
        ))),
        "kanian" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/kanian_pained.png"
        ))),
        "katlakehan" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/katlakehan_pained.png"
        ))),
        "midlander" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/midlander_pained.png"
        ))),
        "pather" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/pather_pained.png"
        ))),
        "vetlander" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/vetlander_pained.png"
        ))),
        "limmtrig" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/limmtrig_pained.png"
        ))),
        "vorova_female" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/vorova_female_pained.png"
        ))),
        "vorova_male" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/races/vorova_male_pained.png"
        ))),
        _ => None,
    }
}

fn embedded_enemy_png(id: &str) -> Option<&'static [u8]> {
    match id {
        "hobgoblin" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/enemies/hobgoblin.png"
        ))),
        _ => None,
    }
}

fn embedded_enemy_pained_png(id: &str) -> Option<&'static [u8]> {
    match id {
        "hobgoblin" => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/enemies/hobgoblin_pained.png"
        ))),
        _ => None,
    }
}

fn embedded_weapon_png(group: WeaponGroup) -> Option<&'static [u8]> {
    match group {
        WeaponGroup::Unarmed => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/weapons/unarmed.png"
        ))),
        WeaponGroup::Axes => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/weapons/axes.png"
        ))),
        WeaponGroup::Basic => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/weapons/basic.png"
        ))),
        WeaponGroup::Blunt => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/weapons/blunt.png"
        ))),
        WeaponGroup::Bows => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/weapons/bows.png"
        ))),
        WeaponGroup::Crossbows => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/weapons/crossbows.png"
        ))),
        WeaponGroup::Double => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/weapons/double.png"
        ))),
        WeaponGroup::Ensnaring => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/weapons/ensnaring.png"
        ))),
        WeaponGroup::Lashes => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/weapons/lashes.png"
        ))),
        WeaponGroup::LargeSwords => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/weapons/large_swords.png"
        ))),
        WeaponGroup::SmallSwords => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/weapons/small_swords.png"
        ))),
        WeaponGroup::Polearms => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/weapons/polearms.png"
        ))),
        WeaponGroup::Spears => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/weapons/spears.png"
        ))),
        WeaponGroup::Shields => Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/sprites/weapons/shields.png"
        ))),
    }
}

#[derive(Clone, Copy)]
struct WeaponStyle {
    length: f32,
    thickness: f32,
    rotation_deg: f32,
    offset: Vec2,
    show: bool,
}

#[derive(Clone, Copy)]
struct WeaponSpriteStyle {
    rotation_deg: f32,
    offset: Vec2,
    anchor: Anchor,
}

impl WeaponStyle {
    fn hidden() -> Self {
        Self {
            length: 0.0,
            thickness: 0.0,
            rotation_deg: 0.0,
            offset: Vec2::ZERO,
            show: false,
        }
    }
}

fn weapon_group_for_name(name: &str, catalog: &WeaponCatalog) -> Option<WeaponGroup> {
    catalog
        .entries()
        .iter()
        .find(|entry| entry.name.eq_ignore_ascii_case(name))
        .map(|entry| entry.group)
}

fn weapon_group_for_profile(weapon: &sim::WeaponProfile, catalog: &WeaponCatalog) -> WeaponGroup {
    weapon_group_for_name(&weapon.name, catalog).unwrap_or_else(|| {
        if weapon.uses_projectiles {
            WeaponGroup::Bows
        } else if weapon.reach_ft >= 6.0 {
            WeaponGroup::Spears
        } else if weapon.is_small_weapon {
            WeaponGroup::SmallSwords
        } else {
            WeaponGroup::Basic
        }
    })
}

fn weapon_style_for(combatant: &sim::Combatant, catalog: &WeaponCatalog) -> WeaponStyle {
    let weapon = combatant.sheet.offense.weapon.as_ref();
    if !weapon.has_weapon || weapon.is_unarmed {
        return WeaponStyle::hidden();
    }

    let group = weapon_group_for_name(&weapon.name, catalog);
    match group {
        Some(WeaponGroup::Unarmed) => WeaponStyle::hidden(),
        Some(WeaponGroup::Bows) => WeaponStyle {
            length: 18.0,
            thickness: 3.0,
            rotation_deg: 0.0,
            offset: Vec2::new(6.0, 8.0),
            show: true,
        },
        Some(WeaponGroup::Crossbows) => WeaponStyle {
            length: 18.0,
            thickness: 3.0,
            rotation_deg: 26.0,
            offset: Vec2::new(6.0, -2.0),
            show: true,
        },
        Some(WeaponGroup::Spears) | Some(WeaponGroup::Polearms) => WeaponStyle {
            length: 26.0,
            thickness: 3.0,
            rotation_deg: 28.0,
            offset: Vec2::new(8.0, -1.0),
            show: true,
        },
        Some(WeaponGroup::Axes) => WeaponStyle {
            length: 16.0,
            thickness: 4.0,
            rotation_deg: 32.0,
            offset: Vec2::new(6.0, -1.0),
            show: true,
        },
        Some(WeaponGroup::Blunt) => WeaponStyle {
            length: 14.0,
            thickness: 4.0,
            rotation_deg: 30.0,
            offset: Vec2::new(6.0, -1.0),
            show: true,
        },
        Some(WeaponGroup::Lashes) | Some(WeaponGroup::Ensnaring) => WeaponStyle {
            length: 20.0,
            thickness: 2.0,
            rotation_deg: 34.0,
            offset: Vec2::new(7.0, -1.0),
            show: true,
        },
        Some(WeaponGroup::SmallSwords) => WeaponStyle {
            length: 12.0,
            thickness: 3.0,
            rotation_deg: 32.0,
            offset: Vec2::new(6.0, -2.0),
            show: true,
        },
        Some(WeaponGroup::LargeSwords)
        | Some(WeaponGroup::Basic)
        | Some(WeaponGroup::Double)
        | Some(WeaponGroup::Shields) => WeaponStyle {
            length: 18.0,
            thickness: 3.0,
            rotation_deg: 32.0,
            offset: Vec2::new(7.0, -1.0),
            show: true,
        },
        _ => {
            if weapon.uses_projectiles {
                WeaponStyle {
                    length: 16.0,
                    thickness: 3.0,
                    rotation_deg: 30.0,
                    offset: Vec2::new(6.0, -2.0),
                    show: true,
                }
            } else if weapon.reach_ft >= 6.0 {
                WeaponStyle {
                    length: 24.0,
                    thickness: 3.0,
                    rotation_deg: 28.0,
                    offset: Vec2::new(8.0, -1.0),
                    show: true,
                }
            } else {
                WeaponStyle {
                    length: 14.0,
                    thickness: 3.0,
                    rotation_deg: 30.0,
                    offset: Vec2::new(6.0, -2.0),
                    show: true,
                }
            }
        }
    }
}

fn weapon_sprite_style_for_group(group: WeaponGroup) -> WeaponSpriteStyle {
    match group {
        WeaponGroup::Bows => WeaponSpriteStyle {
            rotation_deg: 0.0,
            offset: Vec2::new(6.0, 8.0),
            anchor: Anchor::CenterLeft,
        },
        WeaponGroup::Crossbows => WeaponSpriteStyle {
            rotation_deg: 26.0,
            offset: Vec2::new(6.0, -2.0),
            anchor: Anchor::CenterLeft,
        },
        WeaponGroup::Polearms | WeaponGroup::Spears => WeaponSpriteStyle {
            rotation_deg: 28.0,
            offset: Vec2::new(8.0, -1.0),
            anchor: Anchor::CenterLeft,
        },
        WeaponGroup::Axes => WeaponSpriteStyle {
            rotation_deg: 32.0,
            offset: Vec2::new(6.0, -1.0),
            anchor: Anchor::CenterLeft,
        },
        WeaponGroup::Blunt => WeaponSpriteStyle {
            rotation_deg: 30.0,
            offset: Vec2::new(6.0, -1.0),
            anchor: Anchor::CenterLeft,
        },
        WeaponGroup::Lashes => WeaponSpriteStyle {
            rotation_deg: 34.0,
            offset: Vec2::new(7.0, -1.0),
            anchor: Anchor::CenterLeft,
        },
        WeaponGroup::Ensnaring => WeaponSpriteStyle {
            rotation_deg: 30.0,
            offset: Vec2::new(7.0, -1.0),
            anchor: Anchor::CenterLeft,
        },
        WeaponGroup::SmallSwords => WeaponSpriteStyle {
            rotation_deg: 32.0,
            offset: Vec2::new(6.0, -2.0),
            anchor: Anchor::CenterLeft,
        },
        WeaponGroup::LargeSwords | WeaponGroup::Basic | WeaponGroup::Double => WeaponSpriteStyle {
            rotation_deg: 32.0,
            offset: Vec2::new(7.0, -1.0),
            anchor: Anchor::CenterLeft,
        },
        WeaponGroup::Shields => WeaponSpriteStyle {
            rotation_deg: 18.0,
            offset: Vec2::new(5.0, -1.0),
            anchor: Anchor::CenterLeft,
        },
        WeaponGroup::Unarmed => WeaponSpriteStyle {
            rotation_deg: 0.0,
            offset: Vec2::new(0.0, 0.0),
            anchor: Anchor::CenterLeft,
        },
    }
}

fn fallback_weapon_sprite(assets: &RenderAssets) -> WeaponSpriteAsset {
    assets
        .weapon_sprites
        .get(&WeaponGroup::Basic)
        .cloned()
        .unwrap_or(WeaponSpriteAsset {
            texture: assets.white_texture.clone(),
            size: Vec2::new(24.0, 8.0),
        })
}

fn fallback_body_texture(assets: &RenderAssets, downed: bool) -> Handle<Image> {
    let texture = assets
        .race_textures
        .get("midlander")
        .cloned()
        .unwrap_or_else(|| assets.stick_texture.clone());
    if downed {
        assets
            .race_pained_textures
            .get("midlander")
            .cloned()
            .unwrap_or(texture)
    } else {
        texture
    }
}

fn body_visual_from_texture(
    texture: Handle<Image>,
    render_config: &RenderConfig,
    downed: bool,
    color: Color,
) -> BodyVisual {
    let mut size = render_config.person_size;
    if downed {
        size = Vec2::new(size.x * 1.5, size.y * 0.65);
    }
    let ground_offset = if downed { size.y * 0.4 } else { size.y * 0.5 };
    BodyVisual {
        texture,
        size,
        color,
        anchor: Anchor::Center,
        ground_offset,
    }
}

fn body_visual_for(
    combatant: &sim::Combatant,
    assets: &RenderAssets,
    render_config: &RenderConfig,
    player_race_id: Option<&str>,
    enemy_sprite_key: Option<&str>,
    downed: bool,
) -> BodyVisual {
    match render_config.body_art {
        BodyArt::Pixel => {
            let (normal, pained, key) = if combatant.team_id == 0 {
                (
                    &assets.race_textures,
                    &assets.race_pained_textures,
                    player_race_id,
                )
            } else {
                (
                    &assets.enemy_textures,
                    &assets.enemy_pained_textures,
                    enemy_sprite_key,
                )
            };
            let texture = if downed {
                key.and_then(|id| pained.get(id))
                    .or_else(|| key.and_then(|id| normal.get(id)))
                    .cloned()
            } else {
                key.and_then(|id| normal.get(id)).cloned()
            }
            .unwrap_or_else(|| fallback_body_texture(assets, downed));
            body_visual_from_texture(texture, render_config, downed, Color::WHITE)
        }
        BodyArt::StickFigure => body_visual_from_texture(
            assets.stick_texture.clone(),
            render_config,
            downed,
            team_color(combatant.team_id),
        ),
    }
}

pub(crate) fn body_visual_for_race_id(
    race_id: &str,
    assets: &RenderAssets,
    render_config: &RenderConfig,
    downed: bool,
) -> BodyVisual {
    match render_config.body_art {
        BodyArt::Pixel => {
            let (normal_map, pained_map) = if assets.race_textures.contains_key(race_id)
                || assets.race_pained_textures.contains_key(race_id)
            {
                (&assets.race_textures, &assets.race_pained_textures)
            } else {
                (&assets.enemy_textures, &assets.enemy_pained_textures)
            };
            let texture = if downed {
                pained_map
                    .get(race_id)
                    .or_else(|| normal_map.get(race_id))
            } else {
                normal_map.get(race_id)
            }
            .cloned()
            .unwrap_or_else(|| fallback_body_texture(assets, downed));
            body_visual_from_texture(texture, render_config, downed, Color::WHITE)
        }
        BodyArt::StickFigure => body_visual_from_texture(
            assets.stick_texture.clone(),
            render_config,
            downed,
            Color::WHITE,
        ),
    }
}

fn weapon_visual_for(
    combatant: &sim::Combatant,
    assets: &RenderAssets,
    render_config: &RenderConfig,
    catalog: &WeaponCatalog,
    downed: bool,
) -> WeaponVisual {
    if downed {
        return WeaponVisual::hidden();
    }
    let style = weapon_style_for(combatant, catalog);
    if !style.show {
        return WeaponVisual::hidden();
    }
    let weapon = combatant.sheet.offense.weapon.as_ref();
    match render_config.weapon_art {
        WeaponArt::Pixel => {
            let group = weapon_group_for_profile(weapon, catalog);
            weapon_visual_for_group(group, assets, render_config)
        }
        WeaponArt::Block => WeaponVisual {
            texture: assets.white_texture.clone(),
            size: Vec2::new(style.length, style.thickness),
            color: render_config.weapon_color,
            rotation_deg: style.rotation_deg,
            offset: style.offset,
            anchor: Anchor::CenterLeft,
            show: true,
        },
    }
}

pub(crate) fn weapon_visual_for_group(
    group: WeaponGroup,
    assets: &RenderAssets,
    render_config: &RenderConfig,
) -> WeaponVisual {
    match render_config.weapon_art {
        WeaponArt::Pixel => {
            let sprite = assets
                .weapon_sprites
                .get(&group)
                .cloned()
                .unwrap_or_else(|| fallback_weapon_sprite(assets));
            let sprite_style = weapon_sprite_style_for_group(group);
            WeaponVisual {
                texture: sprite.texture,
                size: sprite.size,
                color: Color::WHITE,
                rotation_deg: sprite_style.rotation_deg,
                offset: sprite_style.offset,
                anchor: sprite_style.anchor,
                show: true,
            }
        }
        WeaponArt::Block => WeaponVisual {
            texture: assets.white_texture.clone(),
            size: Vec2::new(18.0, 3.0),
            color: render_config.weapon_color,
            rotation_deg: 30.0,
            offset: Vec2::new(6.0, -2.0),
            anchor: Anchor::CenterLeft,
            show: true,
        },
    }
}

pub fn setup_render_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    state: Res<AutobattlerState>,
    headless: Option<Res<HeadlessConfig>>,
) {
    let render_config = RenderConfig::default();
    let assets = load_render_assets(&mut images, &state.app.race_catalog);
    let line_texture = assets.white_texture.clone();
    commands.insert_resource(render_config);
    commands.insert_resource(assets);
    let headless_config = headless.map(|config| *config);
    let camera_bundle = if let Some(config) = headless_config {
        let target = create_headless_render_target(&mut images, config);
        let mut camera = Camera2dBundle::default();
        camera.camera.target = RenderTarget::Image(target.image.clone());
        commands.insert_resource(target);
        camera
    } else {
        Camera2dBundle::default()
    };
    commands.spawn(camera_bundle);

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: render_config.line_color,
                custom_size: Some(Vec2::new(800.0, render_config.line_height)),
                ..Default::default()
            },
            texture: line_texture,
            transform: Transform::from_translation(Vec3::new(0.0, -120.0, render_config.line_z)),
            ..Default::default()
        },
        ArenaLine,
    ));
}

pub fn sync_render_system(
    mut commands: Commands,
    state: Res<AutobattlerState>,
    assets: Res<RenderAssets>,
    render_config: Res<RenderConfig>,
    windows: Query<&Window, With<PrimaryWindow>>,
    headless: Option<Res<HeadlessRenderTarget>>,
    mut queries: ParamSet<(
        Query<(&mut Transform, &mut Sprite), With<ArenaLine>>,
        Query<
            (
                Entity,
                &CombatantSprite,
                &mut Transform,
                &mut Sprite,
                &mut Handle<Image>,
            ),
            Without<ArenaLine>,
        >,
        Query<
            (
                Entity,
                &WeaponSprite,
                &mut Transform,
                &mut Sprite,
                &mut Handle<Image>,
            ),
            Without<ArenaLine>,
        >,
    )>,
) {
    if matches!(state.app.screen, AppScreen::SpriteReview) {
        if let Ok((_, mut sprite)) = queries.p0().get_single_mut() {
            sprite.color = Color::NONE;
        }
        return;
    }
    let Ok(window) = windows.get_single() else {
        return;
    };
    let headless = headless.is_some();
    let live = state
        .app
        .run_state
        .as_ref()
        .and_then(|view| view.live_fight.as_ref());
    let Some(live) = live else {
        for (entity, _, _, _, _) in queries.p1().iter_mut() {
            commands.entity(entity).despawn_recursive();
        }
        for (entity, _, _, _, _) in queries.p2().iter_mut() {
            commands.entity(entity).despawn_recursive();
        }
        return;
    };
    let sim = &live.sim;
    let player_race_id = state.app.creation.player.race_id.as_deref();
    let enemy_key = enemy_sprite_key(live, &state.app.npc_presets);
    let left_ui = if headless {
        0.0
    } else if matches!(state.app.screen, AppScreen::Run) {
        RUN_PANEL_WIDTH
    } else {
        0.0
    };
    let right_ui = if headless { 0.0 } else { SUMMARY_PANEL_WIDTH };
    let layout = arena_layout(window, &state.app.sim_config, left_ui, right_ui);
    if let Ok((mut transform, mut sprite)) = queries.p0().get_single_mut() {
        sprite.color = render_config.line_color;
        let width = (layout.right - layout.left).max(1.0);
        transform.translation = Vec3::new(
            (layout.left + layout.right) * 0.5,
            layout.ground_y,
            render_config.line_z,
        );
        sprite.custom_size = Some(Vec2::new(width, render_config.line_height));
    }
    let mut existing: HashMap<usize, Entity> = HashMap::new();
    for (entity, sprite, _, _, _) in queries.p1().iter_mut() {
        existing.insert(sprite.idx, entity);
    }
    let mut existing_weapons: HashMap<usize, Entity> = HashMap::new();
    for (entity, sprite, _, _, _) in queries.p2().iter_mut() {
        existing_weapons.insert(sprite.idx, entity);
    }
    for idx in 0..sim.actors.len() {
        if !existing.contains_key(&idx) {
            let combatant = sim.combatants.get(idx);
            let visual = combatant
                .map(|combatant| {
                    let downed = combatant.state.hp <= 0
                        || combatant.state.trauma_remaining_seconds > 0;
                    body_visual_for(
                        combatant,
                        &assets,
                        &render_config,
                        player_race_id,
                        enemy_key,
                        downed,
                    )
                })
                .unwrap_or_else(|| {
                    body_visual_from_texture(
                        fallback_body_texture(&assets, false),
                        &render_config,
                        false,
                        Color::WHITE,
                    )
                });
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: visual.color,
                        custom_size: Some(visual.size),
                        anchor: visual.anchor,
                        ..Default::default()
                    },
                    texture: visual.texture,
                    transform: Transform::from_translation(Vec3::new(
                        0.0,
                        layout.ground_y + visual.ground_offset,
                        render_config.person_z,
                    )),
                    ..Default::default()
                },
                CombatantSprite { idx },
            ));
        }
        if let Some(combatant) = sim.combatants.get(idx) {
            let downed = combatant.state.hp <= 0 || combatant.state.trauma_remaining_seconds > 0;
            let body_visual = body_visual_for(
                combatant,
                &assets,
                &render_config,
                player_race_id,
                enemy_key,
                downed,
            );
            let visual = weapon_visual_for(
                combatant,
                &assets,
                &render_config,
                &state.app.weapon_catalog,
                downed,
            );
            if visual.show && !existing_weapons.contains_key(&idx) {
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: visual.color,
                            custom_size: Some(visual.size),
                            anchor: visual.anchor,
                            ..Default::default()
                        },
                        texture: visual.texture,
                        transform: Transform::from_translation(Vec3::new(
                            0.0,
                            layout.ground_y + body_visual.size.y * render_config.weapon_anchor_y,
                            render_config.weapon_z,
                        )),
                        ..Default::default()
                    },
                    WeaponSprite { idx },
                ));
            } else if !visual.show {
                if let Some(entity) = existing_weapons.remove(&idx) {
                    commands.entity(entity).despawn_recursive();
                }
            }
        }
    }
    for (entity, sprite, mut transform, mut sprite_bundle, mut texture) in queries.p1().iter_mut() {
        let Some(actor) = sim.actors.get(sprite.idx) else {
            commands.entity(entity).despawn_recursive();
            continue;
        };
        let x_ft = (actor.position.x - layout.padding_tiles) as f32 * layout.tile_size_ft;
        let x = (layout.left + x_ft * layout.scale).clamp(layout.left, layout.right);
        let visual = sim
            .combatants
            .get(sprite.idx)
            .map(|combatant| {
                let downed = combatant.state.hp <= 0
                    || combatant.state.trauma_remaining_seconds > 0;
                body_visual_for(
                    combatant,
                    &assets,
                    &render_config,
                    player_race_id,
                    enemy_key,
                    downed,
                )
            })
            .unwrap_or_else(|| {
                body_visual_from_texture(
                    fallback_body_texture(&assets, false),
                    &render_config,
                    false,
                    Color::WHITE,
                )
            });
        transform.translation = Vec3::new(
            x,
            layout.ground_y + visual.ground_offset,
            render_config.person_z,
        );
        sprite_bundle.flip_x = facing_for(sprite.idx, sim).unwrap_or(1.0) < 0.0;
        sprite_bundle.color = visual.color;
        sprite_bundle.custom_size = Some(visual.size);
        sprite_bundle.anchor = visual.anchor;
        *texture = visual.texture;
    }

    for (entity, sprite, mut transform, mut sprite_bundle, mut texture) in queries.p2().iter_mut() {
        let Some(actor) = sim.actors.get(sprite.idx) else {
            commands.entity(entity).despawn_recursive();
            continue;
        };
        let Some(combatant) = sim.combatants.get(sprite.idx) else {
            commands.entity(entity).despawn_recursive();
            continue;
        };
        let downed = combatant.state.hp <= 0 || combatant.state.trauma_remaining_seconds > 0;
        let body_visual = body_visual_for(
            combatant,
            &assets,
            &render_config,
            player_race_id,
            enemy_key,
            downed,
        );
        let visual = weapon_visual_for(
            combatant,
            &assets,
            &render_config,
            &state.app.weapon_catalog,
            downed,
        );
        if !visual.show {
            commands.entity(entity).despawn_recursive();
            continue;
        }
        let facing = facing_for(sprite.idx, sim).unwrap_or(1.0);
        let x_ft = (actor.position.x - layout.padding_tiles) as f32 * layout.tile_size_ft;
        let x = (layout.left + x_ft * layout.scale).clamp(layout.left, layout.right);
        let base_y = layout.ground_y + body_visual.size.y * render_config.weapon_anchor_y;
        let offset = Vec2::new(visual.offset.x * facing, visual.offset.y);
        transform.translation = Vec3::new(x + offset.x, base_y + offset.y, render_config.weapon_z);
        transform.rotation =
            Quat::from_rotation_z(visual.rotation_deg.to_radians() * facing.signum());
        sprite_bundle.custom_size = Some(visual.size);
        sprite_bundle.color = visual.color;
        sprite_bundle.anchor = anchor_for_facing(visual.anchor, facing);
        *texture = visual.texture;
        sprite_bundle.flip_x = facing < 0.0;
    }
}

fn enemy_sprite_key(
    live: &crate::autobattler::state::LiveFight,
    npc_presets: &NpcPresetCatalog,
) -> Option<&'static str> {
    npc_presets
        .get(live.enemy.preset_id)
        .and_then(|preset| enemy_sprite_id(&preset.name))
}

fn enemy_sprite_id(name: &str) -> Option<&'static str> {
    if logic::hobgoblin_level(name).is_some() {
        Some("hobgoblin")
    } else {
        None
    }
}

fn anchor_for_facing(anchor: Anchor, facing: f32) -> Anchor {
    if facing < 0.0 {
        mirror_anchor(anchor)
    } else {
        anchor
    }
}

fn mirror_anchor(anchor: Anchor) -> Anchor {
    match anchor {
        Anchor::CenterLeft => Anchor::CenterRight,
        Anchor::CenterRight => Anchor::CenterLeft,
        Anchor::TopLeft => Anchor::TopRight,
        Anchor::TopRight => Anchor::TopLeft,
        Anchor::BottomLeft => Anchor::BottomRight,
        Anchor::BottomRight => Anchor::BottomLeft,
        _ => anchor,
    }
}

fn facing_for(idx: usize, sim: &crate::core::sim::SimState) -> Option<f32> {
    let actor = sim.actors.get(idx)?;
    let team_id = sim.combatants.get(idx)?.team_id;
    let mut best: Option<(i32, i32)> = None;
    for (other_idx, other_actor) in sim.actors.iter().enumerate() {
        if other_idx == idx {
            continue;
        }
        if sim.combatants.get(other_idx).map(|c| c.team_id) == Some(team_id) {
            continue;
        }
        let dx = other_actor.position.x - actor.position.x;
        let dist = dx.abs();
        match best {
            None => best = Some((dist, dx)),
            Some((best_dist, _)) => {
                if dist < best_dist {
                    best = Some((dist, dx));
                }
            }
        }
    }
    let dx = best.map(|(_, dx)| dx).unwrap_or(1);
    if dx == 0 {
        Some(1.0)
    } else {
        Some(dx.signum() as f32)
    }
}

fn team_color(team_id: u8) -> Color {
    match team_id {
        0 => Color::rgb(0.88, 0.35, 0.22),
        1 => Color::rgb(0.20, 0.55, 0.82),
        _ => Color::rgb(0.68, 0.72, 0.24),
    }
}
