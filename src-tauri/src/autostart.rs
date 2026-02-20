/// System shortcut registration for Trace.
///
/// Registers an OS/DE-level keyboard shortcut to LAUNCH the Trace binary.
/// Combined with `tauri-plugin-single-instance`, pressing the shortcut:
///   1. If Trace is NOT running → launches it
///   2. If Trace IS running → toggles window visibility (spotlight-like UX)
///
/// Shortcut preference order: Super+T → Super+F → Super+J → Super+Y → ...
///
/// Supported environments:
///   Linux GNOME/Ubuntu/Pop/Budgie/Cinnamon: gsettings custom keybinding
///   Linux KDE/Plasma: kwriteconfig5/6
///   Linux i3: appends bindsym to ~/.config/i3/config
///   Linux Sway: appends bindsym to ~/.config/sway/config
///   Linux Hyprland: appends bind to ~/.config/hypr/hyprland.conf
///   Linux XFCE: xfconf-query
///   Windows: Start Menu .lnk with Ctrl+Alt+T hotkey

use std::path::PathBuf;
use std::process::Command;

/// Candidate keys in preference order.
const KEY_CANDIDATES: &[&str] = &["f", "j", "y", "k", "g", "b", "n"];

// ═══════════════════════════════════════════════
//  Public API
// ═══════════════════════════════════════════════

/// Register a system keyboard shortcut that launches Trace.
/// Returns a human-readable string like "Super+T" describing what was registered.
pub fn register_system_shortcut(exe_path: &str) -> String {
    // Clean up old autostart .desktop from previous versions
    cleanup_old_autostart();

    match do_register(exe_path) {
        Ok(shortcut) => {
            println!("[trace][shortcut] ✓ System shortcut: {}", shortcut);
            save_shortcut_info(&shortcut);
            shortcut
        }
        Err(e) => {
            eprintln!("[trace][shortcut] ✗ Failed: {}", e);
            "(none)".to_string()
        }
    }
}

/// Install a .desktop / Start Menu entry so Trace shows in app launchers.
pub fn install_desktop_entry(exe_path: &str) {
    if let Err(e) = do_install_entry(exe_path) {
        eprintln!("[trace][shortcut] Desktop entry error: {}", e);
    }
}

/// Save which shortcut was registered so the UI can display it later.
fn save_shortcut_info(shortcut: &str) {
    let dir = dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("trace");
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("shortcut"), shortcut);
}

/// Remove the old autostart .desktop entry from previous versions.
fn cleanup_old_autostart() {
    if let Some(config) = dirs::config_dir() {
        let old = config.join("autostart").join("trace.desktop");
        if old.exists() {
            let _ = std::fs::remove_file(&old);
            println!("[trace][shortcut] Cleaned up old autostart entry");
        }
    }
}

// ═══════════════════════════════════════════════
//  LINUX
// ═══════════════════════════════════════════════

#[cfg(target_os = "linux")]
fn do_register(exe_path: &str) -> Result<String, String> {
    let de = detect_de();
    println!("[trace][shortcut] Detected environment: {}", de);

    match de.as_str() {
        "gnome" | "cinnamon" => register_gnome(exe_path),
        "kde" => register_kde(exe_path),
        "i3" => register_i3(exe_path),
        "sway" => register_sway(exe_path),
        "hyprland" => register_hyprland(exe_path),
        "xfce" => register_xfce(exe_path),
        _ => {
            // Try gsettings (GNOME-compatible) as fallback
            register_gnome(exe_path).or_else(|gnome_err| {
                eprintln!("[trace][shortcut] gsettings fallback failed: {}", gnome_err);
                Err(format!(
                    "Unknown DE '{}'. Please manually bind a shortcut to: {}",
                    de, exe_path
                ))
            })
        }
    }
}

/// Detect the desktop environment / window manager.
#[cfg(target_os = "linux")]
fn detect_de() -> String {
    // Check WM-specific env vars first (most reliable)
    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        return "hyprland".to_string();
    }
    if std::env::var("SWAYSOCK").is_ok() {
        return "sway".to_string();
    }
    if std::env::var("I3SOCK").is_ok() {
        return "i3".to_string();
    }

    // Parse XDG_CURRENT_DESKTOP (can be "ubuntu:GNOME", "KDE", etc.)
    let xdg = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();
    let session = std::env::var("DESKTOP_SESSION")
        .unwrap_or_default()
        .to_lowercase();

    if xdg.contains("gnome") || xdg.contains("unity") || xdg.contains("budgie")
        || xdg.contains("pop") || xdg.contains("cosmic") || session.contains("gnome")
    {
        return "gnome".to_string();
    }
    if xdg.contains("kde") || xdg.contains("plasma") {
        return "kde".to_string();
    }
    if xdg.contains("cinnamon") {
        return "cinnamon".to_string();
    }
    if xdg.contains("xfce") {
        return "xfce".to_string();
    }
    if xdg.contains("i3") || session.contains("i3") {
        return "i3".to_string();
    }
    if xdg.contains("sway") {
        return "sway".to_string();
    }
    if xdg.contains("hyprland") {
        return "hyprland".to_string();
    }

    "unknown".to_string()
}

// ─── GNOME / Cinnamon ──────────────────────────

#[cfg(target_os = "linux")]
fn register_gnome(exe_path: &str) -> Result<String, String> {
    let key = find_free_gnome_key();
    let binding = format!("<Super>{}", key);
    let display = format!("Super+{}", key.to_uppercase());

    let our_path = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/trace/";
    let base = "org.gnome.settings-daemon.plugins.media-keys";
    let custom = format!("{}.custom-keybinding:{}", base, our_path);

    // Read existing custom keybinding list
    let output = Command::new("gsettings")
        .args(["get", base, "custom-keybindings"])
        .output()
        .map_err(|e| format!("gsettings not available: {}", e))?;

    if !output.status.success() {
        return Err("gsettings schema not found — probably not GNOME".to_string());
    }

    let existing = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Build updated list with our path included
    let new_list = if existing.contains(our_path) {
        existing // already registered
    } else if existing == "@as []" || existing == "[]" || existing.is_empty() {
        format!("['{}']", our_path)
    } else {
        // Append: ['/a/', '/b/'] → ['/a/', '/b/', '/trace/']
        let base_str = existing.trim_end_matches(']');
        format!("{}, '{}']", base_str, our_path)
    };

    // Set the list, then our keybinding properties
    gsettings_set(base, "custom-keybindings", &new_list)?;
    gsettings_set(&custom, "name", "Trace")?;
    gsettings_set(&custom, "command", exe_path)?;
    gsettings_set(&custom, "binding", &binding)?;

    Ok(display)
}

/// Find a Super+KEY that isn't already bound in GNOME.
#[cfg(target_os = "linux")]
fn find_free_gnome_key() -> String {
    let mut taken: Vec<String> = Vec::new();

    let schemas = [
        "org.gnome.desktop.wm.keybindings",
        "org.gnome.shell.keybindings",
        "org.gnome.settings-daemon.plugins.media-keys",
        "org.gnome.mutter.keybindings",
        "org.gnome.mutter.wayland.keybindings",
    ];

    for schema in schemas {
        if let Ok(out) = Command::new("gsettings")
            .args(["list-recursively", schema])
            .output()
        {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
                for line in text.lines() {
                    if line.contains("custom-keybindings") {
                        continue;
                    }
                    // Extract keys from <super>X patterns
                    let mut rest = line;
                    while let Some(pos) = rest.find("<super>") {
                        let after = &rest[pos + 7..];
                        if let Some(ch) = after.chars().next() {
                            if ch.is_alphanumeric() {
                                taken.push(ch.to_string());
                            }
                        }
                        rest = after;
                    }
                }
            }
        }
    }

    // Also check dconf dump for extension-registered shortcuts
    // (GNOME extensions use schemas not covered by the list above)
    if let Ok(out) = Command::new("dconf")
        .args(["dump", "/org/gnome/"])
        .output()
    {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            let mut rest: &str = &text;
            while let Some(pos) = rest.find("<super>") {
                let after: &str = &rest[pos + 7..];
                if let Some(ch) = after.chars().next() {
                    if ch.is_alphanumeric() && !taken.contains(&ch.to_string()) {
                        taken.push(ch.to_string());
                    }
                }
                rest = after;
            }
        }
    }

    println!("[trace][shortcut] GNOME taken Super keys: {:?}", taken);

    for key in KEY_CANDIDATES {
        if !taken.contains(&key.to_string()) {
            return key.to_string();
        }
    }

    println!("[trace][shortcut] All preferred keys taken, defaulting to 't'");
    "t".to_string()
}

#[cfg(target_os = "linux")]
fn gsettings_set(schema: &str, key: &str, value: &str) -> Result<(), String> {
    let out = Command::new("gsettings")
        .args(["set", schema, key, value])
        .output()
        .map_err(|e| format!("gsettings failed: {}", e))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("gsettings error: {}", stderr.trim()));
    }
    Ok(())
}

// ─── KDE / Plasma ─────────────────────────────

#[cfg(target_os = "linux")]
fn register_kde(_exe_path: &str) -> Result<String, String> {
    // KDE shortcuts reference the .desktop file by name — install_desktop_entry() creates it.
    let tool = find_kwriteconfig()?;
    let key = find_free_kde_key();
    let display = format!("Super+{}", key.to_uppercase());
    let meta_key = format!("Meta+{}", key.to_uppercase());

    let out = Command::new(&tool)
        .args([
            "--file",
            "kglobalshortcutsrc",
            "--group",
            "trace.desktop",
            "--key",
            "_launch",
            &format!("{},{},Launch Trace", meta_key, meta_key),
        ])
        .output()
        .map_err(|e| format!("{} failed: {}", tool, e))?;

    if !out.status.success() {
        return Err(format!(
            "{} error: {}",
            tool,
            String::from_utf8_lossy(&out.stderr)
        ));
    }

    // Reload shortcuts (best-effort)
    let _ = Command::new("dbus-send")
        .args([
            "--session",
            "--type=signal",
            "--dest=org.kde.KGlobalAccel",
            "/kglobalaccel",
            "org.kde.KGlobalAccel.yourShortcutsChanged",
        ])
        .output();

    Ok(display)
}

/// Find a Super+KEY that isn't already bound in KDE Plasma.
#[cfg(target_os = "linux")]
fn find_free_kde_key() -> String {
    // Read kglobalshortcutsrc which stores all KDE global shortcuts
    let config_path = dirs::config_dir()
        .unwrap_or_default()
        .join("kglobalshortcutsrc");
    let content = std::fs::read_to_string(&config_path)
        .unwrap_or_default()
        .to_lowercase();

    // Also check kwinrc for KWin window management shortcuts
    let kwinrc = dirs::config_dir()
        .unwrap_or_default()
        .join("kwinrc");
    let kwin_content = std::fs::read_to_string(&kwinrc)
        .unwrap_or_default()
        .to_lowercase();

    let combined = format!("{}\n{}", content, kwin_content);

    for key in KEY_CANDIDATES {
        // KDE stores shortcuts as Meta+T, Meta+Shift+T, etc.
        let pattern = format!("meta+{}", key);
        // Make sure we match exactly Meta+KEY and not Meta+Shift+KEY etc.
        let is_taken = combined.lines().any(|line| {
            line.contains(&pattern) && {
                // Verify it's a real binding, not just "none" or empty
                if let Some((_key_part, val)) = line.split_once('=') {
                    let val = val.trim();
                    val.contains(&pattern) && !val.starts_with("none") && !val.is_empty()
                } else {
                    false
                }
            }
        });
        if !is_taken {
            return key.to_string();
        }
    }

    println!("[trace][shortcut] All preferred keys taken in KDE, defaulting to 't'");
    "t".to_string()
}

#[cfg(target_os = "linux")]
fn find_kwriteconfig() -> Result<String, String> {
    for tool in ["kwriteconfig6", "kwriteconfig5"] {
        if Command::new("which")
            .arg(tool)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(tool.to_string());
        }
    }
    Err("Neither kwriteconfig6 nor kwriteconfig5 found".to_string())
}

// ─── i3 ───────────────────────────────────────

#[cfg(target_os = "linux")]
fn register_i3(exe_path: &str) -> Result<String, String> {
    let config = find_config_file(&[".config/i3/config", ".i3/config"], "i3")?;
    let content = std::fs::read_to_string(&config).unwrap_or_default();

    if content.contains("# Trace launcher") {
        let old_shortcut = read_wm_shortcut(&content, "$mod+");
        let old_key = old_shortcut.split('+').last().unwrap_or("T").to_lowercase();
        let content_without_trace = remove_trace_block(&content);
        let best_key = find_free_wm_key(&content_without_trace, "$mod+");

        if best_key == old_key {
            return Ok(old_shortcut);
        }

        println!(
            "[trace][shortcut] Conflict: Super+{} is taken, switching to Super+{}",
            old_key.to_uppercase(), best_key.to_uppercase()
        );
        std::fs::write(&config, content_without_trace)
            .map_err(|e| format!("Failed to rewrite config: {}", e))?;
        let display = format!("Super+{}", best_key.to_uppercase());
        let block = format!(
            "\n# Trace launcher\nbindsym $mod+{} exec --no-startup-id {}\n",
            best_key, exe_path
        );
        append_to_file(&config, &block)?;
        let _ = Command::new("i3-msg").arg("reload").output();
        return Ok(display);
    }

    let key = find_free_wm_key(&content, "$mod+");
    let display = format!("Super+{}", key.to_uppercase());

    let block = format!(
        "\n# Trace launcher\nbindsym $mod+{} exec --no-startup-id {}\n",
        key, exe_path
    );
    append_to_file(&config, &block)?;

    let _ = Command::new("i3-msg").arg("reload").output();

    Ok(display)
}

// ─── Sway ─────────────────────────────────────

#[cfg(target_os = "linux")]
fn register_sway(exe_path: &str) -> Result<String, String> {
    let config = find_config_file(&[".config/sway/config"], "sway")?;
    let content = std::fs::read_to_string(&config).unwrap_or_default();

    if content.contains("# Trace launcher") {
        let old_shortcut = read_wm_shortcut(&content, "$mod+");
        let old_key = old_shortcut.split('+').last().unwrap_or("T").to_lowercase();
        let content_without_trace = remove_trace_block(&content);
        let best_key = find_free_wm_key(&content_without_trace, "$mod+");

        if best_key == old_key {
            return Ok(old_shortcut);
        }

        println!(
            "[trace][shortcut] Conflict: Super+{} is taken, switching to Super+{}",
            old_key.to_uppercase(), best_key.to_uppercase()
        );
        std::fs::write(&config, content_without_trace)
            .map_err(|e| format!("Failed to rewrite config: {}", e))?;
        let display = format!("Super+{}", best_key.to_uppercase());
        let block = format!(
            "\n# Trace launcher\nbindsym $mod+{} exec {}\n",
            best_key, exe_path
        );
        append_to_file(&config, &block)?;
        let _ = Command::new("swaymsg").arg("reload").output();
        return Ok(display);
    }

    let key = find_free_wm_key(&content, "$mod+");
    let display = format!("Super+{}", key.to_uppercase());

    let block = format!(
        "\n# Trace launcher\nbindsym $mod+{} exec {}\n",
        key, exe_path
    );
    append_to_file(&config, &block)?;

    let _ = Command::new("swaymsg").arg("reload").output();

    Ok(display)
}

// ─── Hyprland ─────────────────────────────────

#[cfg(target_os = "linux")]
fn register_hyprland(exe_path: &str) -> Result<String, String> {
    let config = find_config_file(&[".config/hypr/hyprland.conf"], "Hyprland")?;
    let content = std::fs::read_to_string(&config).unwrap_or_default();

    if content.contains("# Trace launcher") {
        let old_shortcut = read_hyprland_shortcut(&content);
        // Extract the key letter from "Super+T" → "t"
        let old_key = old_shortcut
            .split('+').last().unwrap_or("t")
            .to_lowercase();

        // Check if the old key now conflicts with another (non-Trace) binding.
        // Build content WITHOUT the Trace block for conflict checking.
        let content_without_trace = remove_trace_block(&content);
        let best_key = find_free_hyprland_key(&content_without_trace);

        if best_key == old_key {
            // No conflict — keep existing binding
            return Ok(old_shortcut);
        }

        // Conflict detected — remove old block, re-register with a free key
        println!(
            "[trace][shortcut] Conflict: Super+{} is taken, switching to Super+{}",
            old_key.to_uppercase(),
            best_key.to_uppercase()
        );
        std::fs::write(&config, content_without_trace)
            .map_err(|e| format!("Failed to rewrite config: {}", e))?;
        // Fall through to append the new binding below
        let display = format!("Super+{}", best_key.to_uppercase());
        let block = format!(
            "\n# Trace launcher\nbind = $mainMod, {}, exec, {}\n",
            best_key.to_uppercase(),
            exe_path
        );
        append_to_file(&config, &block)?;
        return Ok(display);
    }

    let key = find_free_hyprland_key(&content);
    let display = format!("Super+{}", key.to_uppercase());

    let block = format!(
        "\n# Trace launcher\nbind = $mainMod, {}, exec, {}\n",
        key.to_uppercase(),
        exe_path
    );
    append_to_file(&config, &block)?;

    // Hyprland hot-reloads on config file change

    Ok(display)
}

#[cfg(target_os = "linux")]
fn find_free_hyprland_key(content: &str) -> String {
    let lower = content.to_lowercase();
    for key in KEY_CANDIDATES {
        let has_binding = lower.lines().any(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('#') || !trimmed.starts_with("bind") {
                return false;
            }
            // Parse: bind[flags] = MODIFIERS, KEY, DISPATCHER[, PARAMS]
            let Some(after_eq) = trimmed.split_once('=').map(|(_, r)| r) else {
                return false;
            };
            let parts: Vec<&str> = after_eq.splitn(3, ',').collect();
            if parts.len() < 2 {
                return false;
            }
            let mods = parts[0].trim();
            let bound_key = parts[1].trim();

            // Only conflict if modifier is exactly Super/$mainMod (not Super+Shift etc.)
            let is_super_only = mods == "$mainmod"
                || mods == "super"
                || mods == "mod4"
                || mods == "$mod";
            is_super_only && bound_key == *key
        });
        if !has_binding {
            return key.to_string();
        }
    }
    "t".to_string()
}

#[cfg(target_os = "linux")]
fn read_hyprland_shortcut(content: &str) -> String {
    let mut found = false;
    for line in content.lines() {
        if line.contains("# Trace launcher") {
            found = true;
            continue;
        }
        if found && line.trim().starts_with("bind") {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                return format!("Super+{}", parts[1].trim().to_uppercase());
            }
        }
    }
    "Super+T".to_string()
}

// ─── XFCE ─────────────────────────────────────

#[cfg(target_os = "linux")]
fn register_xfce(exe_path: &str) -> Result<String, String> {
    let key = find_free_xfce_key();
    let display = format!("Super+{}", key.to_uppercase());

    let out = Command::new("xfconf-query")
        .args([
            "-c",
            "xfce4-keyboard-shortcuts",
            "-p",
            &format!("/commands/custom/<Super>{}", key),
            "-n",
            "-t",
            "string",
            "-s",
            exe_path,
        ])
        .output()
        .map_err(|e| format!("xfconf-query failed: {}", e))?;

    if !out.status.success() {
        return Err(format!(
            "xfconf-query error: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }

    Ok(display)
}

/// Find a Super+KEY that isn't already bound in XFCE.
#[cfg(target_os = "linux")]
fn find_free_xfce_key() -> String {
    if let Ok(out) = Command::new("xfconf-query")
        .args(["-c", "xfce4-keyboard-shortcuts", "-lv"])
        .output()
    {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            for key in KEY_CANDIDATES {
                let pattern = format!("<super>{}", key);
                if !text.contains(&pattern) {
                    return key.to_string();
                }
            }
        }
    }
    // Fallback: just use first candidate
    KEY_CANDIDATES[0].to_string()
}

// ─── Shared Linux helpers ─────────────────────

#[cfg(target_os = "linux")]
fn find_config_file(candidates: &[&str], wm_name: &str) -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("No home directory")?;
    for c in candidates {
        let path = home.join(c);
        if path.exists() {
            return Ok(path);
        }
    }
    Err(format!("{} config file not found", wm_name))
}

/// Find a Super+KEY that isn't used in an i3/sway config.
#[cfg(target_os = "linux")]
fn find_free_wm_key(content: &str, prefix: &str) -> String {
    let lower = content.to_lowercase();
    for key in KEY_CANDIDATES {
        // Build all equivalent binding patterns
        let patterns: Vec<String> = vec![
            format!("{}{}", prefix, key).to_lowercase(),   // $mod+t
            format!("mod4+{}", key),                        // mod4+t
        ];
        let is_taken = lower.lines().any(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                return false;
            }
            // Match bindsym, bindsym --release, bindcode, etc.
            if !trimmed.starts_with("bindsym") && !trimmed.starts_with("bindcode") {
                return false;
            }
            // Skip the command keyword and any --flags to find the actual binding
            let binding = trimmed
                .split_whitespace()
                .skip(1) // skip bindsym/bindcode
                .find(|p| !p.starts_with("--"))
                .unwrap_or("");
            patterns.iter().any(|p| binding == p.as_str())
        });
        if !is_taken {
            return key.to_string();
        }
    }
    "t".to_string()
}

/// Read back which shortcut we previously wrote into a WM config.
#[cfg(target_os = "linux")]
fn read_wm_shortcut(content: &str, prefix: &str) -> String {
    let mut found = false;
    for line in content.lines() {
        if line.contains("# Trace launcher") {
            found = true;
            continue;
        }
        if found && line.trim().starts_with("bindsym") {
            let parts: Vec<&str> = line.trim().split_whitespace().collect();
            if parts.len() >= 2 {
                if let Some(key) = parts[1].strip_prefix(prefix) {
                    return format!("Super+{}", key.to_uppercase());
                }
            }
        }
    }
    "Super+T".to_string()
}

#[cfg(target_os = "linux")]
fn append_to_file(path: &PathBuf, content: &str) -> Result<(), String> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .map_err(|e| format!("Cannot open {}: {}", path.display(), e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Write failed: {}", e))?;
    Ok(())
}

/// Remove the "# Trace launcher" block (comment + the binding line after it)
/// from a WM config string, preserving everything else.
#[cfg(target_os = "linux")]
fn remove_trace_block(content: &str) -> String {
    let mut result = String::new();
    let mut skip_next = false;
    for line in content.lines() {
        if line.trim() == "# Trace launcher" {
            skip_next = true;
            continue;
        }
        if skip_next {
            skip_next = false;
            // Skip the binding line that follows the comment
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }
    // Trim trailing blank lines that may have accumulated
    result.trim_end_matches('\n').to_string() + "\n"
}

/// Install a .desktop entry in ~/.local/share/applications/
#[cfg(target_os = "linux")]
fn do_install_entry(exe_path: &str) -> Result<(), String> {
    let app_dir = dirs::data_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_default()
                .join(".local/share")
        })
        .join("applications");

    std::fs::create_dir_all(&app_dir)
        .map_err(|e| format!("Cannot create applications dir: {}", e))?;

    let desktop = app_dir.join("trace.desktop");
    let content = format!(
        r#"[Desktop Entry]
Type=Application
Name=Trace
Comment=The Intelligence Layer for your OS — instant search, app launcher, NLP shell
Exec={exe_path}
Icon=trace
Terminal=false
StartupNotify=true
Categories=Utility;System;
Keywords=search;launcher;files;ai;
"#
    );

    std::fs::write(&desktop, content)
        .map_err(|e| format!("Cannot write desktop entry: {}", e))?;

    println!("[trace][shortcut] Desktop entry: {}", desktop.display());
    Ok(())
}

// ═══════════════════════════════════════════════
//  WINDOWS
// ═══════════════════════════════════════════════

/// On Windows, create a Start Menu .lnk with Ctrl+Alt+T hotkey.
/// Windows doesn't support Win+KEY system shortcuts without a running process,
/// so Ctrl+Alt+T via .lnk Hotkey is the best native approach.
#[cfg(target_os = "windows")]
fn do_register(exe_path: &str) -> Result<String, String> {
    let display = "Ctrl+Alt+T".to_string();

    let start_menu = match std::env::var("APPDATA") {
        Ok(appdata) => {
            PathBuf::from(appdata).join("Microsoft\\Windows\\Start Menu\\Programs")
        }
        Err(_) => return Err("APPDATA not set".to_string()),
    };

    std::fs::create_dir_all(&start_menu)
        .map_err(|e| format!("Cannot create Start Menu dir: {}", e))?;

    let shortcut_path = start_menu
        .join("Trace.lnk")
        .to_string_lossy()
        .replace('/', "\\");
    let exe_escaped = exe_path.replace('/', "\\");

    let ps = format!(
        r#"$ws = New-Object -ComObject WScript.Shell; $sc = $ws.CreateShortcut('{}'); $sc.TargetPath = '{}'; $sc.Description = 'The Intelligence Layer for your OS'; $sc.Hotkey = 'Ctrl+Alt+T'; $sc.Save()"#,
        shortcut_path, exe_escaped
    );

    let out = Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps])
        .output()
        .map_err(|e| format!("PowerShell failed: {}", e))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("Shortcut creation failed: {}", stderr));
    }

    println!(
        "[trace][shortcut] Start Menu shortcut created: {}",
        shortcut_path
    );
    Ok(display)
}

/// On Windows, the Start Menu .lnk already serves as the app entry.
#[cfg(target_os = "windows")]
fn do_install_entry(_exe_path: &str) -> Result<(), String> {
    Ok(())
}