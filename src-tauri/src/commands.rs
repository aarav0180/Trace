/// Tauri command handlers — the bridge between the frontend and Rust backend.
use crate::doc_chat;
use crate::indexer::FileIndex;
use crate::launcher;
use crate::llm::LlmClient;
use crate::search::{self, SearchResult};
use crate::settings::Settings;
use crate::shell_cmd::{self, ShellOutput, ShellTranslation};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

/// Shared app state passed to every Tauri command.
pub struct AppState {
    pub index: FileIndex,
    pub settings: Arc<RwLock<Settings>>,
    pub llm: LlmClient,
    pub chat_file_content: Arc<RwLock<Option<(String, String)>>>, // (path, content)
}

// ─── SEARCH ──────────────────────────────────────────────

#[tauri::command]
pub async fn search_files(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    let settings = state.settings.read().await;
    let results = search::fuzzy_search(&state.index, &query, settings.max_results).await;
    Ok(results)
}

// ─── FILE OPEN / APP LAUNCH ──────────────────────────────

#[tauri::command]
pub async fn open_result(path: String, kind: String) -> Result<(), String> {
    match kind.as_str() {
        "App" => open_app(&path),
        _ => {
            // Open file with default application
            open::that(&path).map_err(|e| format!("Failed to open: {}", e))
        }
    }
}

/// Open an app entry. On Linux, re-parse the .desktop file for Exec=.
/// On Windows, launch the .lnk shortcut directly.
#[cfg(target_os = "linux")]
fn open_app(path: &str) -> Result<(), String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read .desktop file: {}", e))?;

    let exec = content
        .lines()
        .find(|l| l.starts_with("Exec="))
        .map(|l| {
            l.strip_prefix("Exec=")
                .unwrap_or("")
                .replace("%f", "")
                .replace("%F", "")
                .replace("%u", "")
                .replace("%U", "")
                .trim()
                .to_string()
        })
        .ok_or("No Exec= field found in .desktop file")?;

    launcher::launch_app(&exec)
}

/// On Windows, open the .lnk shortcut (the OS knows how to follow it).
#[cfg(target_os = "windows")]
fn open_app(path: &str) -> Result<(), String> {
    // The path stored is the .lnk file itself — open::that will follow the shortcut
    open::that(path).map_err(|e| format!("Failed to launch app: {}", e))
}

// ─── SETTINGS ────────────────────────────────────────────

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let settings = state.settings.read().await;
    Ok(settings.clone())
}

#[tauri::command]
pub async fn save_settings(
    new_settings: Settings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    new_settings.save()?;
    let mut settings = state.settings.write().await;
    *settings = new_settings;
    Ok(())
}

// ─── NLP-TO-SHELL (Phase 3) ─────────────────────────────

#[tauri::command]
pub async fn translate_command(
    input: String,
    state: State<'_, AppState>,
) -> Result<ShellTranslation, String> {
    let settings = state.settings.read().await;
    shell_cmd::translate_to_command(&state.llm, &settings, &input).await
}

#[tauri::command]
pub async fn execute_shell(command: String) -> Result<ShellOutput, String> {
    // Run on blocking thread since Command::output blocks
    tokio::task::spawn_blocking(move || shell_cmd::execute_command(&command))
        .await
        .map_err(|e| format!("Task failed: {}", e))?
}

// ─── DOCUMENT CHAT (Phase 3) ────────────────────────────

#[tauri::command]
pub async fn enter_chat_mode(
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let content = doc_chat::read_file_content(&path)?;
    let preview = if content.len() > 500 {
        format!("{}...", &content[..500])
    } else {
        content.clone()
    };

    let mut chat = state.chat_file_content.write().await;
    *chat = Some((path, content));

    Ok(preview)
}

#[tauri::command]
pub async fn chat_message(
    question: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let chat = state.chat_file_content.read().await;
    let (file_path, file_content) = chat
        .as_ref()
        .ok_or("Not in chat mode. Select a file first.")?;

    let settings = state.settings.read().await;
    doc_chat::chat_about_file(&state.llm, &settings, file_path, file_content, &question).await
}

#[tauri::command]
pub async fn exit_chat_mode(state: State<'_, AppState>) -> Result<(), String> {
    let mut chat = state.chat_file_content.write().await;
    *chat = None;
    Ok(())
}

// ─── REGISTERED SHORTCUT ─────────────────────────────────

#[tauri::command]
pub fn get_registered_shortcut() -> Result<String, String> {
    let path = dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("trace")
        .join("shortcut");
    std::fs::read_to_string(&path)
        .map(|s| s.trim().to_string())
        .map_err(|_| "No shortcut registered yet".to_string())
}

// ─── SYSTEM INFO ─────────────────────────────────────────

#[tauri::command]
pub fn get_system_info() -> serde_json::Value {
    use sysinfo::System;

    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_usage = sys.global_cpu_usage();
    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();

    let top_processes: Vec<serde_json::Value> = {
        let mut procs: Vec<_> = sys.processes().values().collect();
        procs.sort_by(|a, b| {
            b.memory().cmp(&a.memory())
        });
        procs
            .iter()
            .take(3)
            .map(|p| {
                serde_json::json!({
                    "name": p.name().to_string_lossy(),
                    "memory_mb": p.memory() / 1_048_576,
                    "cpu_percent": p.cpu_usage(),
                })
            })
            .collect()
    };

    serde_json::json!({
        "cpu_usage": cpu_usage,
        "total_memory_mb": total_mem / 1_048_576,
        "used_memory_mb": used_mem / 1_048_576,
        "memory_percent": (used_mem as f64 / total_mem as f64) * 100.0,
        "top_processes": top_processes,
    })
}
