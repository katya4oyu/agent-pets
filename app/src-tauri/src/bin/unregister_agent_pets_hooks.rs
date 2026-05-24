use agent_pets_lib::{
    is_agent_pets_copilot_config, remove_agent_pets_hooks_from_claude_settings,
    remove_agent_pets_hooks_from_codex,
};
use serde_json::Value;
use std::{env, fs, path::Path};

fn read_json(path: &Path) -> Result<Option<Value>, String> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|error| format!("{} のパースに失敗: {error}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("{} の読み込みに失敗: {error}", path.display())),
    }
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("JSON シリアライズに失敗: {error}"))?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, text)
        .map_err(|error| format!("{} への書き込みに失敗: {error}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|error| format!("{} への反映に失敗: {error}", path.display()))
}

fn home_dir() -> Result<std::path::PathBuf, String> {
    env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| "HOME が見つかりません".to_string())
}

fn main() {
    let result = run();
    match result {
        Ok(messages) => {
            for message in messages {
                println!("{message}");
            }
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<Vec<String>, String> {
    let home = home_dir()?;
    let mut messages = Vec::new();

    let codex_path = home.join(".codex").join("hooks.json");
    if let Some(mut hooks) = read_json(&codex_path)? {
        let removed = remove_agent_pets_hooks_from_codex(&mut hooks);
        if removed > 0 {
            write_json(&codex_path, &hooks)?;
        }
        messages.push(format!(
            "Codex: {removed} handler(s) removed from {}",
            codex_path.display()
        ));
    }

    let claude_path = home.join(".claude").join("settings.json");
    if let Some(mut settings) = read_json(&claude_path)? {
        let removed = remove_agent_pets_hooks_from_claude_settings(&mut settings);
        if removed > 0 {
            write_json(&claude_path, &settings)?;
        }
        messages.push(format!(
            "Claude Code: {removed} handler(s) removed from {}",
            claude_path.display()
        ));
    }

    let copilot_path = home.join(".copilot").join("hooks").join("agent-pets.json");
    if let Some(config) = read_json(&copilot_path)? {
        if is_agent_pets_copilot_config(&config) {
            fs::remove_file(&copilot_path)
                .map_err(|error| format!("{} の削除に失敗: {error}", copilot_path.display()))?;
            messages.push(format!("Copilot: removed {}", copilot_path.display()));
        } else {
            messages.push(format!(
                "Copilot: skipped {}, unrelated hook command found",
                copilot_path.display()
            ));
        }
    }

    if messages.is_empty() {
        messages.push("No hook files found.".to_string());
    }
    Ok(messages)
}
