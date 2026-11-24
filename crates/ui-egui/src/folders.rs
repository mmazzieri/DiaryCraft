//! The user's Documents folder, where DiaryCraft suggests keeping the diary.
//!
//! Found without extra dependencies: on Windows from the registry (`reg query`, so a Documents
//! folder that OneDrive or an administrator moved elsewhere is found too), on Linux and the BSDs
//! from the XDG user directories, on macOS as `~/Documents`.

use std::path::{Path, PathBuf};

/// The folder suggested for a new diary: `DiaryCraft` inside Documents.
pub fn suggested_diary() -> Option<PathBuf> {
    documents().map(|d| d.join("DiaryCraft"))
}

fn home() -> Option<PathBuf> {
    let var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(var).filter(|v| !v.is_empty()).map(PathBuf::from)
}

#[cfg(windows)]
pub fn documents() -> Option<PathBuf> {
    use std::os::windows::process::CommandExt;
    /// No console window flashes while `reg` runs.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let output = std::process::Command::new("reg")
        .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders", "/v", "Personal"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()
        .filter(|o| o.status.success());
    output
        .and_then(|o| parse_reg_query(&String::from_utf8_lossy(&o.stdout)))
        .filter(|p| p.is_dir())
        .or_else(|| home().map(|h| h.join("Documents")))
}

#[cfg(target_os = "macos")]
pub fn documents() -> Option<PathBuf> {
    home().map(|h| h.join("Documents"))
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn documents() -> Option<PathBuf> {
    let home = home()?;
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    let from_file = std::fs::read_to_string(config.join("user-dirs.dirs"))
        .ok()
        .and_then(|text| parse_user_dirs(&text, &home))
        .filter(|p| p.is_dir());
    Some(from_file.unwrap_or_else(|| home.join("Documents")))
}

/// Reads the value out of `reg query … /v Personal`: a line `    Personal    REG_SZ    C:\…`.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn parse_reg_query(text: &str) -> Option<PathBuf> {
    text.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix("Personal")?;
        let rest =
            rest.trim_start().strip_prefix("REG_SZ").or_else(|| rest.trim_start().strip_prefix("REG_EXPAND_SZ"))?;
        let value = rest.trim();
        (!value.is_empty()).then(|| PathBuf::from(value))
    })
}

/// Reads `XDG_DOCUMENTS_DIR="$HOME/Documenti"` out of `user-dirs.dirs`.
#[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
pub fn parse_user_dirs(text: &str, home: &Path) -> Option<PathBuf> {
    text.lines().find_map(|line| {
        let value = line.trim().strip_prefix("XDG_DOCUMENTS_DIR=")?;
        let value = value.trim().trim_matches('"');
        let path = match value.strip_prefix("$HOME") {
            Some(rest) => home.join(rest.trim_start_matches('/')),
            None if value.starts_with('/') => PathBuf::from(value),
            None => return None,
        };
        // A documents folder set to the home folder itself means "none".
        (path != home).then_some(path)
    })
}
