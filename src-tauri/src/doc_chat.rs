/// Document Chat (RAG-Lite): Read a file's content and chat about it via LLM.
use crate::llm::LlmClient;
use crate::settings::Settings;
use std::path::Path;

const MAX_CONTENT_CHARS: usize = 100_000;

/// Read a file's text content. Supports plain text, code, markdown.
/// For very large files, truncates to MAX_CONTENT_CHARS.
pub fn read_file_content(path: &str) -> Result<String, String> {
    let p = Path::new(path);

    if !p.exists() {
        return Err(format!("File not found: {}", path));
    }

    // Check file size before reading
    let metadata = std::fs::metadata(p)
        .map_err(|e| format!("Cannot read file metadata: {}", e))?;

    if metadata.len() > 50_000_000 {
        // 50MB limit
        return Err("File is too large (>50MB) for chat mode".to_string());
    }

    let content = std::fs::read_to_string(p)
        .map_err(|e| format!("Cannot read file (may be binary): {}", e))?;

    // Truncate if needed
    if content.len() > MAX_CONTENT_CHARS {
        Ok(format!(
            "{}\n\n[... truncated — showing first {} characters of {} total]",
            &content[..MAX_CONTENT_CHARS],
            MAX_CONTENT_CHARS,
            content.len()
        ))
    } else {
        Ok(content)
    }
}

/// Send a question about a file to the LLM.
pub async fn chat_about_file(
    llm: &LlmClient,
    settings: &Settings,
    file_path: &str,
    file_content: &str,
    question: &str,
) -> Result<String, String> {
    let file_name = Path::new(file_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.to_string());

    let system = format!(
        "You are an expert code and document analyst. The user has opened the file '{}'. \
         Below is the file's content. Answer the user's question about this file concisely and accurately.\n\n\
         --- FILE CONTENT ---\n{}\n--- END FILE CONTENT ---",
        file_name, file_content
    );

    llm.prompt(settings, &system, question).await
}
