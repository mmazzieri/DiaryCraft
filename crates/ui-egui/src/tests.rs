use crate::folders::{parse_reg_query, parse_user_dirs};
use std::path::{Path, PathBuf};

#[test]
fn documents_from_the_windows_registry() {
    let out = "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Shell Folders\r\n    Personal    REG_SZ    C:\\Users\\mauro\\OneDrive\\Documenti\r\n\r\n";
    assert_eq!(parse_reg_query(out), Some(PathBuf::from("C:\\Users\\mauro\\OneDrive\\Documenti")));
    assert_eq!(parse_reg_query("ERROR: not found"), None);
}

#[test]
fn documents_from_xdg_user_dirs() {
    let home = Path::new("/home/mauro");
    let text = "# comment\nXDG_DESKTOP_DIR=\"$HOME/Scrivania\"\nXDG_DOCUMENTS_DIR=\"$HOME/Documenti\"\n";
    assert_eq!(parse_user_dirs(text, home), Some(PathBuf::from("/home/mauro/Documenti")));
    assert_eq!(parse_user_dirs("XDG_DOCUMENTS_DIR=\"/data/docs\"", home), Some(PathBuf::from("/data/docs")));
    assert_eq!(parse_user_dirs("XDG_DOCUMENTS_DIR=\"$HOME/\"", home), None);
    assert_eq!(parse_user_dirs("XDG_DOCUMENTS_DIR=\"relative\"", home), None);
}

#[test]
fn language_from_system_tags() {
    use crate::i18n::Lang;
    for tag in ["it", "it-IT", "it_IT.UTF-8", "it_CH@euro", "IT-it"] {
        assert_eq!(Lang::from_tag(tag), Lang::It, "{tag}");
    }
    for tag in ["en-US", "C", "", "de_DE.UTF-8", "ita"] {
        assert_eq!(Lang::from_tag(tag), Lang::En, "{tag}");
    }
}

#[test]
fn errors_are_described_in_italian() {
    use crate::i18n::Lang;
    let e = diarycraft_file::Error::TooNew { found: 9 };
    assert!(Lang::It.describe(&e).contains("versione più recente"));
    assert!(Lang::En.describe(&e).contains("newer"));
}

#[test]
fn links_of_the_formatted_view_stay_in_the_diary() {
    use crate::markdown::resolve_link;
    let root = Path::new("/diario");
    let dir = Path::new("/diario/2026/11");
    assert_eq!(
        resolve_link("2026-11-24/foto.jpg", dir, root),
        Some(PathBuf::from("/diario/2026/11/2026-11-24/foto.jpg"))
    );
    assert_eq!(resolve_link("https://example.org", dir, root), Some(PathBuf::from("https://example.org")));
    assert_eq!(resolve_link("../../../etc/passwd", dir, root), None);
    assert_eq!(resolve_link("file:///etc/passwd", dir, root), None);
}

#[test]
fn theme_follows_the_system_until_chosen() {
    use crate::app::{Prefs, Theme};
    // Preferences saved before the theme existed open with the system's colours.
    let old: Prefs = serde_json::from_str(r#"{"preview":true}"#).unwrap();
    assert_eq!(old.theme, Theme::System);
    let mut prefs = Prefs { theme: Theme::Dark, ..Prefs::default() };
    prefs = serde_json::from_str(&serde_json::to_string(&prefs).unwrap()).unwrap();
    assert_eq!(prefs.theme, Theme::Dark);
}
