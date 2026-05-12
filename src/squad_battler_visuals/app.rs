use bevy::prelude::*;
use bevy::window::WindowResizeConstraints;

use crate::squad_battler::encounters::SquadNodeKind;
use crate::squad_battler::state::{FightCommand, SquadBattlerApp, SquadBattlerView};

use super::assets;
use super::board::{self, BoardGeometry};
use super::camera;
use super::units;

const DEMO_SEED: u64 = 0x5155_4144_4256_0001;

#[derive(Resource)]
pub struct VisualGameState {
    pub app: SquadBattlerApp,
    pub view: SquadBattlerView,
}

#[derive(Resource)]
struct CombatTickTimer(Timer);

pub fn run() {
    App::new()
        .insert_resource(ClearColor(assets::clear_color()))
        .insert_resource(CombatTickTimer(Timer::from_seconds(
            1.0,
            TimerMode::Repeating,
        )))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "HackMaster Squad Battler".to_string(),
                resolution: (1280.0, 768.0).into(),
                resize_constraints: WindowResizeConstraints {
                    min_width: 860.0,
                    min_height: 520.0,
                    ..default()
                },
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                camera::fit_camera_to_board,
                advance_combat,
                units::sync_unit_targets,
                units::animate_unit_motion,
            ),
        )
        .run();
}

fn setup(mut commands: Commands) {
    let (app, view) = boot_demo_fight().expect("failed to create squad battler demo fight");
    let fight = view
        .live_fight
        .as_ref()
        .expect("demo fight should be live after boot");
    let geometry = BoardGeometry::new(fight.grid);

    commands.insert_resource(geometry);
    camera::spawn_camera(&mut commands);
    board::spawn_board(&mut commands, geometry);
    units::spawn_units(&mut commands, geometry, fight);
    commands.insert_resource(VisualGameState { app, view });
}

fn advance_combat(
    time: Res<Time>,
    mut timer: ResMut<CombatTickTimer>,
    mut state: ResMut<VisualGameState>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    if state
        .view
        .live_fight
        .as_ref()
        .is_none_or(|fight| fight.done)
    {
        return;
    }
    match state.app.fight_command(FightCommand::Tick, Some(1)) {
        Ok(view) => state.view = view,
        Err(err) => eprintln!("squad battler visual tick failed: {err}"),
    }
}

fn boot_demo_fight() -> Result<(SquadBattlerApp, SquadBattlerView), String> {
    let mut app = SquadBattlerApp::new()?;
    let initial = app.new_run(Some(DEMO_SEED));
    let node_id = first_available_fight_node(&initial)
        .ok_or_else(|| "no available fight node in generated route".to_string())?;
    app.choose_node(node_id)?;
    let view = app.start_fight()?;
    Ok((app, view))
}

fn first_available_fight_node(view: &SquadBattlerView) -> Option<usize> {
    view.available_nodes.iter().find_map(|available_id| {
        let node = view.route.iter().find(|node| node.id == *available_id)?;
        matches!(
            node.kind,
            SquadNodeKind::Fight | SquadNodeKind::Elite | SquadNodeKind::Boss
        )
        .then_some(node.id)
    })
}
