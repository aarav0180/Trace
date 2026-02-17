/// NLP-to-Shell: Translates natural language into shell commands using an LLM.
use crate::llm::LlmClient;
use crate::settings::Settings;
use serde::Serialize;
use std::process::Command;

const SYSTEM_PROMPT: &str = r#"You are a Linux shell command translator. The user will describe what they want to do in plain English. You must respond with ONLY the exact shell command to execute — no explanation, no markdown, no code fences, just the raw command. If the task requires multiple commands, chain them with && or ;. Always use standard Linux utilities."#;

/// Dangerous command patterns that require extra confirmation.
const DANGEROUS_PATTERNS: &[&str] = &[
    "rm -rf",
    "rm -r /",
    "mkfs",
    "dd if=",
    ":(){",
    "chmod -R 777 /",
    "chown -R",
    "> /dev/sd",
    "wget | sh",
    "curl | sh",
];

#[derive(Debug, Clone, Serialize)]
pub struct ShellTranslation {
    pub command: String,
    pub is_dangerous: bool,
}

/// Translate natural language to a shell command via LLM.
pub async fn translate_to_command(
    llm: &LlmClient,
    settings: &Settings,
    natural_input: &str,
) -> Result<ShellTranslation, String> {
    let command = llm.prompt(settings, SYSTEM_PROMPT, natural_input).await?;

    // Clean up any residual formatting
    let command = command
        .trim()
        .trim_start_matches("```bash")
        .trim_start_matches("```sh")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
        .to_string();

    let is_dangerous = DANGEROUS_PATTERNS
        .iter()
        .any(|p| command.contains(p));

    Ok(ShellTranslation {
        command,
        is_dangerous,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct ShellOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Execute a shell command and return its output.
pub fn execute_command(cmd: &str) -> Result<ShellOutput, String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    Ok(ShellOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}
