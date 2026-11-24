//! Interface text in Italian and English.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lang {
    It,
    En,
}

impl Lang {
    /// The language of the system, from the usual environment variables; English otherwise.
    pub fn from_env() -> Self {
        let it = ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"]
            .iter()
            .filter_map(|k| std::env::var(k).ok())
            .find(|v| !v.is_empty())
            .is_some_and(|v| v.to_ascii_lowercase().starts_with("it"));
        if it { Lang::It } else { Lang::En }
    }

    pub fn strings(self) -> &'static Strings {
        match self {
            Lang::It => &IT,
            Lang::En => &EN,
        }
    }
}

pub struct Strings {
    pub file: &'static str,
    pub open_diary: &'static str,
    pub open_recent: &'static str,
    pub close_diary: &'static str,
    pub quit: &'static str,
    pub language: &'static str,
    pub today: &'static str,
    pub search_hint: &'static str,
    pub no_results: &'static str,
    pub recent_entries: &'static str,
    pub results: &'static str,
    pub empty_title: &'static str,
    pub empty_body: &'static str,
    pub use_suggested: &'static str,
    pub choose_other: &'static str,
    pub saved: &'static str,
    pub pending: &'static str,
    pub words: &'static str,
    pub attach: &'static str,
    pub attachments: &'static str,
    pub open_folder: &'static str,
    pub conflict_title: &'static str,
    pub conflict_body: &'static str,
    pub use_disk: &'static str,
    pub keep_mine: &'static str,
    pub compare: &'static str,
    pub disk_version: &'static str,
    pub my_version: &'static str,
    pub close: &'static str,
    pub conflict_copies: &'static str,
    pub reloaded: &'static str,
    pub legacy_encoding: &'static str,
    pub months: [&'static str; 12],
    pub weekdays: [&'static str; 7],
    pub weekdays_long: [&'static str; 7],
}

pub static IT: Strings = Strings {
    file: "File",
    open_diary: "Apri cartella del diario…",
    use_suggested: "Usa questa cartella",
    choose_other: "Scegli un'altra cartella…",
    open_recent: "Diari recenti",
    close_diary: "Chiudi diario",
    quit: "Esci",
    language: "Lingua",
    today: "Oggi",
    search_hint: "Cerca nel diario",
    no_results: "Nessuna voce contiene queste parole.",
    recent_entries: "Ultime voci",
    results: "Risultati",
    empty_title: "Nessun diario aperto",
    empty_body: "Un diario è una cartella: ogni giorno è un file di testo in anno/mese. DiaryCraft propone la cartella qui sotto; puoi sceglierne un'altra, vuota per cominciare o quella di un diario che hai già. Può stare in una cartella sincronizzata (OneDrive, Google Drive, Dropbox, Syncthing) per usarlo da più computer.",
    saved: "Salvato",
    pending: "Modifiche non ancora salvate",
    words: "parole",
    attach: "Allega file…",
    attachments: "Allegati",
    open_folder: "Apri la cartella",
    conflict_title: "Questa voce è stata cambiata altrove",
    conflict_body: "Mentre la scrivevi, il file è stato cambiato da un altro computer o programma. DiaryCraft non l'ha sovrascritto: il tuo testo è salvato in",
    use_disk: "Usa la versione sul disco",
    keep_mine: "Tieni la mia",
    compare: "Confronta",
    disk_version: "Sul disco",
    my_version: "La mia",
    close: "Chiudi",
    conflict_copies: "copie di conflitto nel diario",
    reloaded: "Voce ricaricata: era stata cambiata altrove",
    legacy_encoding: "File in codifica Windows-1252: alla prossima modifica sarà salvato in UTF-8.",
    months: [
        "gennaio",
        "febbraio",
        "marzo",
        "aprile",
        "maggio",
        "giugno",
        "luglio",
        "agosto",
        "settembre",
        "ottobre",
        "novembre",
        "dicembre",
    ],
    weekdays: ["L", "M", "M", "G", "V", "S", "D"],
    weekdays_long: ["lunedì", "martedì", "mercoledì", "giovedì", "venerdì", "sabato", "domenica"],
};

pub static EN: Strings = Strings {
    file: "File",
    open_diary: "Open diary folder…",
    use_suggested: "Use this folder",
    choose_other: "Choose another folder…",
    open_recent: "Recent diaries",
    close_diary: "Close diary",
    quit: "Quit",
    language: "Language",
    today: "Today",
    search_hint: "Search the diary",
    no_results: "No entry contains these words.",
    recent_entries: "Latest entries",
    results: "Results",
    empty_title: "No diary open",
    empty_body: "A diary is a folder: each day is a text file under year/month. DiaryCraft suggests the folder below; you can choose another one, empty to start or the folder of a diary you already have. It can be in a synced folder (OneDrive, Google Drive, Dropbox, Syncthing) to use it from several computers.",
    saved: "Saved",
    pending: "Changes not saved yet",
    words: "words",
    attach: "Attach file…",
    attachments: "Attachments",
    open_folder: "Open the folder",
    conflict_title: "This entry was changed elsewhere",
    conflict_body: "While you were writing, the file was changed by another computer or program. DiaryCraft did not overwrite it: your text is saved in",
    use_disk: "Use the version on disk",
    keep_mine: "Keep mine",
    compare: "Compare",
    disk_version: "On disk",
    my_version: "Mine",
    close: "Close",
    conflict_copies: "conflict copies in the diary",
    reloaded: "Entry reloaded: it was changed elsewhere",
    legacy_encoding: "File in Windows-1252 encoding: it will be saved in UTF-8 at the next change.",
    months: [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ],
    weekdays: ["M", "T", "W", "T", "F", "S", "S"],
    weekdays_long: ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"],
};

impl Strings {
    pub fn month(&self, m: u32) -> &'static str {
        self.months.get(m.saturating_sub(1) as usize).copied().unwrap_or("")
    }

    /// "martedì 24 novembre 2026" / "Tuesday 24 November 2026".
    pub fn long_date(&self, date: chrono::NaiveDate) -> String {
        use chrono::Datelike;
        let wd = self.weekdays_long.get(date.weekday().num_days_from_monday() as usize).copied().unwrap_or("");
        format!("{wd} {} {} {}", date.day(), self.month(date.month()), date.year())
    }
}
