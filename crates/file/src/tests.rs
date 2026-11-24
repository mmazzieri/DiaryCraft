use super::{Diary, Encoding, Error, META_DIR, Saved};
use chrono::NaiveDate;
use std::fs;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

#[test]
fn opening_an_empty_folder_creates_the_settings() {
    let dir = tempfile::tempdir().unwrap();
    let diary = Diary::open(dir.path()).unwrap();
    assert!(dir.path().join(META_DIR).join("config.json").is_file());
    assert_eq!(diary.dates().count(), 0);
}

#[test]
fn a_new_entry_is_written_under_year_and_month() {
    let dir = tempfile::tempdir().unwrap();
    let mut diary = Diary::open(dir.path()).unwrap();
    let date = d(2026, 11, 24);
    let loaded = diary.read(date).unwrap();
    assert_eq!(loaded.base, None);
    let saved = diary.save(&loaded, "Primo giorno.\n", "PC", "1").unwrap();
    assert!(matches!(saved, Saved::Written { .. }));
    let path = dir.path().join("2026").join("11").join("2026-11-24.md");
    assert_eq!(fs::read_to_string(&path).unwrap(), "Primo giorno.\n");
    assert!(diary.has_entry(date));
    assert_eq!(diary.read(date).unwrap().text, "Primo giorno.\n");
}

#[test]
fn a_blank_new_entry_creates_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut diary = Diary::open(dir.path()).unwrap();
    let loaded = diary.read(d(2026, 11, 24)).unwrap();
    assert_eq!(diary.save(&loaded, "  \n", "PC", "1").unwrap(), Saved::Skipped);
    assert!(!dir.path().join("2026").exists());
}

#[test]
fn successive_saves_do_not_conflict_with_themselves() {
    let dir = tempfile::tempdir().unwrap();
    let mut diary = Diary::open(dir.path()).unwrap();
    let mut loaded = diary.read(d(2026, 11, 24)).unwrap();
    for text in ["a", "ab", "abc"] {
        match diary.save(&loaded, text, "PC", "1").unwrap() {
            Saved::Written { base, path } => loaded.written(base, path),
            other => panic!("unexpected {other:?}"),
        }
    }
    assert_eq!(diary.read(d(2026, 11, 24)).unwrap().text, "abc");
    assert!(diary.conflict_copies().is_empty());
}

#[test]
fn a_file_changed_elsewhere_is_never_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let mut diary = Diary::open(dir.path()).unwrap();
    let date = d(2026, 11, 24);
    let loaded = diary.read(date).unwrap();
    let Saved::Written { base, .. } = diary.save(&loaded, "mine", "PC", "1").unwrap() else { panic!() };
    let loaded = super::Loaded { base: Some(base), ..diary.read(date).unwrap() };
    // Another computer writes through the sync client.
    fs::write(&loaded.path, "theirs").unwrap();
    let saved = diary.save(&loaded, "mine, edited", "P18B0", "20261124-1530").unwrap();
    let Saved::Conflict { copy, disk } = saved else { panic!("expected a conflict") };
    assert_eq!(fs::read_to_string(&loaded.path).unwrap(), "theirs");
    assert_eq!(fs::read_to_string(&copy).unwrap(), "mine, edited");
    assert_eq!(disk.text, "theirs");
    assert!(copy.file_name().unwrap().to_string_lossy().contains("conflict P18B0"));
    assert_eq!(diary.conflict_copies().len(), 1);
    // A second conflict in the same minute gets its own copy.
    let again = diary.save(&loaded, "mine, again", "P18B0", "20261124-1530").unwrap();
    let Saved::Conflict { copy: second, .. } = again else { panic!() };
    assert_ne!(copy, second);
    assert_eq!(fs::read_to_string(&copy).unwrap(), "mine, edited");
}

#[test]
fn a_file_created_elsewhere_for_a_new_day_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    let mut diary = Diary::open(dir.path()).unwrap();
    let date = d(2026, 11, 24);
    let loaded = diary.read(date).unwrap();
    fs::create_dir_all(dir.path().join("2026/11")).unwrap();
    fs::write(&loaded.path, "theirs").unwrap();
    let saved = diary.save(&loaded, "mine", "PC", "1").unwrap();
    assert!(matches!(saved, Saved::Conflict { .. }));
    assert_eq!(fs::read_to_string(&loaded.path).unwrap(), "theirs");
}

#[test]
fn legacy_entries_are_read_and_become_markdown_at_their_first_save() {
    let dir = tempfile::tempdir().unwrap();
    let month = dir.path().join("2014").join("12");
    fs::create_dir_all(&month).unwrap();
    // Windows-1252: "perché" with é = 0xE9, and a euro sign 0x80.
    fs::write(month.join("16.txt"), b"perch\xE9 \x80").unwrap();
    fs::write(month.join("20141217.txt"), "compact").unwrap();
    fs::write(month.join("note.txt"), "not an entry").unwrap();
    let mut diary = Diary::open(dir.path()).unwrap();
    assert_eq!(diary.dates().collect::<Vec<_>>(), vec![d(2014, 12, 16), d(2014, 12, 17)]);
    let mut loaded = diary.read(d(2014, 12, 16)).unwrap();
    assert_eq!(loaded.text, "perché €");
    assert_eq!(loaded.encoding, Encoding::Windows1252);
    let Saved::Written { base, path } = diary.save(&loaded, "perché € ok", "PC", "1").unwrap() else { panic!() };
    let md = month.join("2014-12-16.md");
    assert_eq!(path, md);
    assert_eq!(fs::read_to_string(&md).unwrap(), "perché € ok");
    assert!(!month.join("16.txt").exists());
    assert!(diary.conflict_copies().is_empty());
    loaded.written(base, path);
    assert_eq!(loaded.encoding, Encoding::Utf8);
    // The next save goes to the Markdown file without seeing a conflict.
    let Saved::Written { path, .. } = diary.save(&loaded, "again", "PC", "2").unwrap() else { panic!() };
    assert_eq!(path, md);
    assert_eq!(fs::read_to_string(&md).unwrap(), "again");
    // An entry nobody edits stays as it is.
    assert_eq!(fs::read_to_string(month.join("20141217.txt")).unwrap(), "compact");
    assert_eq!(fs::read_to_string(month.join("note.txt")).unwrap(), "not an entry");
}

#[test]
fn a_legacy_entry_changed_elsewhere_is_not_moved() {
    let dir = tempfile::tempdir().unwrap();
    let month = dir.path().join("2014").join("12");
    fs::create_dir_all(&month).unwrap();
    fs::write(month.join("20141216.txt"), "first").unwrap();
    let mut diary = Diary::open(dir.path()).unwrap();
    let loaded = diary.read(d(2014, 12, 16)).unwrap();
    fs::write(month.join("20141216.txt"), "theirs").unwrap();
    let Saved::Conflict { copy, disk } = diary.save(&loaded, "mine", "PC", "1").unwrap() else { panic!() };
    assert_eq!(fs::read_to_string(month.join("20141216.txt")).unwrap(), "theirs");
    assert_eq!(fs::read_to_string(copy).unwrap(), "mine");
    assert_eq!(disk.text, "theirs");
    assert!(!month.join("2014-12-16.md").exists());
}

#[test]
fn a_markdown_entry_that_appeared_meanwhile_is_not_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let month = dir.path().join("2014").join("12");
    fs::create_dir_all(&month).unwrap();
    fs::write(month.join("16.txt"), "old").unwrap();
    let mut diary = Diary::open(dir.path()).unwrap();
    let loaded = diary.read(d(2014, 12, 16)).unwrap();
    fs::write(month.join("2014-12-16.md"), "from the other computer").unwrap();
    let Saved::Written { path, .. } = diary.save(&loaded, "mine", "PC", "1").unwrap() else { panic!() };
    assert_eq!(path, month.join("16.txt"));
    assert_eq!(fs::read_to_string(month.join("16.txt")).unwrap(), "mine");
    assert_eq!(fs::read_to_string(month.join("2014-12-16.md")).unwrap(), "from the other computer");
}

#[test]
fn a_byte_order_mark_survives_the_move_to_markdown() {
    let dir = tempfile::tempdir().unwrap();
    let month = dir.path().join("2014").join("12");
    fs::create_dir_all(&month).unwrap();
    fs::write(month.join("16.txt"), b"\xEF\xBB\xBFciao").unwrap();
    let mut diary = Diary::open(dir.path()).unwrap();
    let loaded = diary.read(d(2014, 12, 16)).unwrap();
    diary.save(&loaded, "ciao!", "PC", "1").unwrap();
    assert_eq!(fs::read(month.join("2014-12-16.md")).unwrap(), b"\xEF\xBB\xBFciao!");
    assert!(!month.join("16.txt").exists());
}

#[test]
fn the_standard_name_wins_over_a_legacy_one() {
    let dir = tempfile::tempdir().unwrap();
    let month = dir.path().join("2014").join("12");
    fs::create_dir_all(&month).unwrap();
    fs::write(month.join("16.txt"), "old").unwrap();
    fs::write(month.join("2014-12-16.md"), "new").unwrap();
    let diary = Diary::open(dir.path()).unwrap();
    assert_eq!(diary.read(d(2014, 12, 16)).unwrap().text, "new");
    // The hidden plain-text file is listed with the conflict copies, so its text stays in sight.
    assert_eq!(diary.conflict_copies(), &[(d(2014, 12, 16), month.join("16.txt"))]);
}

#[test]
fn a_byte_order_mark_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    let month = dir.path().join("2026").join("01");
    fs::create_dir_all(&month).unwrap();
    fs::write(month.join("2026-01-02.md"), b"\xEF\xBB\xBFciao").unwrap();
    let mut diary = Diary::open(dir.path()).unwrap();
    let loaded = diary.read(d(2026, 1, 2)).unwrap();
    assert_eq!(loaded.text, "ciao");
    diary.save(&loaded, "ciao!", "PC", "1").unwrap();
    assert_eq!(fs::read(month.join("2026-01-02.md")).unwrap(), b"\xEF\xBB\xBFciao!");
}

#[test]
fn a_newer_format_is_refused_and_unknown_keys_survive() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join(META_DIR)).unwrap();
    let config = dir.path().join(META_DIR).join("config.json");
    fs::write(&config, r#"{"format":"diarycraft","formatVersion":9}"#).unwrap();
    assert!(matches!(Diary::open(dir.path()), Err(Error::TooNew { found: 9 })));
    fs::write(&config, r#"{"format":"diarycraft","formatVersion":1,"future":true}"#).unwrap();
    let diary = Diary::open(dir.path()).unwrap();
    assert!(diary.config().extra.contains_key("future"));
    fs::write(&config, "not json").unwrap();
    assert!(matches!(Diary::open(dir.path()), Err(Error::Config { .. })));
}

#[test]
fn search_reads_every_entry_and_attachments_are_numbered() {
    let dir = tempfile::tempdir().unwrap();
    let mut diary = Diary::open(dir.path()).unwrap();
    for (day, text) in [(1, "uno"), (2, "due")] {
        let loaded = diary.read(d(2026, 3, day)).unwrap();
        diary.save(&loaded, text, "PC", "1").unwrap();
    }
    let all = diary.load_all();
    assert_eq!(all.len(), 2);
    let source = dir.path().join("foto.jpg");
    fs::write(&source, "img").unwrap();
    let first = diary.attach(d(2026, 3, 1), &source).unwrap();
    let second = diary.attach(d(2026, 3, 1), &source).unwrap();
    assert_ne!(first, second);
    assert!(second.ends_with("2026/03/2026-03-01/foto (2).jpg"));
    assert_eq!(diary.attachments(d(2026, 3, 1)).len(), 2);
    // The attachments folder is not mistaken for an entry.
    diary.scan().unwrap();
    assert_eq!(diary.dates().count(), 2);
}

#[test]
fn opening_a_file_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("x.md");
    fs::write(&file, "x").unwrap();
    assert!(matches!(Diary::open(&file), Err(Error::NotAFolder(_))));
}
