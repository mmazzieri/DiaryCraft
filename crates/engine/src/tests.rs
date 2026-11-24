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

mod markdown {
    use crate::markdown::{Mark, highlight};

    /// The marked pieces of a text, without the plain ones.
    fn marked(text: &str) -> Vec<(&str, Mark)> {
        highlight(text).into_iter().filter(|s| s.mark != Mark::Plain).map(|s| (&text[s.range], s.mark)).collect()
    }

    fn assert_covers(text: &str) {
        let spans = highlight(text);
        let mut end = 0;
        for s in &spans {
            assert_eq!(s.range.start, end, "gap or overlap in {text:?}: {spans:?}");
            assert!(s.range.start < s.range.end, "empty span in {text:?}");
            assert!(text.is_char_boundary(s.range.start) && text.is_char_boundary(s.range.end));
            end = s.range.end;
        }
        assert_eq!(end, text.len(), "{text:?} not covered: {spans:?}");
    }

    #[test]
    fn headings_emphasis_and_code() {
        assert_eq!(marked("# Oggi\n"), vec![("#", Mark::Syntax), (" Oggi\n", Mark::Heading(1))]);
        assert_eq!(marked("### Tre"), vec![("###", Mark::Syntax), (" Tre", Mark::Heading(3))]);
        assert_eq!(marked("#hashtag"), vec![]);
        assert_eq!(
            marked("un **grande** giorno"),
            vec![("**", Mark::Syntax), ("grande", Mark::Strong), ("**", Mark::Syntax)]
        );
        assert_eq!(marked("_piano_"), vec![("_", Mark::Syntax), ("piano", Mark::Emphasis), ("_", Mark::Syntax)]);
        assert_eq!(marked("snake_case_name"), vec![]);
        assert_eq!(marked("2 * 3 * 4"), vec![]);
        assert_eq!(marked("usa `cargo test` ora"), vec![("`cargo test`", Mark::Code)]);
        assert_eq!(marked("``a ` b``"), vec![("``a ` b``", Mark::Code)]);
    }

    #[test]
    fn links_and_addresses() {
        assert_eq!(
            marked("vedi [foto](2026-11-24/a.jpg)."),
            vec![
                ("[", Mark::Syntax),
                ("foto", Mark::Link),
                ("](", Mark::Syntax),
                ("2026-11-24/a.jpg", Mark::Url),
                (")", Mark::Syntax)
            ]
        );
        assert_eq!(marked("![x](y)")[0], ("![", Mark::Syntax));
        assert_eq!(marked("sito https://example.org/a."), vec![("https://example.org/a", Mark::Url)]);
        assert_eq!(
            marked("<https://example.org>"),
            vec![("<", Mark::Syntax), ("https://example.org", Mark::Url), (">", Mark::Syntax)]
        );
        assert_eq!(marked("[non chiuso"), vec![]);
    }

    #[test]
    fn blocks() {
        assert_eq!(marked("- uno\n"), vec![("- ", Mark::ListMarker)]);
        assert_eq!(marked("12. dodici"), vec![("12. ", Mark::ListMarker)]);
        assert_eq!(marked("- [x] fatto"), vec![("- [x] ", Mark::ListMarker)]);
        assert_eq!(marked("> detto\n"), vec![("> ", Mark::Syntax), ("detto\n", Mark::Quote)]);
        assert_eq!(marked("***\n"), vec![("***\n", Mark::Rule)]);
        assert_eq!(marked("- - -"), vec![("- - -", Mark::Rule)]);
        let fenced = "prima\n```rust\nlet a = *b*;\n```\ndopo *x*";
        assert_eq!(
            marked(fenced),
            vec![
                ("```rust\nlet a = *b*;\n```\n", Mark::CodeBlock),
                ("*", Mark::Syntax),
                ("x", Mark::Emphasis),
                ("*", Mark::Syntax)
            ]
        );
        // An unclosed fence runs to the end, as in CommonMark.
        assert_eq!(marked("~~~\n# no"), vec![("~~~\n# no", Mark::CodeBlock)]);
    }

    #[test]
    fn front_matter_is_one_span_and_rules_after_it_are_rules() {
        let text = "---\nluogo: Macerata\n---\n# Titolo\n---\n";
        assert_eq!(
            marked(text),
            vec![
                ("---\nluogo: Macerata\n---\n", Mark::FrontMatter),
                ("#", Mark::Syntax),
                (" Titolo\n", Mark::Heading(1)),
                ("---\n", Mark::Rule)
            ]
        );
    }

    #[test]
    fn any_text_is_covered_on_character_boundaries() {
        for text in [
            "",
            "\n",
            "è",
            "**",
            "*è*",
            "_é_x",
            "[",
            "[]()",
            "![",
            "`",
            "<",
            "<a:>",
            "#",
            "> ",
            "-",
            "1.",
            "- [",
            "http://",
            "€**€**€",
            "a\r\nb\r\n",
        ] {
            assert_covers(text);
        }
        // Pseudo-random texts from pieces that start marks, with multi-byte letters among them.
        let pieces = [
            "*", "**", "_", "`", "``", "[", "]", "(", ")", "!", "<", ">", "#", "-", " ", "\n", "è", "€", "a", "1.",
            "http://x", "~~~", "```", "---\n",
        ];
        let mut seed: u64 = 0x2026_1124;
        for _ in 0..3000 {
            let mut text = String::new();
            for _ in 0..(seed % 40) {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                text.push_str(pieces[(seed >> 33) as usize % pieces.len()]);
            }
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            assert_covers(&text);
        }
    }
}

#[test]
fn links_of_the_formatted_view() {
    use crate::markdown::{LinkTarget, link_target};
    let rel = |p: &[&str]| Some(LinkTarget::Relative(p.iter().map(|s| (*s).to_owned()).collect()));
    assert_eq!(link_target("2026-11-24/foto mare.jpg"), rel(&["2026-11-24", "foto mare.jpg"]));
    assert_eq!(link_target("2026-11-24/foto%20mare.jpg"), rel(&["2026-11-24", "foto mare.jpg"]));
    assert_eq!(link_target("./a.pdf#page=2"), rel(&["a.pdf"]));
    assert_eq!(link_target("https://example.org/x"), Some(LinkTarget::Web("https://example.org/x".into())));
    assert_eq!(link_target("mailto:a@example.org"), Some(LinkTarget::Web("mailto:a@example.org".into())));
    for nowhere in [
        "../segreto.txt",
        "a/../../b",
        "/etc/passwd",
        "\\\\server\\x",
        "file:///etc/passwd",
        "javascript:alert(1)",
        "C:\\x",
        "",
        "%ff",
        "%00",
    ] {
        assert_eq!(link_target(nowhere), None, "{nowhere}");
    }
}
