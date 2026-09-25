//! Throttled progress messages from the calculation worker's private WASM instance.
use std::cell::Cell;
use wasm_bindgen::JsCast;

thread_local! { static LAST_UPDATE: Cell<f64> = const { Cell::new(0.0) }; }

pub fn report(completed: u32, total: u32) {
    let Ok(worker) = js_sys::global().dyn_into::<web_sys::DedicatedWorkerGlobalScope>() else {
        return;
    };
    let now = js_sys::Date::now();
    LAST_UPDATE.with(|last| {
        if completed != total && now - last.get() < 50.0 {
            return;
        }
        last.set(now);
        let message = serde_json::json!({"progress": [completed, total]}).to_string();
        let _ = worker.post_message(&message.into());
    });
}
