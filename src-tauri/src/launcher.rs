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

// ═══════════════════════════════════════════════
//  LINUX: .desktop file scanning & launching
// ═══════════════════════════════════════════════

/// Scan standard Linux .desktop file locations and return parsed app entries.
#[cfg(target_os = "linux")]
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
#[cfg(target_os = "linux")]
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

/// Launch an application by its exec command (Linux).
#[cfg(target_os = "linux")]
pub fn launch_app(exec_cmd: &str) -> Result<(), String> {
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

// ═══════════════════════════════════════════════
//  WINDOWS: Start Menu .lnk scanning & launching
// ═══════════════════════════════════════════════

/// Scan Windows Start Menu folders for .lnk shortcuts and return them as app entries.
#[cfg(target_os = "windows")]
pub fn scan_desktop_apps() -> Vec<FileEntry> {
    let mut apps: Vec<FileEntry> = Vec::new();
    let mut search_dirs: Vec<PathBuf> = Vec::new();

    // System-wide Start Menu: C:\ProgramData\Microsoft\Windows\Start Menu\Programs
    if let Ok(val) = std::env::var("ProgramData") {
        let dir = PathBuf::from(val).join("Microsoft\\Windows\\Start Menu\\Programs");
        if dir.exists() {
            search_dirs.push(dir);
        }
    }

    // Per-user Start Menu: %AppData%\Microsoft\Windows\Start Menu\Programs
    if let Ok(val) = std::env::var("APPDATA") {
        let dir = PathBuf::from(val).join("Microsoft\\Windows\\Start Menu\\Programs");
        if dir.exists() {
            search_dirs.push(dir);
        }
    }

    // User Desktop (often has shortcuts too)
    if let Some(home) = dirs::home_dir() {
        let desktop = home.join("Desktop");
        if desktop.exists() {
            search_dirs.push(desktop);
        }
    }

    let mut seen_names = std::collections::HashSet::new();

    for dir in search_dirs {
        scan_lnk_dir_recursive(&dir, &mut apps, &mut seen_names);
    }

    println!("[trace][launcher] Found {} applications", apps.len());
    apps
}

/// Recursively scan a directory for .lnk files and parse them into FileEntry.
#[cfg(target_os = "windows")]
fn scan_lnk_dir_recursive(
    dir: &PathBuf,
    apps: &mut Vec<FileEntry>,
    seen: &mut std::collections::HashSet<String>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            scan_lnk_dir_recursive(&path, apps, seen);
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext.eq_ignore_ascii_case("lnk") {
            if let Some(app) = parse_lnk_file(&path) {
                // De-duplicate by name (same app can appear in multiple Start Menu folders)
                let key = app.name.to_lowercase();
                if seen.contains(&key) {
                    continue;
                }
                seen.insert(key);

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
}

/// Extract app info from a .lnk shortcut file.
/// We use the filename (minus .lnk extension) as the display name
/// and store the .lnk path itself — Windows `open::that` follows shortcuts natively.
#[cfg(target_os = "windows")]
fn parse_lnk_file(path: &PathBuf) -> Option<DesktopApp> {
    // Derive the display name from the .lnk filename (without extension)
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())?
        .to_string();

    // Skip common uninstallers, readmes, help files by name
    let name_lower = name.to_lowercase();
    if name_lower.contains("uninstall")
        || name_lower.contains("uninst")
        || name_lower.contains("readme")
        || name_lower.contains("help")
        || name_lower.contains("license")
        || name_lower.contains("changelog")
        || name_lower.contains("release notes")
    {
        return None;
    }

    Some(DesktopApp {
        name,
        exec: path.to_string_lossy().to_string(), // store the .lnk path as "exec"
        icon: None,
        path: path.to_string_lossy().to_string(),
    })
}

/// Launch an application from a .lnk shortcut or exe path (Windows).
/// Uses `open::that` which natively follows .lnk shortcuts on Windows.
#[cfg(target_os = "windows")]
pub fn launch_app(exec_cmd: &str) -> Result<(), String> {
    open::that(exec_cmd).map_err(|e| format!("Failed to launch {}: {}", exec_cmd, e))
}
