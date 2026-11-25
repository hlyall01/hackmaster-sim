use super::*;
use std::{sync::mpsc, time::Duration};
use wasm_bindgen::{JsCast, prelude::*};

#[wasm_bindgen(
    inline_js = "export function show_feature_requests(visible, top) { window.dispatchEvent(new CustomEvent('sim-feature-tab', {detail: {visible, top}})); }"
)]
extern "C" {
    pub(super) fn show_feature_requests(visible: bool, top: f32);
}

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
    completed: std::rc::Rc<std::cell::Cell<bool>>,
    _message: Closure<dyn FnMut(web_sys::MessageEvent)>,
    _error: Closure<dyn FnMut(web_sys::ErrorEvent)>,
}

thread_local! { static IDLE_WORKER: std::cell::RefCell<Option<web_sys::Worker>> = const { std::cell::RefCell::new(None) }; }

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
        let completed = std::rc::Rc::new(std::cell::Cell::new(false));
        let finished = completed.clone();
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
                    finished.set(true);
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
        let worker = IDLE_WORKER
            .with(|idle| {
                idle.borrow_mut()
                    .take()
                    .map(Ok)
                    .unwrap_or_else(|| web_sys::Worker::new_with_options("./worker.js", &options))
            })
            .and_then(|worker| {
                worker.set_onmessage(Some(message.as_ref().unchecked_ref()));
                worker.set_onerror(Some(error.as_ref().unchecked_ref()));
                let bootstrap = js_sys::Object::new();
                if js_sys::Reflect::set(&bootstrap, &"module".into(), &wasm_bindgen::module())
                    .is_ok()
                {
                    let _ = worker.post_message(&bootstrap);
                }
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
            completed,
            _message: message,
            _error: error,
        }
    }
}
impl Drop for BackgroundJob {
    fn drop(&mut self) {
        self.control.cancel();
        if let Some(worker) = self.worker.take() {
            worker.set_onmessage(None);
            worker.set_onerror(None);
            if self.completed.get() {
                IDLE_WORKER.with(|idle| {
                    if let Some(previous) = idle.borrow_mut().replace(worker) {
                        previous.terminate();
                    }
                });
            } else {
                worker.terminate();
            }
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
                sim::bulk_simulate_batch(
                    config,
                    combatants,
                    runs,
                    BULK_SIM_MAX_SECONDS,
                    seed,
                    0,
                    |completed| control.keep_running(completed),
                )
                .map(|result| JobOutput::Bulk(Box::new(result.finish()), seed))
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

struct PreparedBulkJob {
    request: String,
    simulation: sim::PreparedBulkSimulation,
    runs: u32,
    seed: u64,
}
thread_local! { static PREPARED_BULK: std::cell::RefCell<Option<PreparedBulkJob>> = const { std::cell::RefCell::new(None) }; }

fn bulk_shard(request: &str, first_run: u32, runs: u32, report_progress: bool) -> String {
    let result = PREPARED_BULK.with(|cache| -> Result<sim::BulkSimBatch, String> {
        let mut cache = cache.borrow_mut();
        if cache.as_ref().is_none_or(|job| job.request != request) {
            let parsed: Request = serde_json::from_str(request).map_err(|e| e.to_string())?;
            let JobKind::Bulk { runs, seed } = parsed.kind else {
                return Err("Expected a bulk request".into());
            };
            let (weapons, armor, shields) = data::load_catalogs()?;
            let npcs = data::load_npc_presets("data/npc_presets.json")?;
            let talents = data::load_talents(data::TALENTS_PATH)?;
            let fighters = game_logic::build_combatants(
                &parsed.players,
                &weapons,
                &armor,
                &shields,
                &npcs,
                &talents,
            );
            *cache = Some(PreparedBulkJob {
                request: request.to_owned(),
                simulation: sim::PreparedBulkSimulation::new(SimConfig::new(parsed.start_distance, parsed.stop_distance), fighters),
                runs,
                seed,
            });
        }
        let job = cache.as_ref().unwrap();
        if first_run > job.runs || runs > job.runs - first_run {
            return Err("Invalid bulk range".into());
        }
        let control = JobControl::default();
        control.set_total(runs);
        let mut previous_fight = None;
        let mut same_fight_polls = 0u32;
        job.simulation.run_range(
            runs,
            BULK_SIM_MAX_SECONDS,
            job.seed,
            first_run,
            |completed| {
                if !report_progress { return !control.is_cancelled(); }
                // A long fight also polls every 64 simulated seconds. Keep its
                // heartbeat even when its index falls between clock samples.
                let check_clock = if previous_fight == Some(completed) {
                    same_fight_polls = same_fight_polls.wrapping_add(1);
                    same_fight_polls.is_multiple_of(16)
                } else {
                    previous_fight = Some(completed);
                    same_fight_polls = 0;
                    if first_run == 0 && completed == 1 {
                        hackmaster_sim::web_progress::report_first_fight(runs);
                        return !control.is_cancelled();
                    }
                    completed.is_multiple_of(64) || completed == runs
                };
                if check_clock { control.keep_running(completed) } else { !control.is_cancelled() }
            },
        )
        .ok_or_else(|| "Calculation cancelled".into())
    });
    serde_json::to_string(&result).unwrap()
}

/// Preserve the original request so JavaScript never rounds a u64 seed.
#[wasm_bindgen]
pub fn run_web_bulk_shard(request: &str, first_run: u32, runs: u32) -> String {
    bulk_shard(request, first_run, runs, true)
}

/// The coordinator reports aggregate progress after each short local batch.
#[wasm_bindgen]
pub fn run_web_bulk_shard_quiet(request: &str, first_run: u32, runs: u32) -> String {
    bulk_shard(request, first_run, runs, false)
}

#[wasm_bindgen]
pub fn merge_web_bulk_shards(request: &str, shards: &str, elapsed_ms: f64) -> String {
    let result = (|| -> JobResult {
        let request: Request = serde_json::from_str(request).map_err(|e| e.to_string())?;
        let JobKind::Bulk { runs, seed } = request.kind else {
            return Err("Expected a bulk request".into());
        };
        let shards: Vec<sim::BulkSimBatch> =
            serde_json::from_str(shards).map_err(|e| e.to_string())?;
        let mut merged = sim::BulkSimBatch::default();
        for shard in shards {
            merged.merge(shard);
        }
        if merged.runs() != runs {
            return Err("Incomplete bulk calculation".into());
        }
        Ok((
            Some(JobOutput::Bulk(Box::new(merged.finish()), seed)),
            Duration::from_secs_f64(elapsed_ms.max(0.0) / 1000.0),
        ))
    })();
    serde_json::to_string(&result).unwrap()
}

#[wasm_bindgen]
pub async fn start_web(show_requests: bool) -> Result<(), JsValue> {
    eframe::WebRunner::new()
        .start(
            "sim_canvas",
            eframe::WebOptions {
                follow_system_theme: false,
                default_theme: eframe::Theme::Dark,
                ..Default::default()
            },
            Box::new(move |_cc| {
                let mut app = SimGuiApp::new();
                if show_requests {
                    app.active_tab = super::MainTab::FeatureRequests;
                }
                Box::new(app)
            }),
        )
        .await
}
