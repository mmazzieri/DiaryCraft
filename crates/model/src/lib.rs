//! DiaryCraft data model: a diary is a folder of plain-text entries, one per day.
#![forbid(unsafe_code)]

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Version of the diary folder layout written in `.diarycraft/config.json`.
pub const FORMAT_VERSION: u32 = 1;

/// How the files of the entries are named inside the diary folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Layout {
    /// `YYYY/MM/YYYY-MM-DD.md`: the layout DiaryCraft writes.
    Standard,
    /// `YYYY/MM/DD.txt`: one folder per day of the 2014 LogBook note, flattened.
    LegacyDay,
    /// `YYYY/MM/YYYYMMDD.txt`: the other naming of the 2014 LogBook note.
    LegacyCompact,
}

/// Where an entry lives: its day and the file naming it was found with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntryRef {
    pub date: NaiveDate,
    pub layout: Layout,
}

/// The text of one day, as read from disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub date: NaiveDate,
    /// The whole file, front matter included: DiaryCraft never rewrites what it does not edit.
    pub text: String,
}

/// Settings shared by every computer that opens the diary, kept in `.diarycraft/config.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiaryConfig {
    pub format: String,
    pub format_version: u32,
    /// Keys this version does not know, kept when the file is written again.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for DiaryConfig {
    fn default() -> Self {
        Self { format: "diarycraft".into(), format_version: FORMAT_VERSION, extra: serde_json::Map::new() }
    }
}

#[cfg(test)]
mod tests;
