//! The browser worker receives complete character settings and returns all metrics.
use super::*;

#[test]
fn worker_character_transport_preserves_presets_and_seeded_results() {
    let mut app = SimGuiApp::new();
    for preset in app.fighter_presets.entries() {
        apply_fighter_preset(
            &mut app.players[0],
            preset,
            &app.weapon_catalog,
            &app.armor_catalog,
            &app.shield_catalog,
            &app.race_catalog,
        );
        // Exercise settings that do not belong to a saved fighter preset.
        app.players[0].environment.temperature_c = -5;
        app.players[0].misc_modifiers.attack_bonus = 3;
        let json = serde_json::to_string(&app.players).unwrap();
        let restored: [PlayerConfig; 2] = serde_json::from_str(&json).unwrap();
        assert!(
            app.players == restored,
            "{} lost character settings",
            preset.name
        );
        let combatants = |players| {
            game_logic::build_combatants(
                players,
                &app.weapon_catalog,
                &app.armor_catalog,
                &app.shield_catalog,
                &app.npc_presets,
                &app.talent_catalog,
            )
        };
        let before = sim::bulk_simulate_batch(
            app.sim.config,
            combatants(&app.players),
            7,
            120,
            u64::MAX - 1,
            0,
            |_| true,
        )
        .unwrap()
        .finish();
        let after = sim::bulk_simulate_batch(
            app.sim.config,
            combatants(&restored),
            7,
            120,
            u64::MAX - 1,
            0,
            |_| true,
        )
        .unwrap()
        .finish();
        let mut merged = sim::BulkSimBatch::default();
        for (offset, runs) in [(0, 2), (2, 3), (5, 2)] {
            let batch = sim::bulk_simulate_batch(
                app.sim.config,
                combatants(&restored),
                runs,
                120,
                u64::MAX - 1,
                offset,
                |_| true,
            )
            .unwrap();
            let encoded = serde_json::to_string(&batch).unwrap();
            merged.merge(serde_json::from_str(&encoded).unwrap());
        }
        let before = serde_json::to_value(before).unwrap();
        let after = serde_json::to_value(after).unwrap();
        assert_eq!(before, after, "{} changed seeded combat", preset.name);
        assert_eq!(
            serde_json::to_value(merged.finish()).unwrap(),
            after,
            "{} changed across worker ranges",
            preset.name
        );
        let restored: BulkSimResult = serde_json::from_value(after.clone()).unwrap();
        assert_eq!(serde_json::to_value(restored).unwrap(), after);
    }
}
