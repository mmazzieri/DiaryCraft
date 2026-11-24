//! Where each entry lives inside the diary folder.
//!
//! DiaryCraft writes `YYYY/MM/YYYY-MM-DD.md`. It also reads the two namings of the 2014 LogBook
//! note, `YYYY/MM/DD.txt` and `YYYY/MM/YYYYMMDD.txt`, and it recognises the copies that sync
//! clients make when two computers change the same file.

use chrono::{Datelike, NaiveDate};
use diarycraft_model::{EntryRef, Layout};

/// Extension of the entries DiaryCraft writes.
pub const EXTENSION: &str = "md";

/// The folder of the year, the folder of the month and the file name of an entry.
pub fn parts(date: NaiveDate, layout: Layout) -> [String; 3] {
    let year = format!("{:04}", date.year());
    let month = format!("{:02}", date.month());
    let file = match layout {
        Layout::Standard => format!("{}.{EXTENSION}", date.format("%Y-%m-%d")),
        Layout::LegacyDay => format!("{:02}.txt", date.day()),
        Layout::LegacyCompact => format!("{}.txt", date.format("%Y%m%d")),
    };
    [year, month, file]
}

/// The folder of the attachments of an entry, beside its file: `YYYY/MM/YYYY-MM-DD/`.
pub fn attachments_parts(date: NaiveDate) -> [String; 3] {
    let [year, month, _] = parts(date, Layout::Standard);
    [year, month, date.format("%Y-%m-%d").to_string()]
}

fn number(s: &str, len: usize) -> Option<u32> {
    if s.len() == len && s.bytes().all(|b| b.is_ascii_digit()) { s.parse().ok() } else { None }
}

/// Reads the year and month folders; `None` when they are not `YYYY` and `MM`.
pub fn year_month(year: &str, month: &str) -> Option<(i32, u32)> {
    let y = i32::try_from(number(year, 4)?).ok()?;
    let m = number(month, 2)?;
    (1..=12).contains(&m).then_some((y, m))
}

/// Recognises an entry from its file name inside the folder of its year and month.
///
/// The date in the name must agree with the folders: a file put in the wrong month is not an
/// entry of either month, and is left alone.
pub fn parse(year: i32, month: u32, file: &str) -> Option<EntryRef> {
    let (stem, ext) = file.rsplit_once('.')?;
    let ext = ext.to_ascii_lowercase();
    let (date, layout) = if ext == EXTENSION {
        (NaiveDate::parse_from_str(stem, "%Y-%m-%d").ok().filter(|_| stem.len() == 10)?, Layout::Standard)
    } else if ext == "txt" && stem.len() == 2 {
        (NaiveDate::from_ymd_opt(year, month, number(stem, 2)?)?, Layout::LegacyDay)
    } else if ext == "txt" && stem.len() == 8 {
        number(stem, 8)?;
        (NaiveDate::parse_from_str(stem, "%Y%m%d").ok()?, Layout::LegacyCompact)
    } else {
        return None;
    };
    (date.year() == year && date.month() == month).then_some(EntryRef { date, layout })
}

/// Recognises a copy of an entry that is not the entry itself: the copies sync clients make on a
/// conflict (`2026-11-24 (1).md`, `2026-11-24-PC.md`) and the ones DiaryCraft writes when it
/// cannot overwrite a file that changed on disk (`2026-11-24 (conflict …).md`).
pub fn conflict_copy(year: i32, month: u32, file: &str) -> Option<NaiveDate> {
    let (stem, ext) = file.rsplit_once('.')?;
    if !ext.eq_ignore_ascii_case(EXTENSION) || stem.len() <= 10 {
        return None;
    }
    let head = stem.get(..10)?;
    let rest = stem.get(10..)?;
    if !(rest.starts_with(' ') || rest.starts_with('-') || rest.starts_with('_') || rest.starts_with('.')) {
        return None;
    }
    let date = NaiveDate::parse_from_str(head, "%Y-%m-%d").ok()?;
    (date.year() == year && date.month() == month).then_some(date)
}

/// Name of the copy DiaryCraft writes when the file on disk changed while it was being edited.
/// `label` says where the copy comes from (the computer's name); `stamp` makes it unique.
pub fn conflict_name(date: NaiveDate, label: &str, stamp: &str) -> String {
    let clean: String = label
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == ' ')
        .take(40)
        .collect::<String>()
        .trim()
        .to_owned();
    let label = if clean.is_empty() { "DiaryCraft".to_owned() } else { clean };
    format!("{} (conflict {label} {stamp}).{EXTENSION}", date.format("%Y-%m-%d"))
}
