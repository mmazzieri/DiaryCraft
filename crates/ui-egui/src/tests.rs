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
