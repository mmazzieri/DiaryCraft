//! The text of an entry: front matter, body, word count.

/// Splits an optional YAML front matter (`---` on the first line, up to the next `---` line)
/// from the body. The front matter is returned without its fences; DiaryCraft keeps it as it is.
pub fn split_front_matter(text: &str) -> (Option<&str>, &str) {
    let rest = match text.strip_prefix("---\n").or_else(|| text.strip_prefix("---\r\n")) {
        Some(rest) => rest,
        None => return (None, text),
    };
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            let front = rest.get(..offset).unwrap_or_default();
            let body = rest.get(offset + line.len()..).unwrap_or_default();
            return (Some(front), body);
        }
        offset += line.len();
    }
    (None, text)
}

/// Words of the body, front matter excluded.
pub fn word_count(text: &str) -> usize {
    split_front_matter(text).1.split_whitespace().filter(|w| w.chars().any(char::is_alphanumeric)).count()
}

/// Whether there is anything worth a file: an entry with only blanks is not written.
pub fn is_blank(text: &str) -> bool {
    text.trim().is_empty()
}

/// The first line of the body with some text, shortened to `max` characters, for lists.
pub fn title(text: &str, max: usize) -> String {
    let body = split_front_matter(text).1;
    let line = body.lines().map(|l| l.trim().trim_start_matches('#').trim()).find(|l| !l.is_empty());
    shorten(line.unwrap_or_default(), max)
}

/// At most `max` characters, with an ellipsis when cut.
pub fn shorten(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}
