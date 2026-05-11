#[path = "server.rs"]
mod server;
#[path = "web_assets.rs"]
mod web_assets;

use hackmaster_sim::squad_battler::state::SquadBattlerApp;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

const DEFAULT_PORT: u16 = 8788;

pub(crate) fn run() {
    hackmaster_sim::console::maybe_enable_console();
    let port = std::env::args()
        .skip_while(|arg| arg != "--port")
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);

    let app =
        Arc::new(Mutex::new(SquadBattlerApp::new().unwrap_or_else(|err| {
            panic!("Failed to start squad battler: {err}")
        })));
    let listener = TcpListener::bind(("127.0.0.1", port))
        .unwrap_or_else(|err| panic!("Failed to bind 127.0.0.1:{port}: {err}"));

    println!("HackMaster squad battler demo running at http://127.0.0.1:{port}");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let app = Arc::clone(&app);
                std::thread::spawn(move || server::handle_connection(stream, app));
            }
            Err(err) => eprintln!("Connection failed: {err}"),
        }
    }
}
