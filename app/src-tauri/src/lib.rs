use serde::Serialize;
use std::{env, fs, path::PathBuf};

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

#[derive(serde::Deserialize)]
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
        .invoke_handler(tauri::generate_handler![load_pet_asset, ping])
        .run(tauri::generate_context!())
        .expect("error while running Agent Pets");
}
