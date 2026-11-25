//! Throttled progress messages from the calculation worker's private WASM instance.
use std::cell::Cell;
use wasm_bindgen::JsCast;

thread_local! { static LAST_UPDATE: Cell<f64> = const { Cell::new(0.0) }; }

/// A warm pool can carry its throttle time from the previous job.
pub fn report_first_fight(total: u32) {
    send_progress(1, total, 0.0);
}

pub fn report(completed: u32, total: u32) {
    send_progress(completed, total, 50.0);
}

fn send_progress(completed: u32, total: u32, min_interval_ms: f64) {
    let Ok(worker) = js_sys::global().dyn_into::<web_sys::DedicatedWorkerGlobalScope>() else {
        return;
    };
    let now = js_sys::Date::now();
    LAST_UPDATE.with(|last| {
        if completed != total && now - last.get() < min_interval_ms {
            return;
        }
        last.set(now);
        let message = serde_json::json!({"progress": [completed, total]}).to_string();
        let _ = worker.post_message(&message.into());
    });
}
