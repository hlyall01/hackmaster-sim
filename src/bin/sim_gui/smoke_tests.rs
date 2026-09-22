//! Render every simulator UI section without requiring a native window.
use super::*;

#[test]
fn all_gui_sections_render_and_tessellate() {
    let mut app = SimGuiApp::new();
    let ctx = egui::Context::default();
    for view in 0..15 {
        app.show_player_editor = [false, false];
        if view < 3 {
            app.active_tab = [MainTab::Simulator, MainTab::DetailedStats, MainTab::Tools][view];
        } else if view < 6 {
            app.active_tab = MainTab::Tools;
            app.active_tool_tab = [
                ToolTab::WoundHealing,
                ToolTab::EssenceWounds,
                ToolTab::EgoGeneration,
            ][view - 3];
        } else {
            app.active_tab = MainTab::Simulator;
            app.show_player_editor = [true, false];
            app.player_editor_tabs[0] = PLAYER_EDITOR_TABS[view - 6];
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
