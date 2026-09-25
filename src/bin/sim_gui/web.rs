use super::*;
use std::{sync::mpsc, time::Duration};
use wasm_bindgen::{JsCast, prelude::*};

#[derive(serde::Serialize, serde::Deserialize)]
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
#[derive(serde::Serialize, serde::Deserialize)]
pub(super) enum JobOutput {
    Bulk(Box<BulkSimResult>, u64),
    Dps(DpsTestResult),
    Plot(usize, simulation_jobs::DamageRollPlotData),
}
type JobResult = Result<(Option<JobOutput>, Duration), String>;

#[derive(serde::Serialize, serde::Deserialize)]
struct Request {
    players: [PlayerConfig; 2],
    start_distance: f32,
    stop_distance: f32,
    kind: JobKind,
}

pub(super) struct BackgroundJob {
    pub receiver: mpsc::Receiver<JobResult>,
    pub control: JobControl,
    pub players: [PlayerConfig; 2],
    pub start_distance: f32,
    pub label: &'static str,
    worker: Option<web_sys::Worker>,
    _message: Closure<dyn FnMut(web_sys::MessageEvent)>,
    _error: Closure<dyn FnMut(web_sys::ErrorEvent)>,
}

impl BackgroundJob {
    pub fn spawn(app: &SimGuiApp, kind: JobKind) -> Self {
        let (label, total) = match &kind {
            JobKind::Bulk { runs, .. } => ("Bulk simulation", *runs),
            JobKind::Dps(request) => ("DPS test", request.iterations),
            JobKind::Plot { iterations, .. } => ("Damage distribution", (*iterations * 2) as u32),
        };
        let control = JobControl::default();
        control.set_total(total);
        let (sender, receiver) = mpsc::channel();
        let progress = control.clone();
        let result_sender = sender.clone();
        let message = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            let result = event
                .data()
                .as_string()
                .ok_or_else(|| "Invalid worker message".to_owned())
                .and_then(|text| {
                    serde_json::from_str::<serde_json::Value>(&text).map_err(|err| err.to_string())
                });
            match result {
                Ok(value) if value.get("progress").is_some() => {
                    if let Ok([completed, total]) =
                        serde_json::from_value::<[u32; 2]>(value["progress"].clone())
                    {
                        progress.set_total(total);
                        progress.keep_running(completed);
                    }
                }
                Ok(value) => {
                    let result = serde_json::from_value::<JobResult>(value)
                        .unwrap_or_else(|err| Err(format!("Invalid calculation result: {err}")));
                    let _ = result_sender.send(result);
                }
                Err(err) => {
                    let _ = result_sender.send(Err(err));
                }
            }
        }) as Box<dyn FnMut(web_sys::MessageEvent)>);
        let error_sender = sender.clone();
        let error = Closure::wrap(Box::new(move |event: web_sys::ErrorEvent| {
            let _ = error_sender.send(Err(format!(
                "Calculation worker failed: {}",
                event.message()
            )));
        }) as Box<dyn FnMut(web_sys::ErrorEvent)>);
        let options = web_sys::WorkerOptions::new();
        options.set_type(web_sys::WorkerType::Module);
        let worker =
            web_sys::Worker::new_with_options("./worker.js", &options).and_then(|worker| {
                worker.set_onmessage(Some(message.as_ref().unchecked_ref()));
                worker.set_onerror(Some(error.as_ref().unchecked_ref()));
                let request = Request {
                    players: app.players.clone(),
                    start_distance: app.sim.config.start_distance,
                    stop_distance: app.sim.config.stop_distance,
                    kind,
                };
                let sent = serde_json::to_string(&request)
                    .map_err(|err| JsValue::from_str(&err.to_string()))
                    .and_then(|request| worker.post_message(&request.into()));
                if let Err(err) = sent {
                    worker.terminate();
                    return Err(err);
                }
                Ok(worker)
            });
        let worker = match worker {
            Ok(worker) => Some(worker),
            Err(err) => {
                let _ = sender.send(Err(format!("Cannot start calculation: {err:?}")));
                None
            }
        };
        Self {
            receiver,
            control,
            players: app.players.clone(),
            start_distance: app.sim.config.start_distance,
            label,
            worker,
            _message: message,
            _error: error,
        }
    }
}
impl Drop for BackgroundJob {
    fn drop(&mut self) {
        self.control.cancel();
        if let Some(worker) = &self.worker {
            worker.set_onmessage(None);
            worker.set_onerror(None);
            worker.terminate();
        }
    }
}

/// Called only inside a dedicated worker; no GUI or filesystem is initialized.
#[wasm_bindgen]
pub fn run_web_job(request: &str) -> String {
    let result = (|| -> JobResult {
        let request: Request = serde_json::from_str(request).map_err(|err| err.to_string())?;
        let (weapons, armor, shields) = data::load_catalogs()?;
        let npcs = data::load_npc_presets("data/npc_presets.json")?;
        let talents = data::load_talents(data::TALENTS_PATH)?;
        let combatants = game_logic::build_combatants(
            &request.players,
            &weapons,
            &armor,
            &shields,
            &npcs,
            &talents,
        );
        let config = SimConfig::new(request.start_distance, request.stop_distance);
        let control = JobControl::default();
        let start = js_sys::Date::now();
        let output = match request.kind {
            JobKind::Bulk { runs, seed } => {
                control.set_total(runs);
                sim::bulk_simulate_with_seed_controlled(
                    config,
                    combatants,
                    runs,
                    BULK_SIM_MAX_SECONDS,
                    seed,
                    |completed| control.keep_running(completed),
                )
                .map(|result| JobOutput::Bulk(Box::new(result), seed))
            }
            JobKind::Dps(request) => {
                simulation_jobs::run_dps_test(config, combatants, request, &control)
                    .map(JobOutput::Dps)
            }
            JobKind::Plot {
                player_idx,
                iterations,
            } => {
                let combatant = combatants.get(player_idx).ok_or("Invalid fighter index")?;
                simulation_jobs::build_damage_roll_plot(combatant, iterations, &control)
                    .map(|result| JobOutput::Plot(player_idx, result))
            }
        };
        Ok((
            output,
            Duration::from_secs_f64(((js_sys::Date::now() - start) / 1000.0).max(0.0)),
        ))
    })();
    serde_json::to_string(&result)
        .unwrap_or_else(|err| serde_json::to_string(&Err::<(), _>(err.to_string())).unwrap())
}

#[wasm_bindgen]
pub async fn start_web() -> Result<(), JsValue> {
    eframe::WebRunner::new()
        .start(
            "sim_canvas",
            eframe::WebOptions::default(),
            Box::new(|_cc| Box::new(SimGuiApp::new())),
        )
        .await
}
