//! Search in the text of every entry.

use crate::text::shorten;
use chrono::NaiveDate;

/// One entry that contains the text searched for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub date: NaiveDate,
    /// The line around the first match.
    pub snippet: String,
    /// How many times the text appears in the entry.
    pub count: usize,
}

/// Most results returned: a search for a common word should not build a list nobody reads.
pub const MAX_HITS: usize = 500;

fn fold(s: &str) -> String {
    s.chars().flat_map(char::to_lowercase).collect()
}

/// Entries containing every word of `query`, ignoring case; newest first.
///
/// Every word must appear somewhere in the entry; the snippet is the first line holding the
/// first word.
pub fn search<'a>(entries: impl IntoIterator<Item = (NaiveDate, &'a str)>, query: &str) -> Vec<Hit> {
    let words: Vec<String> = query.split_whitespace().map(fold).collect();
    let Some(first) = words.first() else {
        return Vec::new();
    };
    let mut hits: Vec<Hit> = entries
        .into_iter()
        .filter_map(|(date, text)| {
            let folded = fold(text);
            if !words.iter().all(|w| folded.contains(w.as_str())) {
                return None;
            }
            let count = folded.matches(first.as_str()).count();
            let line = text.lines().find(|l| fold(l).contains(first.as_str())).unwrap_or_default();
            Some(Hit { date, snippet: shorten(line.trim(), 120), count })
        })
        .collect();
    hits.sort_by_key(|h| std::cmp::Reverse(h.date));
    hits.truncate(MAX_HITS);
    hits
}
