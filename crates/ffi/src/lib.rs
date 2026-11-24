//! DiaryCraft's engine for the phone apps.
//!
//! The apps call one function, [`call`], with a request in JSON and get an answer in JSON:
//! `{"ok": …}` or `{"error": "…"}`. The names of the entries, their encodings and what a save may
//! write are the desktop's own code, so a phone treats the diary folder as a computer does. The
//! phone does the file work. The requests are in [`Request`]; `docs/mobile.md` says how they are used.
//!
//! Only the modules that make [`call`] reachable from Java (`android`) and from C and Swift (`c`)
//! use `unsafe`.

#[cfg(target_os = "android")]
mod android;
#[cfg(not(target_os = "android"))]
pub mod c;

use chrono::NaiveDate;
use diarycraft_engine::{layout, search, text};
use diarycraft_file::{Encoding, Fingerprint, MAX_ENTRY_BYTES, add_entry, decode, encode};
use diarycraft_model::{EntryRef, Layout};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Debug, Clone, Deserialize)]
pub struct Entry {
    pub date: NaiveDate,
    pub text: String,
}

/// Bytes travel as text with one character (U+0000 to U+00FF) for each byte.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    /// Which files, named `YYYY/MM/name`, are entries and which are conflict copies.
    Scan { files: Vec<String> },
    /// The text of a file.
    Decode { raw: String },
    /// What to write to save `text`, given what the folder holds now.
    Plan(Plan),
    /// Entries with every word of `query`.
    Search { entries: Vec<Entry>, query: String },
    /// Title and number of words.
    Summary { text: String },
}

#[derive(Debug, Clone, Deserialize)]
pub struct Plan {
    pub date: NaiveDate,
    /// The file the text was read from, `YYYY/MM/name`; none for a new entry.
    #[serde(default)]
    pub path: Option<String>,
    /// The version the text was read from; none for a new entry.
    #[serde(default)]
    pub base: Option<String>,
    #[serde(default)]
    pub encoding: Option<Encoding>,
    /// What that file holds now; none when it is not there.
    #[serde(default)]
    pub disk: Option<String>,
    pub text: String,
    /// Names this phone in a conflict copy.
    pub label: String,
    pub stamp: String,
    /// Names already in the folder of the month.
    #[serde(default)]
    pub names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "do", rename_all = "snake_case")]
pub enum Action {
    /// Nothing to write.
    Skip,
    /// Replace or create `path`.
    Write { path: String, raw: String, version: String },
    /// The file changed meanwhile: leave it, and create `path` beside it.
    Conflict { path: String, raw: String },
}

fn to_raw(bytes: &[u8]) -> String {
    bytes.iter().copied().map(char::from).collect()
}

fn from_raw(raw: &str) -> Result<Vec<u8>, String> {
    let bytes: Option<Vec<u8>> = raw.chars().map(|c| u8::try_from(u32::from(c)).ok()).collect();
    let bytes = bytes.ok_or("the bytes could not be read")?;
    if bytes.len() as u64 > MAX_ENTRY_BYTES { Err("the entry is too large".to_owned()) } else { Ok(bytes) }
}

fn scan(files: &[String]) -> Value {
    let mut entries: BTreeMap<NaiveDate, Layout> = BTreeMap::new();
    let mut names: BTreeMap<(NaiveDate, u8), &str> = BTreeMap::new();
    let mut copies: Vec<(NaiveDate, &str)> = Vec::new();
    for file in files {
        let mut parts = file.split('/');
        let (Some(y), Some(m), Some(name), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        let Some((year, month)) = layout::year_month(y, m) else { continue };
        if let Some(r) = layout::parse(year, month, name) {
            names.insert((r.date, r.layout as u8), file);
            if let Some(lost) = add_entry(&mut entries, r)
                && let Some(path) = names.get(&(lost.date, lost.layout as u8))
            {
                copies.push((lost.date, path));
            }
        } else if let Some(date) = layout::conflict_copy(year, month, name) {
            copies.push((date, file));
        }
    }
    copies.sort_unstable();
    let list = |rows: Vec<(NaiveDate, &str)>| -> Vec<Value> {
        rows.into_iter().map(|(date, path)| json!({"date": date, "path": path})).collect()
    };
    let found = entries.iter().rev().filter_map(|(d, l)| Some((*d, *names.get(&(*d, *l as u8))?))).collect();
    json!({"entries": list(found), "conflicts": list(copies)})
}

fn plan(p: &Plan) -> Result<Action, String> {
    let [year, month, standard] = layout::parts(p.date, Layout::Standard);
    let path = p.path.clone().unwrap_or_else(|| format!("{year}/{month}/{standard}"));
    let name = path.strip_prefix(&format!("{year}/{month}/")).unwrap_or_default();
    let own = layout::year_month(&year, &month)
        .and_then(|(y, m)| layout::parse(y, m, name))
        .is_some_and(|EntryRef { date, .. }| date == p.date);
    if !own {
        return Err("the file is not an entry of that day".to_owned());
    }
    let encoding = p.encoding.unwrap_or(Encoding::Utf8);
    let bytes = encode(&p.text, encoding);
    if bytes.len() as u64 > MAX_ENTRY_BYTES {
        return Err("the entry is too large".to_owned());
    }
    let disk = p.disk.as_deref().map(from_raw).transpose()?;
    if disk.as_deref() == Some(bytes.as_slice()) {
        return Ok(Action::Skip);
    }
    let now = disk.as_deref().map(|d| Fingerprint::of(d).to_hex());
    if now != p.base {
        let mut copy = layout::conflict_name(p.date, &p.label, &p.stamp);
        let mut n = 2u32;
        while p.names.contains(&copy) {
            copy = layout::conflict_name(p.date, &p.label, &format!("{}-{n}", p.stamp));
            n += 1;
        }
        return Ok(Action::Conflict { path: format!("{year}/{month}/{copy}"), raw: to_raw(&bytes) });
    }
    if disk.is_none() && text::is_blank(&p.text) {
        return Ok(Action::Skip);
    }
    Ok(Action::Write { path, raw: to_raw(&bytes), version: Fingerprint::of(&bytes).to_hex() })
}

fn run(request: Request) -> Result<Value, String> {
    Ok(match request {
        Request::Scan { files } => scan(&files),
        Request::Decode { raw } => {
            let bytes = from_raw(&raw)?;
            let (text, encoding) = decode(&bytes);
            json!({"text": text, "encoding": encoding, "version": Fingerprint::of(&bytes).to_hex()})
        }
        Request::Plan(p) => serde_json::to_value(plan(&p)?).map_err(|e| e.to_string())?,
        Request::Search { entries, query } => {
            let hits = search::search(entries.iter().map(|e| (e.date, e.text.as_str())), &query);
            let hits: Vec<Value> =
                hits.iter().map(|h| json!({"date": h.date, "snippet": h.snippet, "count": h.count})).collect();
            json!(hits)
        }
        Request::Summary { text } => json!({"title": text::title(&text, 80), "words": text::word_count(&text)}),
    })
}

/// Runs one request. Never panics: a request that cannot be read, or a fault inside, comes back
/// as `{"error": …}`.
pub fn call(input: &str) -> String {
    let answer = catch_unwind(AssertUnwindSafe(|| {
        serde_json::from_str(input).map_err(|e| format!("the request could not be read: {e}")).and_then(run)
    }))
    .unwrap_or_else(|_| Err("DiaryCraft's engine stopped on an internal error".to_owned()));
    let value = match answer {
        Ok(v) => json!({ "ok": v }),
        Err(e) => json!({ "error": e }),
    };
    serde_json::to_string(&value).unwrap_or_else(|_| "{\"error\":\"the answer could not be written\"}".to_owned())
}

#[cfg(test)]
mod tests;
