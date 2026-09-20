pub const START_BP: i32 = 65;
pub const START_LP: i32 = 15;
pub const START_AP: i32 = 15;
pub const START_RP: i32 = 6;
pub const SAVE_VERSION: u32 = 1;
pub const RUN_SAVE_VERSION: u32 = 1;
pub const CHARACTER_SAVE_DIR: &str = "saves/autobattler";
pub const CHARACTER_SAVE_EXTENSION: &str = "json";
pub const RUN_SAVE_DIR: &str = "saves/autobattler_runs";
pub const RUN_SAVE_EXTENSION: &str = "json";
pub const RUN_AUTOSAVE_FILE: &str = "autosave_run.json";
pub const AUTOBATTLER_CONFIG_PATH: &str = "data/autobattler/autobattler_config.json";
pub const NPC_PRESETS_PATH: &str = "data/sim/npc_presets.json";
pub const QUICK_STARTS_PATH: &str = "data/autobattler/autobattler_quick_starts.json";
pub const LOG_DISPLAY_LIMIT: usize = 200;
pub const RUN_PANEL_WIDTH: f32 = 340.0;
pub const SUMMARY_PANEL_WIDTH: f32 = 260.0;
pub const ARENA_PADDING: f32 = 32.0;
pub const WINDOW_WIDTH: f32 = 1100.0;
pub const WINDOW_HEIGHT: f32 = 720.0;
pub const COPY_BYTES_PER_ROW_ALIGNMENT: u32 = 256;

pub const STAT_COUNT: usize = 7;
pub const STAT_LABELS: [&str; STAT_COUNT] = ["STR", "INT", "WIS", "DEX", "CON", "LKS", "CHA"];

pub const TALENT_TAB_ALL: &str = "All";
pub const TALENT_TAB_RACIALS: &str = "Racials";
pub const WEAPON_GROUP_LABELS: [&str; 13] = [
    "Unarmed",
    "Axes",
    "Basic",
    "Blunt",
    "Bows",
    "Crossbows",
    "Double",
    "Ensnaring",
    "Lashes",
    "Large swords",
    "Small swords",
    "Polearms",
    "Spears",
];
