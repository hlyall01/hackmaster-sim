use hackmaster_sim::{data, game_logic};

fn main() {
    let talents = data::load_talents(data::TALENTS_PATH).expect("Failed to load talent catalog");
    let report = game_logic::sim_capability_report(&talents);
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("Failed to serialize capability report")
    );
}
