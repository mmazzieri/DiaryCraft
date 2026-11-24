use crate::calendar::{add_months, month_grid};
use crate::layout::{conflict_copy, conflict_name, parse, parts, year_month};
use crate::search::search;
use crate::text::{split_front_matter, title, word_count};
use chrono::NaiveDate;
use diarycraft_model::Layout;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

#[test]
fn standard_names_round_trip() {
    let date = d(2026, 11, 24);
    assert_eq!(parts(date, Layout::Standard), ["2026".to_owned(), "11".to_owned(), "2026-11-24.md".to_owned()]);
    let r = parse(2026, 11, "2026-11-24.md").unwrap();
    assert_eq!((r.date, r.layout), (date, Layout::Standard));
    assert_eq!(parse(2026, 11, "2026-11-24.MD").unwrap().layout, Layout::Standard);
}

#[test]
fn legacy_names_are_read() {
    assert_eq!(parse(2014, 12, "16.txt").unwrap().date, d(2014, 12, 16));
    assert_eq!(parse(2014, 12, "16.txt").unwrap().layout, Layout::LegacyDay);
    assert_eq!(parse(2014, 12, "20141216.txt").unwrap().layout, Layout::LegacyCompact);
    assert_eq!(parts(d(2014, 12, 6), Layout::LegacyDay)[2], "06.txt");
    assert_eq!(parts(d(2014, 12, 6), Layout::LegacyCompact)[2], "20141206.txt");
}

#[test]
fn names_that_are_not_entries() {
    assert!(parse(2026, 11, "2026-12-01.md").is_none(), "wrong month folder");
    assert!(parse(2026, 2, "30.txt").is_none(), "no such day");
    assert!(parse(2026, 11, "notes.md").is_none());
    assert!(parse(2026, 11, "2026-11-24 (1).md").is_none());
    assert!(parse(2026, 11, "+0026-11-24.md").is_none());
    assert!(parse(2026, 11, "2026-11-24").is_none());
    assert!(parse(2026, 11, "é.txt").is_none());
    assert!(year_month("2026", "13").is_none());
    assert!(year_month("26", "11").is_none());
    assert_eq!(year_month("2026", "02"), Some((2026, 2)));
}

#[test]
fn conflict_copies_are_recognised() {
    assert_eq!(conflict_copy(2026, 11, "2026-11-24 (1).md"), Some(d(2026, 11, 24)));
    assert_eq!(conflict_copy(2026, 11, "2026-11-24-PC.md"), Some(d(2026, 11, 24)));
    let ours = conflict_name(d(2026, 11, 24), "P18B0", "20261124-1530");
    assert_eq!(ours, "2026-11-24 (conflict P18B0 20261124-1530).md");
    assert_eq!(conflict_copy(2026, 11, &ours), Some(d(2026, 11, 24)));
    assert!(conflict_copy(2026, 11, "2026-11-24.md").is_none());
    assert!(conflict_copy(2026, 11, "2026-11-245.md").is_none());
    assert_eq!(conflict_name(d(2026, 1, 2), "../\\::", "x"), "2026-01-02 (conflict DiaryCraft x).md");
}

#[test]
fn front_matter_is_split() {
    let (front, body) = split_front_matter("---\nmood: good\n---\nHello\n");
    assert_eq!(front, Some("mood: good\n"));
    assert_eq!(body, "Hello\n");
    assert_eq!(split_front_matter("--- not front\ntext").0, None);
    assert_eq!(split_front_matter("---\nunclosed").0, None);
    assert_eq!(word_count("---\nmood: good\n---\nOggi ho scritto, finalmente - tre parole"), 6);
    assert_eq!(title("\n\n# Titolo del giorno\nresto", 40), "Titolo del giorno");
    assert_eq!(title("una riga molto lunga davvero", 10), "una riga …");
}

#[test]
fn search_finds_every_word_ignoring_case() {
    let entries = [(d(2026, 11, 23), "Pranzo con Lucia.\nPoi lavoro."), (d(2026, 11, 24), "LUCIA ha chiamato")];
    let hits = search(entries.iter().map(|(d, t)| (*d, *t)), "lucia");
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].date, d(2026, 11, 24), "newest first");
    let hits = search(entries.iter().map(|(d, t)| (*d, *t)), "lucia lavoro");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].snippet, "Pranzo con Lucia.");
    assert!(search(entries.iter().map(|(d, t)| (*d, *t)), "   ").is_empty());
}

#[test]
fn search_is_safe_with_multibyte_text() {
    let entries = [(d(2026, 1, 1), "İstanbul ß straße ẞ")];
    let hits = search(entries.iter().map(|(d, t)| (*d, *t)), "STRASSE");
    assert!(hits.is_empty());
    let hits = search(entries.iter().map(|(d, t)| (*d, *t)), "straße");
    assert_eq!(hits.len(), 1);
}

#[test]
fn month_grid_starts_on_monday() {
    let grid = month_grid(d(2026, 11, 24));
    // 1 November 2026 is a Sunday.
    assert_eq!(grid[0][6], Some(d(2026, 11, 1)));
    assert_eq!(grid[0][0], None);
    assert_eq!(grid[5][0], Some(d(2026, 11, 30)));
    let days = grid.iter().flatten().flatten().count();
    assert_eq!(days, 30);
    assert_eq!(add_months(d(2026, 1, 31), 1), d(2026, 2, 28));
    assert_eq!(add_months(d(2026, 1, 31), -1), d(2025, 12, 31));
}
