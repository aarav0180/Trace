use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use walkdir::WalkDir;

/// Represents a single indexed entry (file or app).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub kind: EntryKind,
    pub size: u64,
    pub modified: u64, // unix timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntryKind {
    File,
    Directory,
    App,
}

/// The shared file index — an in-memory store behind an async RwLock.
pub type FileIndex = Arc<RwLock<Vec<FileEntry>>>;

/// Create a new empty file index.
pub fn new_index() -> FileIndex {
    Arc::new(RwLock::new(Vec::new()))
}

/// Scan the given root directories and populate the index.
/// Runs on a blocking thread pool to avoid starving the async runtime.
pub async fn build_index(index: FileIndex, roots: Vec<PathBuf>) {
    let entries = tokio::task::spawn_blocking(move || {
        let mut results: Vec<FileEntry> = Vec::with_capacity(100_000);

        for root in &roots {
            let walker = WalkDir::new(root)
                .follow_links(false)
                .max_depth(12) // reasonable depth limit
                .into_iter()
                .filter_entry(|e| !is_hidden(e));

            for entry in walker.flatten() {
                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let modified = metadata
                    .modified()
                    .unwrap_or(SystemTime::UNIX_EPOCH)
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let kind = if metadata.is_dir() {
                    EntryKind::Directory
                } else {
                    EntryKind::File
                };

                let name = entry
                    .file_name()
                    .to_string_lossy()
                    .to_string();

                results.push(FileEntry {
                    name,
                    path: entry.path().to_string_lossy().to_string(),
                    kind,
                    size: metadata.len(),
                    modified,
                });
            }
        }

        results
    })
    .await
    .unwrap_or_default();

    let mut idx = index.write().await;
    *idx = entries;
    println!("[trace] Indexed {} entries", idx.len());
}

/// Returns true for hidden files/dirs (dotfiles) and common junk directories.
fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();

    // Always allow the root itself
    if entry.depth() == 0 {
        return false;
    }

    // Skip dotfiles, node_modules, target, .git, __pycache__, etc.
    name.starts_with('.')
        || name == "node_modules"
        || name == "target"
        || name == "__pycache__"
        || name == ".cache"
        || name == ".local/share/Trash"
        || name == "vendor"
        || name == "dist"
        || name == "build"
}
