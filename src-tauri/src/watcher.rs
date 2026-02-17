use crate::indexer::{EntryKind, FileEntry, FileIndex};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::time::SystemTime;

/// Start a filesystem watcher on the given roots.
/// Updates the shared index in real-time as files are created, modified, or removed.
pub async fn start_watcher(index: FileIndex, roots: Vec<PathBuf>) {
    let index_clone = index.clone();

    tokio::task::spawn_blocking(move || {
        let (tx, rx) = std::sync::mpsc::channel::<notify::Result<Event>>();

        let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("[trace][watcher] Failed to create watcher: {}", e);
                return;
            }
        };

        for root in &roots {
            if let Err(e) = watcher.watch(root, RecursiveMode::Recursive) {
                eprintln!("[trace][watcher] Failed to watch {:?}: {}", root, e);
            }
        }

        println!("[trace][watcher] Watching {} roots for changes", roots.len());

        // Block this thread and process events
        for result in rx {
            match result {
                Ok(event) => {
                    let idx = index_clone.clone();
                    handle_event(idx, event);
                }
                Err(e) => {
                    eprintln!("[trace][watcher] Error: {}", e);
                }
            }
        }
    });
}

fn handle_event(index: FileIndex, event: Event) {
    // We use a blocking approach to update the index from the watcher thread
    let rt = tokio::runtime::Handle::current();

    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            for path in event.paths {
                let metadata = match std::fs::metadata(&path) {
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

                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                let path_str = path.to_string_lossy().to_string();

                let entry = FileEntry {
                    name,
                    path: path_str.clone(),
                    kind,
                    size: metadata.len(),
                    modified,
                };

                let index = index.clone();
                rt.spawn(async move {
                    let mut idx = index.write().await;
                    // Remove old entry if it exists, then insert new
                    idx.retain(|e| e.path != path_str);
                    idx.push(entry);
                });
            }
        }
        EventKind::Remove(_) => {
            for path in event.paths {
                let path_str = path.to_string_lossy().to_string();
                let index = index.clone();
                rt.spawn(async move {
                    let mut idx = index.write().await;
                    idx.retain(|e| e.path != path_str);
                });
            }
        }
        _ => {}
    }
}
