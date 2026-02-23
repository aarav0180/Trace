/// Document Chat (RAG-Lite): Read a file's content and chat about it via LLM.
use crate::llm::LlmClient;
use crate::settings::Settings;
use std::path::Path;

const MAX_CONTENT_CHARS: usize = 100_000;

/// Return the safe maximum character count for file content based on the active model.
/// Prevents context-length overflow errors on small-context models like Gemma / Mistral 7B.
fn model_context_chars(model: &str) -> usize {
    let m = model.to_lowercase();

    // Very small context (~4K tokens): Phi-3 Mini 4K
    if m.contains("phi-3-mini") || m.contains("phi-3.5-mini") || m.contains("4k-instruct") {
        return 10_000;
    }

    // Small context (~8K tokens): Mistral 7B, Llama 8B, Gemma variants
    if m.contains("mistral-7b")
        || m.contains("mistral/mistral-7b")
        || m.contains("llama-3.1-8b")
        || m.contains("llama-3-8b")
        || m.contains("meta-llama-3.1-8b")
        || m.contains("gemma-2-9b")
        || m.contains("gemma-3")
    {
        return 20_000;
    }

    // Medium context (~16K–32K): Mixtral, Mistral Small, CodeLlama
    if m.contains("mixtral") || m.contains("mistral-small") || m.contains("codellama") {
        return 50_000;
    }

    // Large context (128K+): GPT-4o, DeepSeek, Qwen 72B, Claude, Gemini
    if m.contains("gpt-4o")
        || m.contains("o3-mini")
        || m.contains("o1")
        || m.contains("deepseek")
        || m.contains("qwen-2.5-72b")
        || m.contains("qwen/qwen-2.5")
        || m.contains("claude")
        || m.contains("gemini")
    {
        return 100_000;
    }

    // Default: conservative ~16K context
    20_000
}

/// Read a file's text content. Supports plain text, code, markdown, and PDF.
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

    // Detect PDF by extension and extract text
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let content = if ext == "pdf" {
        let bytes = std::fs::read(p)
            .map_err(|e| format!("Cannot read PDF file: {}", e))?;
        pdf_extract::extract_text_from_mem(&bytes)
            .map_err(|e| format!(
                "Could not extract text from PDF (it may be a scanned/image-only document): {}",
                e
            ))?
    } else {
        std::fs::read_to_string(p)
            .map_err(|e| format!("Cannot read file (may be binary): {}", e))?
    };

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

    // Trim content to the model's safe context window
    let limit = model_context_chars(&settings.active_model);
    let trimmed_content: String;
    let content_for_prompt = if file_content.len() > limit {
        // Find a clean UTF-8 boundary
        let boundary = file_content
            .char_indices()
            .map(|(i, _)| i)
            .take_while(|&i| i <= limit)
            .last()
            .unwrap_or(limit);
        trimmed_content = format!(
            "{}\n\n[... content trimmed to {} chars — {} chars total. Ask about a specific section for more detail.]",
            &file_content[..boundary],
            limit,
            file_content.len()
        );
        &trimmed_content
    } else {
        file_content
    };

    let system = format!(
        "You are an expert code and document analyst. The user has opened the file '{}'. \
         Below is the file's content. Answer the user's question about this file concisely and accurately.\n\n\
         --- FILE CONTENT ---\n{}\n--- END FILE CONTENT ---",
        file_name, content_for_prompt
    );

    llm.prompt(settings, &system, question).await
}
