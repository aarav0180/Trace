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
    /// Resolved icon path (Apps only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_path: Option<String>,
    /// Human-readable category (Apps only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generic_name: Option<String>,
}

/// Perform a fuzzy search over the index. Returns top results sorted by score.
/// Matches against name, keywords, and generic_name; boosts App entries.
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
            // Primary: match on name
            let name_match = matcher.fuzzy_indices(&entry.name, query);

            // Secondary: match on keywords (semi-colon separated)
            let kw_score = entry
                .keywords
                .as_deref()
                .and_then(|kw| matcher.fuzzy_match(kw, query))
                .unwrap_or(0);

            // Secondary: match on generic name
            let gn_score = entry
                .generic_name
                .as_deref()
                .and_then(|gn| matcher.fuzzy_match(gn, query))
                .unwrap_or(0);

            // Must match at least one field
            let (name_score, indices) = match name_match {
                Some((s, i)) => (s, i),
                None if kw_score > 0 || gn_score > 0 => (0, vec![]),
                _ => return None,
            };

            let mut score = name_score.max(kw_score).max(gn_score);

            // Minimum score threshold
            if score < 10 {
                return None;
            }

            // Boost applications so they surface above similarly-named files
            if entry.kind == crate::indexer::EntryKind::App {
                score = (score as f64 * 1.3) as i64;
            }

            Some(SearchResult {
                name: entry.name.clone(),
                path: entry.path.clone(),
                kind: entry.kind.clone(),
                score,
                matched_indices: indices,
                icon_path: entry.icon_path.clone(),
                generic_name: entry.generic_name.clone(),
            })
        })
        .collect();

    // Sort by score descending
    results.sort_unstable_by(|a, b| b.score.cmp(&a.score));
    results.truncate(max_results);
    results
}
