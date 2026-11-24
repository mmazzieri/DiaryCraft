//! Text encodings of the entries.
//!
//! DiaryCraft writes UTF-8. Entries from the 2014 LogBook, written on Windows, may be in
//! Windows-1252: they are read as such, so their accented letters are not lost, and written back
//! in UTF-8 the first time they are changed. A byte-order mark is kept if the file had one.

/// How a file was encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    /// UTF-8 with a byte-order mark, written back with it.
    Utf8Bom,
    /// Not valid UTF-8: read as Windows-1252, written back as UTF-8.
    Windows1252,
}

const BOM: &[u8] = b"\xEF\xBB\xBF";

/// Characters of bytes 0x80 to 0x9F in Windows-1252; the five unassigned ones map to the
/// same-numbered C1 controls, as in the WHATWG encoding standard.
const HIGH: [char; 32] = [
    '\u{20AC}', '\u{81}', '\u{201A}', '\u{192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}', '\u{2C6}', '\u{2030}',
    '\u{160}', '\u{2039}', '\u{152}', '\u{8D}', '\u{17D}', '\u{8F}', '\u{90}', '\u{2018}', '\u{2019}', '\u{201C}',
    '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}', '\u{2DC}', '\u{2122}', '\u{161}', '\u{203A}', '\u{153}', '\u{9D}',
    '\u{17E}', '\u{178}',
];

pub fn decode(bytes: &[u8]) -> (String, Encoding) {
    if let Some(rest) = bytes.strip_prefix(BOM)
        && let Ok(text) = std::str::from_utf8(rest)
    {
        return (text.to_owned(), Encoding::Utf8Bom);
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return (text.to_owned(), Encoding::Utf8);
    }
    let text = bytes
        .iter()
        .map(|&b| match b {
            0x80..=0x9F => HIGH.get(usize::from(b - 0x80)).copied().unwrap_or('\u{FFFD}'),
            _ => char::from(b),
        })
        .collect();
    (text, Encoding::Windows1252)
}

pub fn encode(text: &str, encoding: Encoding) -> Vec<u8> {
    match encoding {
        Encoding::Utf8Bom => [BOM, text.as_bytes()].concat(),
        Encoding::Utf8 | Encoding::Windows1252 => text.as_bytes().to_vec(),
    }
}
