/// NLP-to-Shell: Translates natural language into shell commands using an LLM.
use crate::llm::LlmClient;
use crate::settings::Settings;
use serde::Serialize;
use std::process::Command;

/// Collect runtime OS/environment context for injecting into the LLM prompt.
fn os_context() -> String {
    let arch = std::env::consts::ARCH;
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".to_string());

    #[cfg(target_os = "linux")]
    let os_name = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|c| {
            c.lines()
                .find(|l| l.starts_with("PRETTY_NAME="))
                .map(|l| {
                    l.trim_start_matches("PRETTY_NAME=")
                        .trim_matches('"')
                        .to_string()
                })
        })
        .unwrap_or_else(|| "Linux".to_string());

    #[cfg(target_os = "windows")]
    let os_name = "Windows".to_string();

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    let os_name = std::env::consts::OS.to_string();

    let shell = if cfg!(windows) {
        "cmd.exe".to_string()
    } else {
        std::env::var("SHELL")
            .map(|s| s.split('/').last().unwrap_or("bash").to_string())
            .unwrap_or_else(|_| "bash".to_string())
    };

    format!("{os_name} ({arch}), user: {username}, shell: {shell}")
}

/// Returns the OS-appropriate system prompt for the LLM shell translator.
fn system_prompt() -> String {
    let ctx = os_context();
    if cfg!(windows) {
        format!(
            "You are a Windows command-line translator running on {ctx}. \
             Translate the user's plain English request into ONLY the exact command — \
             no explanation, no markdown, no code fences, just the raw command. \
             Use cmd.exe or PowerShell syntax. Chain multiple steps with && or &."
        )
    } else {
        format!(
            "You are a Linux shell command translator running on {ctx}. \
             Translate the user's plain English request into ONLY the exact shell command — \
             no explanation, no markdown, no code fences, just the raw command. \
             Use standard Linux utilities. Chain multiple commands with && or ;."
        )
    }
}

/// Each entry is (pattern, human-readable reason).
/// The pattern is matched case-insensitively against the generated command.
fn dangerous_patterns() -> Vec<(&'static str, &'static str)> {
    let mut patterns: Vec<(&'static str, &'static str)> = vec![
        // Recursive force-delete variants (all common orderings)
        ("rm -rf",           "Recursively and forcefully deletes files/directories with no undo"),
        ("rm -fr",           "Recursively and forcefully deletes files/directories with no undo"),
        ("rm -r -f",         "Recursively and forcefully deletes files/directories with no undo"),
        ("rm -f -r",         "Recursively and forcefully deletes files/directories with no undo"),
        ("rm -r /",          "Targets the filesystem root — will destroy the entire system"),
        ("rm -rf /",         "Targets the filesystem root — will destroy the entire system"),
        ("rm -fr /",         "Targets the filesystem root — will destroy the entire system"),
        // Disk/filesystem operations
        ("mkfs",             "Formats a disk partition, erasing all data on it permanently"),
        ("dd if=",           "Low-level disk operation that can overwrite entire drives"),
        ("> /dev/sd",        "Writes raw data directly to a block device, destroying disk contents"),
        ("> /dev/nvme",      "Writes raw data directly to an NVMe drive, destroying its contents"),
        // Fork bomb
        (":(){",             "Fork bomb — will crash the system by exhausting all process slots"),
        // Permission nukes
        ("chmod -r 777 /",   "Recursively makes every file on the system world-writable"),
        ("chmod 777 /",      "Makes the filesystem root world-writable — major security risk"),
        ("chown -r root /",  "Changes ownership of every file on the system to root"),
        // Remote code execution
        ("wget | sh",        "Downloads and immediately executes unknown remote code"),
        ("curl | sh",        "Downloads and immediately executes unknown remote code"),
        ("curl | bash",      "Downloads and immediately executes unknown remote code"),
        ("wget | bash",      "Downloads and immediately executes unknown remote code"),
        // Dangerous redirects
        ("> /dev/null && rm","Combines output suppression with a delete, masking the damage"),
    ];
    if cfg!(windows) {
        patterns.extend_from_slice(&[
            ("del /s /q",                  "Silently deletes files and subdirectories recursively"),
            ("rd /s /q",                   "Silently removes a directory tree with no confirmation"),
            ("rmdir /s /q",                "Silently removes a directory tree with no confirmation"),
            ("format ",                    "Formats a drive, permanently erasing all data on it"),
            ("diskpart",                   "Low-level disk partition editor — can erase entire drives"),
            ("reg delete",                 "Deletes Windows registry keys, which can break the OS"),
            ("shutdown",                   "Shuts down or reboots the system immediately"),
            ("taskkill /f",                "Forcefully terminates processes, may corrupt running work"),
            ("remove-item -recurse -force","PowerShell equivalent of rm -rf — permanent deletion"),
        ]);
    }
    patterns
}

/// Check a command against all dangerous patterns (case-insensitive).
/// Returns Some(reason) if dangerous, None if safe.
fn check_dangerous(command: &str) -> Option<String> {
    let lower = command.to_lowercase();
    for (pattern, reason) in dangerous_patterns() {
        if lower.contains(pattern) {
            return Some(reason.to_string());
        }
    }
    None
}

#[derive(Debug, Clone, Serialize)]
pub struct ShellTranslation {
    pub command: String,
    pub is_dangerous: bool,
    pub danger_reason: String,
}

/// Translate natural language to a shell command via LLM.
pub async fn translate_to_command(
    llm: &LlmClient,
    settings: &Settings,
    natural_input: &str,
) -> Result<ShellTranslation, String> {
    let prompt = system_prompt();
    let command = llm.prompt(settings, &prompt, natural_input).await?;

    // Clean up any residual formatting
    let command = command
        .trim()
        .trim_start_matches("```bash")
        .trim_start_matches("```sh")
        .trim_start_matches("```powershell")
        .trim_start_matches("```cmd")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
        .to_string();

    let danger_reason = check_dangerous(&command).unwrap_or_default();
    let is_dangerous = !danger_reason.is_empty();

    Ok(ShellTranslation {
        command,
        is_dangerous,
        danger_reason,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct ShellOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Execute a shell command and return its output.
/// Uses `sh -c` on Linux and `cmd /C` on Windows.
pub fn execute_command(cmd: &str) -> Result<ShellOutput, String> {
    let output = if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", cmd])
            .output()
            .map_err(|e| format!("Failed to execute command: {}", e))?
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .map_err(|e| format!("Failed to execute command: {}", e))?
    };

    Ok(ShellOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}
