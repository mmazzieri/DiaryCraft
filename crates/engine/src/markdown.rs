//! Markdown highlighting for the editor.
//!
//! The editor shows the entry as it is in the file, marks included, and colours it while the owner
//! types: headings, emphasis, code, links, lists, quotes, rules and the front matter. This is not
//! a Markdown parser and does not try to be one: it reads one line at a time with a few rules that
//! match what people write in a diary, and it never fails. Whatever it does not recognise stays
//! plain text. The formatted view is drawn by a real CommonMark parser in the interface.

use std::ops::Range;

/// What a piece of the text is, for its colour and font.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Plain,
    /// The YAML front matter, fences included.
    FrontMatter,
    /// The text of a heading, level 1 to 6.
    Heading(u8),
    /// The characters that make the formatting and are not shown in the formatted view:
    /// `#`, `**`, `_`, `>`, brackets of a link.
    Syntax,
    Strong,
    Emphasis,
    /// Inline code, backticks included.
    Code,
    /// A fenced code block, fences included.
    CodeBlock,
    /// The text of a link.
    Link,
    /// A web address or the target of a link.
    Url,
    /// The bullet, number or check box of a list item.
    ListMarker,
    /// The text of a quote.
    Quote,
    /// A horizontal rule.
    Rule,
}

/// A piece of the text and what it is. The spans of [`highlight`] cover the text from start to
/// end, in order, without gaps, and each range lies on character boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub range: Range<usize>,
    pub mark: Mark,
}

/// Splits the text into spans to colour. Lines longer than this are left plain, so that a pasted
/// blob never slows the editor down.
const MAX_LINE: usize = 64 * 1024;

pub fn highlight(text: &str) -> Vec<Span> {
    let mut out = Spans::default();
    let (front, body) = crate::text::split_front_matter(text);
    let start = if front.is_some() { text.len().saturating_sub(body.len()) } else { 0 };
    out.push(0..start, Mark::FrontMatter);
    let mut fence: Option<(u8, usize)> = None;
    let mut offset = start;
    for line in body.split_inclusive('\n') {
        let at = offset;
        offset += line.len();
        if line.len() > MAX_LINE {
            out.push(at..offset, if fence.is_some() { Mark::CodeBlock } else { Mark::Plain });
            continue;
        }
        if let Some((ch, len)) = fence {
            if fence_of(line).is_some_and(|(c, l)| c == ch && l >= len && rest_after_fence(line, l).is_empty()) {
                fence = None;
            }
            out.push(at..offset, Mark::CodeBlock);
            continue;
        }
        if let Some(f) = fence_of(line) {
            fence = Some(f);
            out.push(at..offset, Mark::CodeBlock);
            continue;
        }
        block_line(line, at, &mut out);
    }
    out.finish(text.len())
}

/// The spans being built: consecutive spans with the same mark are joined.
#[derive(Default)]
struct Spans(Vec<Span>);

impl Spans {
    fn push(&mut self, range: Range<usize>, mark: Mark) {
        if range.start >= range.end {
            return;
        }
        if let Some(last) = self.0.last_mut()
            && last.mark == mark
            && last.range.end == range.start
        {
            last.range.end = range.end;
            return;
        }
        self.0.push(Span { range, mark });
    }

    fn finish(mut self, len: usize) -> Vec<Span> {
        let end = self.0.last().map(|s| s.range.end).unwrap_or(0);
        self.push(end..len, Mark::Plain);
        self.0
    }
}

/// Leading spaces of a line, when there are at most three (more is not a block mark).
fn indent(line: &str) -> Option<usize> {
    let n = line.bytes().take_while(|&b| b == b' ').count();
    (n <= 3).then_some(n)
}

/// A code fence: three or more backticks or tildes. Returns the character and how many.
fn fence_of(line: &str) -> Option<(u8, usize)> {
    let rest = line.get(indent(line)?..)?;
    let ch = *rest.as_bytes().first()?;
    if ch != b'`' && ch != b'~' {
        return None;
    }
    let len = rest.bytes().take_while(|&b| b == ch).count();
    if len < 3 {
        return None;
    }
    // A backtick fence's info string has no backticks.
    if ch == b'`' && rest.get(len..).is_some_and(|info| info.contains('`')) {
        return None;
    }
    Some((ch, len))
}

fn rest_after_fence(line: &str, len: usize) -> &str {
    let rest = line.trim_start_matches(' ');
    rest.get(len..).unwrap_or_default().trim()
}

fn block_line(line: &str, at: usize, out: &mut Spans) {
    let Some(ind) = indent(line) else {
        inline(line, at, Mark::Plain, out);
        return;
    };
    let rest = line.get(ind..).unwrap_or_default();
    let content = rest.trim_end_matches(['\n', '\r']);
    // Heading: one to six `#` followed by a space or the end of the line.
    let hashes = content.bytes().take_while(|&b| b == b'#').count();
    if (1..=6).contains(&hashes) && content.get(hashes..).is_some_and(|r| r.is_empty() || r.starts_with([' ', '\t'])) {
        let marker = ind + hashes;
        out.push(at..at + marker, Mark::Syntax);
        let level = u8::try_from(hashes).unwrap_or(6);
        out.push(at + marker..at + line.len(), Mark::Heading(level));
        return;
    }
    if is_rule(content) {
        out.push(at..at + line.len(), Mark::Rule);
        return;
    }
    if let Some(after) = content.strip_prefix('>') {
        let marker = ind + 1 + usize::from(after.starts_with(' '));
        out.push(at..at + marker, Mark::Syntax);
        let quoted = line.get(marker..).unwrap_or_default();
        inline(quoted, at + marker, Mark::Quote, out);
        return;
    }
    if let Some(marker) = list_marker(content) {
        let marker = ind + marker;
        out.push(at..at + marker, Mark::ListMarker);
        inline(line.get(marker..).unwrap_or_default(), at + marker, Mark::Plain, out);
        return;
    }
    inline(line, at, Mark::Plain, out);
}

/// Three or more `-`, `*` or `_`, all the same, with only spaces between them.
fn is_rule(content: &str) -> bool {
    let mut chars = content.chars().filter(|c| !c.is_whitespace());
    let Some(first) = chars.next() else { return false };
    if !matches!(first, '-' | '*' | '_') {
        return false;
    }
    let mut n = 1;
    for c in chars {
        if c != first {
            return false;
        }
        n += 1;
    }
    n >= 3
}

/// Length of the marker of a list item: `- `, `* `, `+ `, `12. `, `12) `, with a following
/// check box `[ ] ` or `[x] ` included.
fn list_marker(content: &str) -> Option<usize> {
    let bytes = content.as_bytes();
    let bullet = match bytes.first()? {
        b'-' | b'*' | b'+' => 1,
        b'0'..=b'9' => {
            let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
            if digits > 9 || !matches!(bytes.get(digits), Some(b'.' | b')')) {
                return None;
            }
            digits + 1
        }
        _ => return None,
    };
    match bytes.get(bullet) {
        Some(b' ' | b'\t') => {}
        None => return Some(bullet),
        _ => return None,
    }
    let mut len = bullet + 1;
    if let Some(b'[') = bytes.get(len)
        && matches!(bytes.get(len + 1), Some(b' ' | b'x' | b'X'))
        && bytes.get(len + 2) == Some(&b']')
        && matches!(bytes.get(len + 3), Some(b' ') | None)
    {
        len += 4;
    }
    Some(len.min(content.len()))
}

/// Emphasis, code and links inside a line. Text that is none of them gets `base`.
fn inline(s: &str, at: usize, base: Mark, out: &mut Spans) {
    let bytes = s.as_bytes();
    let mut plain = 0;
    let mut i = 0;
    while i < bytes.len() {
        let found = match bytes.get(i) {
            Some(b'`') => code(s, i),
            Some(b'*' | b'_') => emphasis(s, i),
            Some(b'[') => link(s, i, 0),
            Some(b'!') if bytes.get(i + 1) == Some(&b'[') => link(s, i + 1, 1),
            Some(b'<') => autolink(s, i),
            Some(b'h') if starts_web_address(s, i) => Some(vec![(i..web_address_end(s, i), Mark::Url)]),
            _ => None,
        };
        match found {
            Some(pieces) if !pieces.is_empty() => {
                out.push(at + plain..at + i, base);
                let mut end = i;
                for (range, mark) in pieces {
                    end = range.end;
                    out.push(at + range.start..at + range.end, mark);
                }
                plain = end;
                i = end;
            }
            _ => {
                // Skip a whole run of the same mark character (`**`, a run of backticks), so that
                // a run that matched nothing is not tried again from its middle.
                let b = bytes.get(i).copied().unwrap_or(0);
                if matches!(b, b'`' | b'*' | b'_') {
                    i += bytes.get(i..).map(|r| r.iter().take_while(|&&c| c == b).count()).unwrap_or(1).max(1);
                } else {
                    i += 1;
                }
                while i < bytes.len() && !s.is_char_boundary(i) {
                    i += 1;
                }
            }
        }
    }
    out.push(at + plain..at + s.len(), base);
}

type Pieces = Vec<(Range<usize>, Mark)>;

/// Inline code: a run of backticks up to the next run of the same length.
fn code(s: &str, i: usize) -> Option<Pieces> {
    let bytes = s.as_bytes();
    let run = bytes.get(i..)?.iter().take_while(|&&b| b == b'`').count();
    let mut j = i + run;
    while j < bytes.len() {
        if bytes.get(j) == Some(&b'`') {
            let other = bytes.get(j..)?.iter().take_while(|&&b| b == b'`').count();
            if other == run {
                return Some(vec![(i..j + run, Mark::Code)]);
            }
            j += other;
        } else if bytes.get(j) == Some(&b'\n') {
            return None;
        } else {
            j += 1;
        }
    }
    None
}

/// `**strong**`, `__strong__`, `*emphasis*`, `_emphasis_`, closed on the same line.
fn emphasis(s: &str, i: usize) -> Option<Pieces> {
    let bytes = s.as_bytes();
    let ch = *bytes.get(i)?;
    let run = bytes.get(i..)?.iter().take_while(|&&b| b == ch).count();
    let len = if run >= 2 { 2 } else { 1 };
    // An underscore inside a word is part of the word (`snake_case`).
    if ch == b'_' && i > 0 && s.get(..i)?.chars().next_back().is_some_and(char::is_alphanumeric) {
        return None;
    }
    let open_end = i + len;
    let next = s.get(open_end..)?.chars().next()?;
    if next.is_whitespace() {
        return None;
    }
    let pair = [ch, ch];
    let delimiter = pair.get(..len)?;
    let mut j = open_end + 1;
    while j + len <= bytes.len() {
        if bytes.get(j) == Some(&b'\n') {
            return None;
        }
        if bytes.get(j..j + len) == Some(delimiter) && s.is_char_boundary(j) {
            let before = s.get(..j)?.chars().next_back()?;
            let after = s.get(j + len..)?.chars().next();
            let closes = !before.is_whitespace()
                && (ch != b'_' || !after.is_some_and(char::is_alphanumeric))
                // `*a**`: a single mark is not closed by a double one.
                && bytes.get(j + len) != Some(&ch);
            if closes {
                let mark = if len == 2 { Mark::Strong } else { Mark::Emphasis };
                return Some(vec![(i..open_end, Mark::Syntax), (open_end..j, mark), (j..j + len, Mark::Syntax)]);
            }
            j += len;
            continue;
        }
        j += 1;
    }
    None
}

/// `[text](target)`; `bang` is 1 for an image, whose `!` comes before the bracket.
fn link(s: &str, i: usize, bang: usize) -> Option<Pieces> {
    let rest = s.get(i + 1..)?;
    let close = rest.find(']')?;
    if rest.get(..close)?.contains(['[', '\n']) {
        return None;
    }
    let text_end = i + 1 + close;
    if s.as_bytes().get(text_end + 1) != Some(&b'(') {
        return None;
    }
    let target = s.get(text_end + 2..)?;
    let paren = target.find(')').filter(|&p| !target.get(..p).is_some_and(|t| t.contains('\n')))?;
    let end = text_end + 2 + paren + 1;
    let start = i.saturating_sub(bang);
    Some(vec![
        (start..i + 1, Mark::Syntax),
        (i + 1..text_end, Mark::Link),
        (text_end..text_end + 2, Mark::Syntax),
        (text_end + 2..end - 1, Mark::Url),
        (end - 1..end, Mark::Syntax),
    ])
}

/// `<https://…>` or `<mailto:…>`.
fn autolink(s: &str, i: usize) -> Option<Pieces> {
    let rest = s.get(i + 1..)?;
    let close = rest.find('>')?;
    let inner = rest.get(..close)?;
    let scheme = inner.split_once(':').map(|(s, _)| s)?;
    let ok = !inner.contains(char::is_whitespace)
        && scheme.len() >= 2
        && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    ok.then(|| {
        vec![(i..i + 1, Mark::Syntax), (i + 1..i + 1 + close, Mark::Url), (i + 1 + close..i + 2 + close, Mark::Syntax)]
    })
}

fn starts_web_address(s: &str, i: usize) -> bool {
    let rest = s.get(i..).unwrap_or_default();
    let at_word_start = i == 0 || s.get(..i).and_then(|b| b.chars().next_back()).is_some_and(|c| !c.is_alphanumeric());
    at_word_start && (rest.starts_with("https://") || rest.starts_with("http://"))
}

fn web_address_end(s: &str, i: usize) -> usize {
    let rest = s.get(i..).unwrap_or_default();
    let len = rest.find(|c: char| c.is_whitespace() || c == '<' || c == '>').unwrap_or(rest.len());
    let address = rest.get(..len).unwrap_or_default();
    // Punctuation that ends a sentence is not part of the address.
    let trimmed = address.trim_end_matches(['.', ',', ';', ':', '!', '?', ')', '"', '\'']);
    i + trimmed.len().max(1)
}

/// Where a link of the formatted view leads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    /// A web page or an e-mail address, opened by the system.
    Web(String),
    /// A file of the diary, as the names of the folders and the file from the entry's folder:
    /// the attachments DiaryCraft links (`2026-11-24/foto.jpg`).
    Relative(Vec<String>),
}

/// Reads the destination of a link. Only `http`, `https` and `mailto` lead outside; a relative
/// path leads to a file of the diary, and never above the entry's folder. Anything else (another
/// scheme, an absolute path, `..`) leads nowhere.
pub fn link_target(destination: &str) -> Option<LinkTarget> {
    let destination = destination.trim();
    if let Some((scheme, _)) = destination.split_once(':')
        && scheme.len() >= 2
        && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    {
        let scheme = scheme.to_ascii_lowercase();
        let ok = matches!(scheme.as_str(), "http" | "https" | "mailto") && !destination.contains(char::is_control);
        return ok.then(|| LinkTarget::Web(destination.to_owned()));
    }
    let path = destination.split(['#', '?']).next().unwrap_or_default();
    let path = percent_decode(path)?;
    if path.is_empty() || path.starts_with(['/', '\\']) {
        return None;
    }
    let mut parts = Vec::new();
    for part in path.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => return None,
            p if p.contains(['\0', ':']) => return None,
            p => parts.push(p.to_owned()),
        }
    }
    (!parts.is_empty()).then_some(LinkTarget::Relative(parts))
}

/// `%20` and the like; `None` when the result is not UTF-8.
fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while let Some(&b) = bytes.get(i) {
        let hex = |j: usize| bytes.get(j).and_then(|&c| char::from(c).to_digit(16));
        if b == b'%'
            && let (Some(hi), Some(lo)) = (hex(i + 1), hex(i + 2))
        {
            out.push(u8::try_from(hi * 16 + lo).ok()?);
            i += 3;
        } else {
            out.push(b);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}
