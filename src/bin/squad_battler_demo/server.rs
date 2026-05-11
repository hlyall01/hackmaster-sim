use super::web_assets;
use hackmaster_sim::squad_battler::rewards::RecruitDestination;
use hackmaster_sim::squad_battler::state::{FightCommand, SquadBattlerApp};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

pub(crate) fn handle_connection(mut stream: TcpStream, app: Arc<Mutex<SquadBattlerApp>>) {
    let Ok(request) = read_request(&mut stream) else {
        return;
    };
    let response = route_request(request, app);
    let _ = stream.write_all(response.as_bytes());
}

fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    let mut buffer = Vec::new();
    let mut header_end = None;
    let mut content_length = 0;
    let mut chunk = [0_u8; 4096];

    loop {
        let size = stream.read(&mut chunk).map_err(|err| err.to_string())?;
        if size == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..size]);

        if header_end.is_none() {
            if let Some(end) = find_header_end(&buffer) {
                let head = String::from_utf8_lossy(&buffer[..end]).to_string();
                content_length = parse_content_length(&head);
                header_end = Some(end + 4);
            }
        }

        if let Some(body_start) = header_end {
            if buffer.len() >= body_start + content_length {
                break;
            }
        }
    }

    let Some(body_start) = header_end else {
        return Err("missing request headers".to_string());
    };
    let head = String::from_utf8_lossy(&buffer[..body_start - 4]).to_string();
    let body_end = (body_start + content_length).min(buffer.len());
    let body = String::from_utf8_lossy(&buffer[body_start..body_end]).to_string();
    let mut lines = head.lines();
    let first = lines.next().ok_or_else(|| "empty request".to_string())?;
    let mut first_parts = first.split_whitespace();
    let method = first_parts.next().unwrap_or_default().to_string();
    let path = first_parts.next().unwrap_or_default().to_string();
    Ok(HttpRequest { method, path, body })
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn parse_content_length(head: &str) -> usize {
    head.lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0)
}

struct HttpRequest {
    method: String,
    path: String,
    body: String,
}

fn route_request(request: HttpRequest, app: Arc<Mutex<SquadBattlerApp>>) -> String {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => html_response(web_assets::INDEX_HTML),
        ("GET", path) if path.starts_with("/static/") => match web_assets::get(path) {
            Some(asset) => http_response(200, asset.content_type, asset.body.to_string()),
            None => error_response(404, "Not found".to_string()),
        },
        ("GET", "/api/state") => {
            let app = app.lock().expect("squad battler lock poisoned");
            json_response(200, &app.view())
        }
        ("POST", "/api/new-run") => {
            let parsed = serde_json::from_str::<NewRunRequest>(&request.body)
                .unwrap_or(NewRunRequest { seed: None });
            let mut app = app.lock().expect("squad battler lock poisoned");
            let view = app.new_run(parsed.seed);
            json_response(200, &view)
        }
        ("POST", "/api/choose-node") => {
            let parsed = serde_json::from_str::<ChooseNodeRequest>(&request.body);
            match parsed {
                Ok(request) => {
                    let mut app = app.lock().expect("squad battler lock poisoned");
                    match app.choose_node(request.node_id) {
                        Ok(view) => json_response(200, &view),
                        Err(err) => error_response(400, err),
                    }
                }
                Err(err) => error_response(400, format!("Bad request: {err}")),
            }
        }
        ("POST", "/api/start-fight") => {
            let mut app = app.lock().expect("squad battler lock poisoned");
            match app.start_fight() {
                Ok(view) => json_response(200, &view),
                Err(err) => error_response(400, err),
            }
        }
        ("POST", "/api/fight-command") => {
            let parsed = serde_json::from_str::<FightCommandRequest>(&request.body);
            match parsed {
                Ok(request) => {
                    let mut app = app.lock().expect("squad battler lock poisoned");
                    match app.fight_command(request.command, request.seconds) {
                        Ok(view) => json_response(200, &view),
                        Err(err) => error_response(400, err),
                    }
                }
                Err(err) => error_response(400, format!("Bad request: {err}")),
            }
        }
        ("POST", "/api/recruit-choice") => {
            let parsed = serde_json::from_str::<RecruitChoiceRequest>(&request.body);
            match parsed {
                Ok(request) => {
                    let mut app = app.lock().expect("squad battler lock poisoned");
                    match app.recruit_choice(
                        request.candidate_id,
                        request.destination,
                        request.replace_member_id,
                    ) {
                        Ok(view) => json_response(200, &view),
                        Err(err) => error_response(400, err),
                    }
                }
                Err(err) => error_response(400, format!("Bad request: {err}")),
            }
        }
        _ => error_response(404, "Not found".to_string()),
    }
}

#[derive(Clone, Debug, Deserialize)]
struct NewRunRequest {
    seed: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
struct ChooseNodeRequest {
    node_id: usize,
}

#[derive(Clone, Debug, Deserialize)]
struct FightCommandRequest {
    command: FightCommand,
    seconds: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
struct RecruitChoiceRequest {
    candidate_id: String,
    destination: RecruitDestination,
    replace_member_id: Option<String>,
}

fn html_response(body: &str) -> String {
    http_response(200, "text/html; charset=utf-8", body.to_string())
}

fn json_response<T: Serialize>(status: u16, body: &T) -> String {
    match serde_json::to_string(body) {
        Ok(json) => http_response(status, "application/json", json),
        Err(err) => error_response(500, err.to_string()),
    }
}

fn error_response(status: u16, message: String) -> String {
    let body = serde_json::json!({ "error": message }).to_string();
    http_response(status, "application/json", body)
}

fn http_response(status: u16, content_type: &str, body: String) -> String {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    };
    format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}
