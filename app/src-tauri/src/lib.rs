use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    Thinking,
    Running,
    Editing,
    WaitingApproval,
    Done,
    Error,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HookEvent {
    pub source: String,
    pub state: AgentState,
    pub label: String,
    pub message: Option<String>,
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub timestamp: Option<String>,
}

fn agent_pets_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".agent-pets"))
}

fn port_file_path() -> Option<std::path::PathBuf> {
    agent_pets_dir().map(|d| d.join("port"))
}

fn cleanup_stale_port_file() {
    if let Some(path) = port_file_path() {
        if let Ok(text) = std::fs::read_to_string(&path) {
            let is_stale = text.trim().parse::<u16>().map_or(true, |port| {
                std::net::TcpStream::connect(std::net::SocketAddr::from(([127, 0, 0, 1], port)))
                    .is_err()
            });
            if is_stale {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

fn write_port_file(port: u16) {
    if let Some(path) = port_file_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, port.to_string());
    }
}

fn remove_port_file() {
    if let Some(path) = port_file_path() {
        let _ = std::fs::remove_file(path);
    }
}

fn start_event_server(app_handle: tauri::AppHandle) {
    use std::io::Read;
    use tauri::Emitter;

    std::thread::spawn(move || {
        cleanup_stale_port_file();

        // Probe for a free port using TcpListener, then bind tiny_http to it
        let port = match std::net::TcpListener::bind("127.0.0.1:0")
            .and_then(|l| l.local_addr().map(|a| a.port()))
        {
            Ok(p) => p,
            Err(e) => {
                eprintln!("agent-pets: could not find free port: {e}");
                return;
            }
        };
        write_port_file(port);

        let server = match tiny_http::Server::http(format!("127.0.0.1:{port}")) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("agent-pets: HTTP server failed to start: {e}");
                remove_port_file();
                return;
            }
        };

        for mut request in server.incoming_requests() {
            if request.method() != &tiny_http::Method::Post || request.url() != "/events" {
                let _ = request.respond(
                    tiny_http::Response::from_string("not found").with_status_code(404),
                );
                continue;
            }

            let mut body = String::new();
            let read_ok = request.as_reader().read_to_string(&mut body).is_ok();

            if !read_ok {
                let _ = request.respond(
                    tiny_http::Response::from_string("bad request").with_status_code(400),
                );
                continue;
            }

            match serde_json::from_str::<HookEvent>(&body) {
                Ok(event) => {
                    let _ = app_handle.emit("agent-state-changed", &event);
                    let _ = request.respond(tiny_http::Response::from_string("ok"));
                }
                Err(_) => {
                    let _ = request.respond(
                        tiny_http::Response::from_string("bad request").with_status_code(400),
                    );
                }
            }
        }
    });
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PetAsset {
    id: String,
    display_name: String,
    description: String,
    spritesheet_path: String,
    spritesheet_mime: String,
    spritesheet_bytes: Vec<u8>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PetManifest {
    id: String,
    display_name: String,
    description: String,
    spritesheet_path: String,
}

#[tauri::command]
fn ping(message: &str) -> String {
    format!("{message} is listening.")
}

#[tauri::command]
fn load_pet_asset(pet_id: Option<String>) -> Result<PetAsset, String> {
    let pet_id = pet_id.unwrap_or_else(|| "mio".to_string());
    let home = env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .ok_or_else(|| "Could not find CODEX_HOME or HOME".to_string())?;
    let pet_dir = home.join("pets").join(&pet_id);
    let manifest_path = pet_dir.join("pet.json");
    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("Failed to read {}: {error}", manifest_path.display()))?;
    let manifest: PetManifest = serde_json::from_str(&manifest_text)
        .map_err(|error| format!("Failed to parse {}: {error}", manifest_path.display()))?;
    let spritesheet_path = pet_dir.join(&manifest.spritesheet_path);
    let spritesheet_bytes = fs::read(&spritesheet_path)
        .map_err(|error| format!("Failed to read {}: {error}", spritesheet_path.display()))?;
    let spritesheet_mime = match spritesheet_path.extension().and_then(|ext| ext.to_str()) {
        Some("png") => "image/png",
        _ => "image/webp",
    }
    .to_string();

    Ok(PetAsset {
        id: manifest.id,
        display_name: manifest.display_name,
        description: manifest.description,
        spritesheet_path: spritesheet_path.display().to_string(),
        spritesheet_mime,
        spritesheet_bytes,
    })
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            start_event_server(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![load_pet_asset, ping])
        .build(tauri::generate_context!())
        .expect("error building Agent Pets")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                remove_port_file();
            }
        });
}
