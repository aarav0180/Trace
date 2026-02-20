mod autostart;
mod commands;
mod doc_chat;
mod indexer;
mod launcher;
mod llm;
mod search;
mod settings;
mod shell_cmd;
mod watcher;

use commands::AppState;
use indexer::FileIndex;
use llm::LlmClient;
use settings::Settings;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let settings = Settings::load();
    let home = dirs::home_dir().unwrap_or_else(|| {
        // Platform-safe fallback
        if cfg!(windows) {
            PathBuf::from(std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users".to_string()))
        } else {
            PathBuf::from("/home")
        }
    });
    let roots: Vec<PathBuf> = settings
        .index_roots
        .iter()
        .map(|r| {
            if r == "~" {
                home.clone()
            } else if r.starts_with("~/") {
                home.join(&r[2..])
            } else {
                PathBuf::from(r)
            }
        })
        .collect();

    let index: FileIndex = indexer::new_index();
    let state = AppState {
        index: index.clone(),
        settings: Arc::new(RwLock::new(settings)),
        llm: LlmClient::new(),
        chat_file_content: Arc::new(RwLock::new(None)),
    };

    let index_for_build = index.clone();
    let roots_for_build = roots.clone();
    let index_for_watch = index.clone();
    let roots_for_watch = roots.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Another instance was launched (e.g. via system shortcut).
            // Toggle the existing window instead of opening a duplicate.
            if let Some(window) = app.get_webview_window("main") {
                if window.is_visible().unwrap_or(false) {
                    let _ = window.hide();
                } else {
                    let _ = window.show();
                    let _ = window.set_focus();
                    let _ = window.center();
                }
            }
        }))
        .manage(state)
        .setup(move |app| {
            let _handle = app.handle().clone();

            // Register system-level keyboard shortcut (Super+T or first free key)
            if let Ok(exe) = std::env::current_exe() {
                let exe_str = exe.to_string_lossy().to_string();
                let shortcut = autostart::register_system_shortcut(&exe_str);
                autostart::install_desktop_entry(&exe_str);
                println!("[trace] Press {} to launch/toggle Trace", shortcut);
            }

            // Build the file index in the background
            tauri::async_runtime::spawn(async move {
                // First, add desktop apps to the index
                let apps = launcher::scan_desktop_apps();
                {
                    let mut idx = index_for_build.write().await;
                    idx.extend(apps);
                }

                // Then scan the filesystem
                indexer::build_index(index_for_build.clone(), roots_for_build).await;

                println!("[trace] Index ready. Starting file watcher...");

                // Start watching for changes
                watcher::start_watcher(index_for_watch, roots_for_watch).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::search_files,
            commands::open_result,
            commands::get_settings,
            commands::save_settings,
            commands::translate_command,
            commands::execute_shell,
            commands::enter_chat_mode,
            commands::chat_message,
            commands::exit_chat_mode,
            commands::get_system_info,
            commands::get_registered_shortcut,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Trace");
}
