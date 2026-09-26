//! Render every simulator UI section without requiring a native window.
use super::*;

#[test]
fn wren_power_attack_refreshes_damage_after_a_previous_attack() {
    use game_logic::roll_macros::{MacroSession, Modifier, RollKind};
    let mut app = SimGuiApp::new();
    let preset = app
        .fighter_presets
        .entries()
        .iter()
        .find(|p| p.name == "Wren")
        .unwrap()
        .clone();
    apply_fighter_preset(
        &mut app.players[0],
        &preset,
        &app.weapon_catalog,
        &app.armor_catalog,
        &app.shield_catalog,
        &app.race_catalog,
    );
    for mounted in [false, true] {
        let mut player = app.players[0].clone();
        player.mounted = mounted;
        player.mounted_combat.trot_or_faster = mounted;
        let build = |session: &MacroSession| {
            game_logic::build_combatant(
                &session.state.profile_config(&player),
                &app.weapon_catalog,
                &app.armor_catalog,
                &app.shield_catalog,
                &app.npc_presets,
                &app.talent_catalog,
            )
        };
        let mut session = MacroSession::default();
        let normal = build(&session);
        let normal_rolls = session.state.preview(&normal, &normal).unwrap();
        session.copy(RollKind::Attack, &normal, &normal).unwrap();
        session.adjust(Modifier::PowerAttack, true);
        let powered = build(&session);
        let rolls = session.state.preview(&powered, &normal).unwrap();
        assert!(rolls.attack_bonus < normal_rolls.attack_bonus);
        assert!(rolls.damage.contains("Power attack"));
        assert!(powered.sheet.offense.strength_damage > normal.sheet.offense.strength_damage);
        assert!(
            rolls
                .damage
                .contains(&format!("{:+}]]", powered.sheet.offense.strength_damage))
        );
        eprintln!(
            "Wren mounted={mounted}: {} -> {}",
            normal_rolls.damage, rolls.damage
        );

        // Removing the selection updates Damage immediately as well.
        session.adjust(Modifier::PowerAttack, false);
        assert_eq!(
            session.state.preview(&normal, &normal).unwrap().damage,
            normal_rolls.damage
        );
        session.adjust(Modifier::PowerAttack, true);
        session.copy(RollKind::Attack, &powered, &normal).unwrap();
        assert_eq!(
            session.copy(RollKind::Damage, &powered, &normal).unwrap(),
            rolls.damage
        );
        assert!(session.state.power_attack);
        session.adjust(Modifier::PowerAttack, false);
        session.copy(RollKind::Attack, &normal, &normal).unwrap();
        assert_eq!(
            session.copy(RollKind::Damage, &normal, &normal).unwrap(),
            normal_rolls.damage
        );
    }
}

#[test]
fn all_gui_sections_render_and_tessellate() {
    let mut app = SimGuiApp::new();
    let ctx = egui::Context::default();
    for view in 0..7 + PLAYER_EDITOR_TABS.len() {
        app.show_player_editor = [false, false];
        if view < 4 {
            app.active_tab = [MainTab::Simulator, MainTab::DetailedStats, MainTab::Tools, MainTab::FeatureRequests][view];
        } else if view < 7 {
            app.active_tab = MainTab::Tools;
            app.active_tool_tab = [
                ToolTab::WoundHealing,
                ToolTab::EssenceWounds,
                ToolTab::EgoGeneration,
            ][view - 4];
        } else {
            app.active_tab = MainTab::Simulator;
            app.show_player_editor = [true, false];
            app.player_editor_tabs[0] = PLAYER_EDITOR_TABS[view - 7];
        }
        let mut output = None;
        for _ in 0..3 {
            output = Some(ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 900.0))),
                    time: Some(view as f64),
                    ..Default::default()
                },
                |ctx| app.show(ctx),
            ));
        }
        let output = output.unwrap();
        assert!(!output.shapes.is_empty());
        let mut texts = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
                _ => None,
            })
            .collect::<Vec<_>>();
        texts.sort();
        assert!(!texts.is_empty(), "view {view} should render text");
        assert!(
            !ctx.tessellate(output.shapes, output.pixels_per_point)
                .is_empty()
        );
    }
}

#[test]
fn macro_ctrl_scroll_zooms_locally_and_stacks_rolls_in_narrow_windows() {
    let app = SimGuiApp::new();
    let mut panel = macros::MacroPanel::default();
    let ctx = egui::Context::default();
    let base_font = ctx.style().text_styles[&egui::TextStyle::Body].size;
    let draw = |panel: &mut macros::MacroPanel, width, ctrl, events| {
        ctx.run(egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(width, 1500.0))),
            events, modifiers: egui::Modifiers { ctrl, ..Default::default() }, ..Default::default()
        }, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    panel.show(ui, &app.players[0], &app.players[1], &app.weapon_catalog,
                        &app.armor_catalog, &app.shield_catalog, &app.npc_presets, &app.talent_catalog);
                });
            });
        })
    };
    let find = |output: &egui::FullOutput, text: &str| {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(t) if t.galley.text() == text || t.galley.text().starts_with(&format!("{text}\n[[")) =>
                Some(Rect::from_min_size(t.pos, t.galley.size())),
            _ => None,
        }).unwrap_or_else(|| panic!("Missing macro text: {text}"))
    };
    let mut output = draw(&mut panel, 1100.0, false, vec![]);
    let original_size = find(&output, "Weapon style").height();
    assert!(original_size < 20.0);
    let pointer = egui::Event::PointerMoved(egui::pos2(200.0, 200.0));
    // The native egui adapter turns Ctrl+wheel into Zoom events.
    output = draw(&mut panel, 1100.0, true, vec![pointer.clone(), egui::Event::Zoom(2.0)]);
    assert!(find(&output, "Weapon style").height() > original_size);
    let enlarged = find(&output, "Weapon style").height();
    output = draw(&mut panel, 1100.0, false, vec![pointer.clone(), egui::Event::Scroll(egui::vec2(0.0, -20.0))]);
    assert_eq!(find(&output, "Weapon style").height(), enlarged);
    output = draw(&mut panel, 1100.0, true, vec![pointer.clone(), egui::Event::Zoom(0.5)]);
    assert_eq!(find(&output, "Weapon style").height(), original_size);
    output = draw(&mut panel, 1100.0, true, vec![pointer.clone(), egui::Event::Zoom(0.75)]);
    assert!(find(&output, "Weapon style").height() < original_size);
    draw(&mut panel, 1100.0, true, vec![pointer, egui::Event::Zoom(2.0 / 0.75)]);
    for _ in 0..3 { output = draw(&mut panel, 450.0, false, vec![]); }
    let attack = find(&output, "Attack");
    let defence = find(&output, "Defence");
    let damage = find(&output, "Damage");
    assert!(attack.bottom() < defence.top() && defence.bottom() < damage.top());
    for rect in [attack, defence, damage] {
        assert!(rect.left() >= 0.0 && rect.right() <= 450.0, "{rect:?}");
    }
    assert_eq!(ctx.style().text_styles[&egui::TextStyle::Body].size, base_font);
}

#[test]
fn macro_tab_mouse_buttons_and_clipboard_work_without_editing_character() {
    let mut app = SimGuiApp::new();
    let original = app.players.clone();
    app.show_player_editor = [true, false];
    app.player_editor_tabs[0] = PlayerEditorTab::Macros;
    let ctx = egui::Context::default();
    let draw = |app: &mut SimGuiApp, events| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 1000.0))),
                events,
                ..Default::default()
            },
            |ctx| app.show(ctx),
        )
    };
    let mut output = draw(&mut app, vec![]);
    for _ in 0..3 {
        output = draw(&mut app, vec![]);
    }
    let find = |output: &egui::FullOutput, needle: &str| {
        output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text().contains(needle) => {
                    Some(t.pos + t.galley.size() * 0.5)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("Missing macro control: {needle}"))
    };
    let click = |app: &mut SimGuiApp, pos, button| {
        draw(
            app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        draw(
            app,
            vec![egui::Event::PointerButton {
                pos,
                button,
                pressed: false,
                modifiers: Default::default(),
            }],
        )
    };
    let ground = find(&output, "+5 DEF / −1 ATK");
    output = click(&mut app, ground, egui::PointerButton::Primary);
    assert_eq!(app.macro_panels[0].session.state.ground, 1);
    let ground = find(&output, "+5 DEF / −1 ATK");
    output = click(&mut app, ground, egui::PointerButton::Secondary);
    assert_eq!(app.macro_panels[0].session.state.ground, 0);
    let ground = find(&output, "+5 DEF / −1 ATK");
    click(&mut app, ground, egui::PointerButton::Primary);
    output = draw(&mut app, vec![]);
    let attack = find(&output, "Attack — Give Ground ×1\n[[");
    output = click(&mut app, attack, egui::PointerButton::Primary);
    assert!(output.platform_output.copied_text.starts_with("[[1d20!p"));
    assert!(
        output
            .platform_output
            .copied_text
            .contains("Give Ground ×1")
    );
    assert_eq!(app.macro_panels[0].session.state.ground, 0);
    assert_eq!(app.macro_panels[1].session.state.ground, 0);
    assert!(app.players == original);

    // The new control follows the actual character and responds to both buttons.
    let power_text = "×2 STR damage / no +INT, +DEX ATK";
    app.players[0].talents.retain(|t| t.id != "power_attack");
    draw(&mut app, vec![]);
    output = draw(&mut app, vec![]);
    assert!(
        !output.shapes.iter().any(
            |s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text().contains(power_text))
        )
    );
    app.players[0].talents.push(TalentSelection {
        id: "power_attack".into(),
        rank: 1,
        weapon: None,
    });
    app.players[0].strength_base = 18;
    app.players[0].weapon_id = app
        .weapon_catalog
        .entries()
        .iter()
        .position(|w| w.name == "Halberd")
        .and_then(|i| app.weapon_catalog.id_from_index(i))
        .unwrap();
    draw(&mut app, vec![]);
    output = draw(&mut app, vec![]);
    let power = find(&output, power_text);
    click(&mut app, power, egui::PointerButton::Primary);
    assert!(app.macro_panels[0].session.state.power_attack);
    output = draw(&mut app, vec![]);
    let damage = find(&output, "Damage — Power attack\n[[");
    let copied = click(&mut app, damage, egui::PointerButton::Primary);
    assert!(
        copied
            .platform_output
            .copied_text
            .contains("Damage — Power attack")
    );
    assert!(app.macro_panels[0].session.state.power_attack);
    output = draw(&mut app, vec![]);
    let power = find(&output, power_text);
    click(&mut app, power, egui::PointerButton::Primary);
    assert!(!app.macro_panels[0].session.state.power_attack);
    output = draw(&mut app, vec![]);
    let power = find(&output, power_text);
    click(&mut app, power, egui::PointerButton::Primary);
    assert!(app.macro_panels[0].session.state.power_attack);
    output = draw(&mut app, vec![]);
    let power = find(&output, power_text);
    click(&mut app, power, egui::PointerButton::Secondary);
    assert!(!app.macro_panels[0].session.state.power_attack);
    app.players[0] = PlayerConfig::new("Opening damage", app.players[0].weapon_id);
    app.macro_panels[0].session.reset();
    app.players[0].proficiencies = vec!["Halberd".into()];
    app.players[0].default_weapon_style_ids = Some(vec!["armeroci_pole".into()]);
    app.players[0].talents.push(TalentSelection {
        id: "armeroci_pole".into(), rank: 1, weapon: None,
    });
    draw(&mut app, vec![]);
    output = draw(&mut app, vec![]);
    find(&output, "Damage options");
    let opening = find(&output, "Opening engagement · +1 weapon die   [[3d10!p");
    click(&mut app, opening, egui::PointerButton::Primary);
    output = draw(&mut app, vec![]);
    let damage = find(&output, "Damage — Opening engagement · +1 weapon die\n[[3d10!p");
    output = click(&mut app, damage, egui::PointerButton::Primary);
    assert!(output.platform_output.copied_text.starts_with("[[3d10!p"));
    assert!(output.platform_output.copied_text.contains("Opening engagement"));
    output = draw(&mut app, vec![]);
    let attack = find(&output, "Attack\n[[");
    click(&mut app, attack, egui::PointerButton::Primary);
    app.players[0].talents.push(TalentSelection {
        id: "rohavalan_bridge".into(), rank: 1, weapon: None,
    });
    app.players[0].talents.push(TalentSelection {
        id: "falling_sun".into(), rank: 1, weapon: None,
    });
    let saved_player = app.players[0].clone();
    output = draw(&mut app, vec![]);
    let dropdown = find(&output, "Armeroci Pole");
    click(&mut app, dropdown, egui::PointerButton::Primary);
    output = draw(&mut app, vec![]);
    find(&output, "Falling Sun — unavailable with current equipment");
    let bridge = find(&output, "Rohavalan Bridge");
    click(&mut app, bridge, egui::PointerButton::Primary);
    output = draw(&mut app, vec![]);
    find(&output, "Close hit · margin <10   [[2d4!p");
    assert_eq!(app.macro_panels[0].session.state.weapon_style_ids,
        Some(vec!["rohavalan_bridge".into()]));
    assert!(app.players[0] == saved_player);
    app.macro_panels[0].session.undo();
    output = draw(&mut app, vec![]);
    find(&output, "Opening engagement · +1 weapon die   [[3d10!p");
}
