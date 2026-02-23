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
    pub keywords: Option<String>,
    pub generic_name: Option<String>,
    /// Resolved absolute path to the icon file on disk.
    pub icon_path: Option<String>,
}

// ═══════════════════════════════════════════════
//  LINUX: .desktop file scanning & launching
// ═══════════════════════════════════════════════

/// Scan all standard + XDG + Flatpak + Snap .desktop locations.
#[cfg(target_os = "linux")]
pub fn scan_desktop_apps() -> Vec<FileEntry> {
    let mut search_dirs: Vec<PathBuf> = Vec::new();

    // 1. XDG_DATA_DIRS (covers system, Flatpak exports, Snap, custom prefixes)
    if let Ok(xdg) = std::env::var("XDG_DATA_DIRS") {
        for dir in xdg.split(':') {
            let app_dir = PathBuf::from(dir).join("applications");
            if app_dir.exists() && !search_dirs.contains(&app_dir) {
                search_dirs.push(app_dir);
            }
        }
    }

    // 2. Hardcoded fallbacks (in case XDG_DATA_DIRS is unset)
    let fallbacks = [
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        dirs::home_dir()
            .map(|h| h.join(".local/share/applications"))
            .unwrap_or_default(),
    ];
    for dir in fallbacks {
        if dir.exists() && !search_dirs.contains(&dir) {
            search_dirs.push(dir);
        }
    }

    // 3. Flatpak exports (user & system)
    if let Some(home) = dirs::home_dir() {
        let fp_user = home.join(".local/share/flatpak/exports/share/applications");
        if fp_user.exists() && !search_dirs.contains(&fp_user) {
            search_dirs.push(fp_user);
        }
    }
    let fp_sys = PathBuf::from("/var/lib/flatpak/exports/share/applications");
    if fp_sys.exists() && !search_dirs.contains(&fp_sys) {
        search_dirs.push(fp_sys);
    }

    // 4. Snap desktop files
    let snap_dir = PathBuf::from("/var/lib/snapd/desktop/applications");
    if snap_dir.exists() && !search_dirs.contains(&snap_dir) {
        search_dirs.push(snap_dir);
    }

    // Detect icon theme once for all apps
    let icon_theme = detect_icon_theme();

    let mut apps: Vec<FileEntry> = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    for dir in search_dirs {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }

            if let Some(app) = parse_desktop_file(&path, &icon_theme) {
                // De-duplicate by lowercase name
                let key = app.name.to_lowercase();
                if seen_names.contains(&key) {
                    continue;
                }
                seen_names.insert(key);

                apps.push(FileEntry {
                    name: app.name,
                    path: app.path,
                    kind: EntryKind::App,
                    size: 0,
                    modified: 0,
                    icon_path: app.icon_path,
                    keywords: app.keywords,
                    generic_name: app.generic_name,
                });
            }
        }
    }

    println!("[trace][launcher] Found {} applications", apps.len());
    apps
}

/// Parse a .desktop file — extracts Name, Exec, Icon, Keywords, GenericName
/// and resolves the icon to an absolute filesystem path.
#[cfg(target_os = "linux")]
fn parse_desktop_file(path: &PathBuf, icon_theme: &str) -> Option<DesktopApp> {
    let content = std::fs::read_to_string(path).ok()?;

    let mut name: Option<String> = None;
    let mut exec: Option<String> = None;
    let mut icon: Option<String> = None;
    let mut keywords: Option<String> = None;
    let mut generic_name: Option<String> = None;
    let mut no_display = false;
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        }
        if trimmed.starts_with('[') && trimmed != "[Desktop Entry]" {
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
                name = Some(val.to_string());
            }
        } else if let Some(val) = trimmed.strip_prefix("Exec=") {
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
        } else if let Some(val) = trimmed.strip_prefix("Keywords=") {
            let kw = val.trim().to_string();
            if !kw.is_empty() {
                keywords = Some(kw);
            }
        } else if let Some(val) = trimmed.strip_prefix("GenericName=") {
            let gn = val.trim().to_string();
            if !gn.is_empty() {
                generic_name = Some(gn);
            }
        } else if trimmed == "NoDisplay=true" {
            no_display = true;
        }
    }

    if no_display {
        return None;
    }

    let name = name?;
    let exec = exec?;

    // Resolve icon to absolute path
    let icon_path = icon
        .as_deref()
        .and_then(|i| resolve_icon(i, icon_theme));

    Some(DesktopApp {
        name,
        exec,
        icon: icon.clone(),
        path: path.to_string_lossy().to_string(),
        keywords,
        generic_name,
        icon_path,
    })
}

// ─── Icon Resolution ─────────────────────────────────────

/// Resolve an icon name (or absolute path) to a real file on disk.
#[cfg(target_os = "linux")]
fn resolve_icon(icon_name: &str, theme: &str) -> Option<String> {
    // Already an absolute path
    if icon_name.starts_with('/') {
        if std::path::Path::new(icon_name).exists() {
            return Some(icon_name.to_string());
        }
        return None;
    }

    // Theme directories to search (in priority order)
    let icon_bases: Vec<String> = vec![
        format!("/usr/share/icons/{}", theme),
        "/usr/share/icons/hicolor".to_string(),
        "/usr/share/icons/Adwaita".to_string(),
        "/usr/share/icons/breeze".to_string(),
    ];

    // Preferred sizes (bigger = sharper for 32×32 display, but prefer 48 sweet-spot)
    let sizes = [
        "48x48", "64x64", "scalable", "32x32", "256x256", "128x128", "96x96", "24x24",
        "22x22", "16x16",
    ];
    let categories = ["apps", "categories", "mimetypes"];
    let extensions = ["png", "svg", "xpm"];

    for base in &icon_bases {
        for size in &sizes {
            for cat in &categories {
                for ext in &extensions {
                    let path = format!("{}/{}/{}/{}.{}", base, size, cat, icon_name, ext);
                    if std::path::Path::new(&path).exists() {
                        return Some(path);
                    }
                }
            }
        }
    }

    // Fallback: /usr/share/pixmaps
    for ext in &extensions {
        let path = format!("/usr/share/pixmaps/{}.{}", icon_name, ext);
        if std::path::Path::new(&path).exists() {
            return Some(path);
        }
    }

    None
}

/// Detect the active GTK icon theme name.
#[cfg(target_os = "linux")]
fn detect_icon_theme() -> String {
    // 1. GTK3 user settings
    if let Some(home) = dirs::home_dir() {
        let gtk3 = home.join(".config/gtk-3.0/settings.ini");
        if let Ok(content) = std::fs::read_to_string(&gtk3) {
            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("gtk-icon-theme-name") {
                    let val = rest
                        .trim_start_matches(|c: char| c == '=' || c.is_whitespace())
                        .trim();
                    if !val.is_empty() {
                        return val.to_string();
                    }
                }
            }
        }
    }

    // 2. gsettings (GNOME / GTK-based DEs)
    if let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "icon-theme"])
        .output()
    {
        if output.status.success() {
            let val = String::from_utf8_lossy(&output.stdout)
                .trim()
                .trim_matches('\'')
                .to_string();
            if !val.is_empty() {
                return val;
            }
        }
    }

    // 3. Fallback
    "hicolor".to_string()
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

    // System-wide Start Menu
    if let Ok(val) = std::env::var("ProgramData") {
        let dir = PathBuf::from(val).join("Microsoft\\Windows\\Start Menu\\Programs");
        if dir.exists() {
            search_dirs.push(dir);
        }
    }

    // Per-user Start Menu
    if let Ok(val) = std::env::var("APPDATA") {
        let dir = PathBuf::from(val).join("Microsoft\\Windows\\Start Menu\\Programs");
        if dir.exists() {
            search_dirs.push(dir);
        }
    }

    // User Desktop
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
                    icon_path: None, // Windows icons embedded in exe — not resolvable as files
                    keywords: None,
                    generic_name: None,
                });
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn parse_lnk_file(path: &PathBuf) -> Option<DesktopApp> {
    let name = path.file_stem().and_then(|s| s.to_str())?.to_string();

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
        exec: path.to_string_lossy().to_string(),
        icon: None,
        path: path.to_string_lossy().to_string(),
        keywords: None,
        generic_name: None,
        icon_path: None,
    })
}

/// Launch an application from a .lnk shortcut or exe path (Windows).
#[cfg(target_os = "windows")]
pub fn launch_app(exec_cmd: &str) -> Result<(), String> {
    open::that(exec_cmd).map_err(|e| format!("Failed to launch {}: {}", exec_cmd, e))
}
