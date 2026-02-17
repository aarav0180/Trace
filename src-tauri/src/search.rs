use crate::indexer::FileIndex;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub name: String,
    pub path: String,
    pub kind: crate::indexer::EntryKind,
    pub score: i64,
    pub matched_indices: Vec<usize>,
}

/// Perform a fuzzy search over the index. Returns top results sorted by score.
pub async fn fuzzy_search(
    index: &FileIndex,
    query: &str,
    max_results: usize,
) -> Vec<SearchResult> {
    if query.is_empty() {
        return Vec::new();
    }

    let idx = index.read().await;
    let matcher = SkimMatcherV2::default();

    let mut results: Vec<SearchResult> = idx
        .iter()
        .filter_map(|entry| {
            let (score, indices) = matcher.fuzzy_indices(&entry.name, query)?;

            // Minimum score threshold to avoid garbage results
            if score < 10 {
                return None;
            }

            Some(SearchResult {
                name: entry.name.clone(),
                path: entry.path.clone(),
                kind: entry.kind.clone(),
                score,
                matched_indices: indices,
            })
        })
        .collect();

    // Sort by score descending
    results.sort_unstable_by(|a, b| b.score.cmp(&a.score));
    results.truncate(max_results);
    results
}
