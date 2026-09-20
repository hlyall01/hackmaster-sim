use super::*;
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

pub(super) enum JobKind {
    Bulk {
        runs: u32,
        seed: u64,
    },
    Dps(DpsConfig),
    Plot {
        player_idx: usize,
        iterations: usize,
    },
}
pub(super) enum JobOutput {
    Bulk(Box<BulkSimResult>),
    Dps(DpsTestResult),
    Plot(usize, simulation_jobs::DamageRollPlotData),
}
type JobResult = Result<(Option<JobOutput>, Duration), String>;
pub(super) struct BackgroundJob {
    pub receiver: mpsc::Receiver<JobResult>,
    pub control: JobControl,
    pub players: [PlayerConfig; 2],
    pub start_distance: f32,
    pub label: &'static str,
}
impl BackgroundJob {
    pub fn spawn(app: &SimGuiApp, kind: JobKind) -> Self {
        let players = app.players.clone();
        let worker_players = players.clone();
        let weapons = app.weapon_catalog.clone();
        let armor = app.armor_catalog.clone();
        let shields = app.shield_catalog.clone();
        let npcs = app.npc_presets.clone();
        let talents = app.talent_catalog.clone();
        let config = SimConfig::new(app.sim.config.start_distance, app.sim.config.stop_distance);
        let (label, total) = match &kind {
            JobKind::Bulk { runs, .. } => ("Bulk simulation", *runs),
            JobKind::Dps(request) => ("DPS test", request.iterations),
            JobKind::Plot { iterations, .. } => ("Damage distribution", (*iterations * 2) as u32),
        };
        let control = JobControl::default();
        control.set_total(total);
        let worker_control = control.clone();
        let (sender, receiver) = mpsc::channel();
        // A detached worker owns all inputs. Closing the app cancels it; the UI never joins it.
        let _ = std::thread::Builder::new()
            .name("sim-gui-calculation".into())
            .spawn(move || {
                let start = Instant::now();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let combatants = game_logic::build_combatants(
                        &worker_players,
                        &weapons,
                        &armor,
                        &shields,
                        &npcs,
                        &talents,
                    );
                    match kind {
                        JobKind::Bulk { runs, seed } => sim::bulk_simulate_with_seed_controlled(
                            config,
                            combatants,
                            runs,
                            BULK_SIM_MAX_SECONDS,
                            seed,
                            |completed| worker_control.keep_running(completed),
                        )
                        .map(|result| JobOutput::Bulk(Box::new(result))),
                        JobKind::Dps(request) => simulation_jobs::run_dps_test(
                            config,
                            combatants,
                            request,
                            &worker_control,
                        )
                        .map(JobOutput::Dps),
                        JobKind::Plot {
                            player_idx,
                            iterations,
                        } => simulation_jobs::build_damage_roll_plot(
                            &combatants[player_idx],
                            iterations,
                            &worker_control,
                        )
                        .map(|result| JobOutput::Plot(player_idx, result)),
                    }
                }))
                .map(|result| (result, start.elapsed()))
                .map_err(|_| "Calculation failed. The simulator is still available.".to_owned());
                let _ = sender.send(result);
            });
        Self {
            receiver,
            control,
            players,
            start_distance: config.start_distance,
            label,
        }
    }
}
impl Drop for BackgroundJob {
    fn drop(&mut self) {
        self.control.cancel();
    }
}
