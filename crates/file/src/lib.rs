//! DiaryCraft storage: the diary folder on disk.
//!
//! The rule that matters more than any feature: **the text of the owner is never lost.** Every
//! write goes to a temporary file in the same folder and is then renamed over the entry, so a
//! crash leaves either the old text or the new one. Before writing, the file on disk is compared
//! with the version the editor started from; if somebody else changed it (another computer through
//! the sync client, another editor), DiaryCraft does not overwrite it: it writes the editor's text
//! beside it as a conflict copy and says so. Files are never deleted.
//!
//! Entries are Markdown. A plain-text entry of the 2014 layouts is read where it is, and the first
//! time the owner changes it, it is moved to its standard name (`YYYY/MM/YYYY-MM-DD.md`) and
//! written there in UTF-8: the file is renamed, never deleted, and only when it is still the
//! version the editor read.
#![forbid(unsafe_code)]

mod encoding;

use chrono::NaiveDate;
use diarycraft_engine::layout;
use diarycraft_model::{DiaryConfig, EntryRef, FORMAT_VERSION, Layout};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub use encoding::{Encoding, decode, encode};

/// Folder of DiaryCraft's own files inside the diary.
pub const META_DIR: &str = ".diarycraft";
const CONFIG_FILE: &str = "config.json";
/// Largest entry DiaryCraft opens: a diary page this big is not a diary page.
pub const MAX_ENTRY_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("{0} is not a folder")]
    NotAFolder(PathBuf),
    #[error("{path}: the diary settings cannot be read: {message}")]
    Config { path: PathBuf, message: String },
    #[error("this diary was written by a newer DiaryCraft (format {found}, this version reads up to {FORMAT_VERSION})")]
    TooNew { found: u32 },
    #[error("{path}: the entry is larger than {MAX_ENTRY_BYTES} bytes and is not opened")]
    TooLarge { path: PathBuf },
}

pub type Result<T> = std::result::Result<T, Error>;

fn io_err(path: &Path) -> impl FnOnce(io::Error) -> Error + '_ {
    move |source| Error::Io { path: path.to_owned(), source }
}

/// Identifies one version of a file: SHA-256 of its bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fingerprint([u8; 32]);

impl Fingerprint {
    pub fn of(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// The fingerprint as lowercase hexadecimal, the form the command line and agents see.
    pub fn to_hex(self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// An entry as read from disk, with what is needed to write it back safely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    pub date: NaiveDate,
    pub text: String,
    /// The version on disk the text comes from; `None` when the day has no file yet.
    pub base: Option<Fingerprint>,
    /// Where the text is written back: the file it was read from, or the standard name.
    pub path: PathBuf,
    pub encoding: Encoding,
}

impl Loaded {
    /// Takes in a successful save: the version now on disk and the file it is in, which differs
    /// from the one read when a plain-text entry was moved to its Markdown name. A Windows-1252
    /// entry is UTF-8 from its first save on.
    pub fn written(&mut self, base: Fingerprint, path: PathBuf) {
        self.base = Some(base);
        self.path = path;
        if self.encoding == Encoding::Windows1252 {
            self.encoding = Encoding::Utf8;
        }
    }
}

/// What happened to a save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Saved {
    /// The text is on disk, in `path`; `base` is the new version to compare with at the next
    /// save. Pass both to [`Loaded::written`].
    Written { base: Fingerprint, path: PathBuf },
    /// Nothing to write: the text is blank and the day has no file.
    Skipped,
    /// The file changed on disk since it was read. It was left as it is and the editor's text
    /// was written to `copy`; `disk` is what the file holds now.
    Conflict { copy: PathBuf, disk: Loaded },
}

/// A diary folder.
#[derive(Debug)]
pub struct Diary {
    root: PathBuf,
    config: DiaryConfig,
    /// The entries found by the last scan, with the naming each was found with.
    entries: BTreeMap<NaiveDate, Layout>,
    /// Conflict copies found by the last scan.
    copies: Vec<(NaiveDate, PathBuf)>,
}

impl Diary {
    /// Opens a diary folder, creating its settings file if there is none.
    pub fn open(root: &Path) -> Result<Self> {
        let meta = fs::metadata(root).map_err(io_err(root))?;
        if !meta.is_dir() {
            return Err(Error::NotAFolder(root.to_owned()));
        }
        let config_path = root.join(META_DIR).join(CONFIG_FILE);
        let config = match fs::read(&config_path) {
            Ok(bytes) => {
                let config: DiaryConfig = serde_json::from_slice(&bytes)
                    .map_err(|e| Error::Config { path: config_path.clone(), message: e.to_string() })?;
                if config.format_version > FORMAT_VERSION {
                    return Err(Error::TooNew { found: config.format_version });
                }
                config
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let config = DiaryConfig::default();
                let dir = root.join(META_DIR);
                fs::create_dir_all(&dir).map_err(io_err(&dir))?;
                let text = serde_json::to_vec_pretty(&config)
                    .map_err(|e| Error::Config { path: config_path.clone(), message: e.to_string() })?;
                write_atomic(&config_path, &text)?;
                config
            }
            Err(e) => return Err(Error::Io { path: config_path, source: e }),
        };
        let mut diary = Self { root: root.to_owned(), config, entries: BTreeMap::new(), copies: Vec::new() };
        diary.scan()?;
        Ok(diary)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config(&self) -> &DiaryConfig {
        &self.config
    }

    /// Looks through the folder again for entries and conflict copies.
    pub fn scan(&mut self) -> Result<()> {
        let mut entries = BTreeMap::new();
        let mut copies = Vec::new();
        for (year_name, year_path) in dirs(&self.root)? {
            for (month_name, month_path) in dirs(&year_path)? {
                let Some((year, month)) = layout::year_month(&year_name, &month_name) else {
                    continue;
                };
                let list = fs::read_dir(&month_path).map_err(io_err(&month_path))?;
                for item in list.flatten() {
                    if !item.file_type().is_ok_and(|t| t.is_file()) {
                        continue;
                    }
                    let Some(name) = item.file_name().to_str().map(str::to_owned) else {
                        continue;
                    };
                    if let Some(r) = layout::parse(year, month, &name) {
                        if let Some(hidden) = add_entry(&mut entries, r) {
                            copies.push((r.date, self.path_of(hidden)));
                        }
                    } else if let Some(date) = layout::conflict_copy(year, month, &name) {
                        copies.push((date, item.path()));
                    }
                }
            }
        }
        copies.sort();
        self.entries = entries;
        self.copies = copies;
        Ok(())
    }

    /// Days that have an entry, oldest first.
    pub fn dates(&self) -> impl DoubleEndedIterator<Item = NaiveDate> + '_ {
        self.entries.keys().copied()
    }

    pub fn has_entry(&self, date: NaiveDate) -> bool {
        self.entries.contains_key(&date)
    }

    /// Conflict copies found by the last scan, by day.
    pub fn conflict_copies(&self) -> &[(NaiveDate, PathBuf)] {
        &self.copies
    }

    /// The file of a day: the one found by the scan, or the standard name for a new entry.
    pub fn entry_path(&self, date: NaiveDate) -> PathBuf {
        let layout = self.entries.get(&date).copied().unwrap_or(Layout::Standard);
        self.path_of(EntryRef { date, layout })
    }

    fn path_of(&self, r: EntryRef) -> PathBuf {
        let [year, month, file] = layout::parts(r.date, r.layout);
        self.root.join(year).join(month).join(file)
    }

    /// Reads the entry of a day; a day without a file reads as empty.
    pub fn read(&self, date: NaiveDate) -> Result<Loaded> {
        let path = self.entry_path(date);
        match read_bytes(&path)? {
            None => Ok(Loaded { date, text: String::new(), base: None, path, encoding: Encoding::Utf8 }),
            Some(bytes) => {
                let (text, encoding) = encoding::decode(&bytes);
                Ok(Loaded { date, text, base: Some(Fingerprint::of(&bytes)), path, encoding })
            }
        }
    }

    /// Writes the text of a day, unless the file changed on disk since `loaded` was read.
    ///
    /// `label` names this computer in a conflict copy and `stamp` makes its name unique (the
    /// caller passes the time: the storage has no clock of its own).
    pub fn save(&mut self, loaded: &Loaded, text: &str, label: &str, stamp: &str) -> Result<Saved> {
        let current = read_bytes(&loaded.path)?;
        let current_print = current.as_deref().map(Fingerprint::of);
        if current_print != loaded.base {
            let copy = self.conflict_path(loaded.date, label, stamp);
            write_atomic(&copy, &encoding::encode(text, loaded.encoding))?;
            self.scan()?;
            let disk = match current {
                Some(bytes) => {
                    let (text, encoding) = encoding::decode(&bytes);
                    Loaded { text, base: current_print, encoding, ..loaded.clone() }
                }
                None => Loaded { text: String::new(), base: None, encoding: Encoding::Utf8, ..loaded.clone() },
            };
            return Ok(Saved::Conflict { copy, disk });
        }
        if current.is_none() && diarycraft_engine::text::is_blank(text) {
            return Ok(Saved::Skipped);
        }
        let bytes = encoding::encode(text, loaded.encoding);
        let base = Fingerprint::of(&bytes);
        let standard = self.path_of(EntryRef { date: loaded.date, layout: Layout::Standard });
        if loaded.path != standard && !standard.exists() {
            // A plain-text entry becomes Markdown. The file is moved first, so that the old text
            // is never without a name: if the write after it fails, the entry still holds it.
            if current.is_some() {
                fs::rename(&loaded.path, &standard).map_err(io_err(&loaded.path))?;
            }
            write_atomic(&standard, &bytes)?;
            self.scan()?;
            return Ok(Saved::Written { base, path: standard });
        }
        write_atomic(&loaded.path, &bytes)?;
        if current.is_none() {
            self.scan()?;
        }
        Ok(Saved::Written { base, path: loaded.path.clone() })
    }

    fn conflict_path(&self, date: NaiveDate, label: &str, stamp: &str) -> PathBuf {
        let [year, month, _] = layout::parts(date, Layout::Standard);
        let dir = self.root.join(year).join(month);
        let mut path = dir.join(layout::conflict_name(date, label, stamp));
        let mut n = 2u32;
        while path.exists() && n < 1000 {
            path = dir.join(layout::conflict_name(date, label, &format!("{stamp}-{n}")));
            n += 1;
        }
        path
    }

    /// The text of every entry, for search. Files that cannot be read are skipped.
    pub fn load_all(&self) -> Vec<(NaiveDate, String)> {
        self.entries
            .iter()
            .filter_map(|(&date, &layout)| {
                let bytes = read_bytes(&self.path_of(EntryRef { date, layout })).ok()??;
                Some((date, encoding::decode(&bytes).0))
            })
            .collect()
    }

    /// The folder of a day's attachments.
    pub fn attachments_dir(&self, date: NaiveDate) -> PathBuf {
        let [year, month, day] = layout::attachments_parts(date);
        self.root.join(year).join(month).join(day)
    }

    /// The attachments of a day, by name.
    pub fn attachments(&self, date: NaiveDate) -> Vec<PathBuf> {
        let Ok(list) = fs::read_dir(self.attachments_dir(date)) else {
            return Vec::new();
        };
        let mut files: Vec<PathBuf> =
            list.flatten().filter(|i| i.file_type().is_ok_and(|t| t.is_file())).map(|i| i.path()).collect();
        files.sort();
        files
    }

    /// Copies a file into the attachments of a day and returns its path there. An attachment
    /// with the same name is never replaced: the copy gets a number.
    pub fn attach(&self, date: NaiveDate, source: &Path) -> Result<PathBuf> {
        let dir = self.attachments_dir(date);
        fs::create_dir_all(&dir).map_err(io_err(&dir))?;
        let name = source.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
        let (stem, ext) = match name.rsplit_once('.') {
            Some((s, e)) if !s.is_empty() => (s.to_owned(), format!(".{e}")),
            _ => (name.clone(), String::new()),
        };
        let mut target = dir.join(&name);
        let mut n = 2u32;
        while target.exists() && n < 10_000 {
            target = dir.join(format!("{stem} ({n}){ext}"));
            n += 1;
        }
        fs::copy(source, &target).map_err(io_err(source))?;
        Ok(target)
    }
}

/// Keeps one naming per day: DiaryCraft's own wins over the 2014 ones. Returns the file that
/// lost, which is listed with the conflict copies so that its text is not out of sight (a sync
/// client may bring back a `.txt` entry that was moved to its `.md` name on another computer).
pub fn add_entry(entries: &mut BTreeMap<NaiveDate, Layout>, r: EntryRef) -> Option<EntryRef> {
    let rank = |l: Layout| match l {
        Layout::Standard => 0,
        Layout::LegacyCompact => 1,
        Layout::LegacyDay => 2,
    };
    match entries.get(&r.date).copied() {
        Some(old) if rank(old) <= rank(r.layout) => Some(r),
        old => {
            entries.insert(r.date, r.layout);
            old.map(|layout| EntryRef { date: r.date, layout })
        }
    }
}

/// Sub-folders of `dir` with names that are valid text.
fn dirs(dir: &Path) -> Result<Vec<(String, PathBuf)>> {
    let list = fs::read_dir(dir).map_err(io_err(dir))?;
    Ok(list
        .flatten()
        .filter(|i| i.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|i| Some((i.file_name().to_str()?.to_owned(), i.path())))
        .collect())
}

fn read_bytes(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::metadata(path) {
        Ok(meta) if meta.len() > MAX_ENTRY_BYTES => return Err(Error::TooLarge { path: path.to_owned() }),
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(Error::Io { path: path.to_owned(), source: e }),
    }
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::Io { path: path.to_owned(), source: e }),
    }
}

/// Writes a file through a temporary file in the same folder and a rename, so that a crash or a
/// full disk never leaves half a file in place of the entry.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(dir).map_err(io_err(dir))?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = dir.join(format!(".{name}.tmp-{}", std::process::id()));
    let result = (|| {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, path)
    })();
    if let Err(source) = result {
        let _ = fs::remove_file(&tmp);
        return Err(Error::Io { path: path.to_owned(), source });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
