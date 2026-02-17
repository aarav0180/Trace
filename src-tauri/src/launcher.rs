use crate::indexer::{EntryKind, FileEntry};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopApp {
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub path: String,
}

/// Scan standard Linux .desktop file locations and return parsed app entries.
pub fn scan_desktop_apps() -> Vec<FileEntry> {
    let search_dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        dirs::home_dir()
            .map(|h| h.join(".local/share/applications"))
            .unwrap_or_default(),
    ];

    let mut apps: Vec<FileEntry> = Vec::new();

    for dir in search_dirs {
        if !dir.exists() {
            continue;
        }

        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }

            if let Some(app) = parse_desktop_file(&path) {
                apps.push(FileEntry {
                    name: app.name,
                    path: app.path,
                    kind: EntryKind::App,
                    size: 0,
                    modified: 0,
                });
            }
        }
    }

    println!("[trace][launcher] Found {} applications", apps.len());
    apps
}

/// Parse a .desktop file and extract Name and Exec fields.
fn parse_desktop_file(path: &PathBuf) -> Option<DesktopApp> {
    let content = std::fs::read_to_string(path).ok()?;

    let mut name: Option<String> = None;
    let mut exec: Option<String> = None;
    let mut icon: Option<String> = None;
    let mut no_display = false;
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        }
        if trimmed.starts_with('[') && trimmed != "[Desktop Entry]" {
            // New section — stop parsing
            if in_desktop_entry {
                break;
            }
            continue;
        }

        if !in_desktop_entry {
            continue;
        }

        if let Some(val) = trimmed.strip_prefix("Name=") {
            if name.is_none() {
                // Only use first Name= (not localized ones)
                name = Some(val.to_string());
            }
        } else if let Some(val) = trimmed.strip_prefix("Exec=") {
            // Strip field codes like %f, %u, %U, etc.
            let clean = val
                .replace("%f", "")
                .replace("%F", "")
                .replace("%u", "")
                .replace("%U", "")
                .replace("%i", "")
                .replace("%c", "")
                .replace("%k", "")
                .trim()
                .to_string();
            exec = Some(clean);
        } else if let Some(val) = trimmed.strip_prefix("Icon=") {
            icon = Some(val.to_string());
        } else if trimmed == "NoDisplay=true" {
            no_display = true;
        }
    }

    if no_display {
        return None;
    }

    let name = name?;
    let exec = exec?;

    Some(DesktopApp {
        name,
        exec,
        icon,
        path: path.to_string_lossy().to_string(),
    })
}

/// Launch an application by its exec command.
pub fn launch_app(exec_cmd: &str) -> Result<(), String> {
    // Split exec command into program and args
    let parts: Vec<&str> = exec_cmd.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty exec command".to_string());
    }

    let program = parts[0];
    let args = &parts[1..];

    Command::new(program)
        .args(args)
        .spawn()
        .map_err(|e| format!("Failed to launch {}: {}", program, e))?;

    Ok(())
}
