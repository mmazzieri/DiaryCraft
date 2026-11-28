use super::{DiaryConfig, WeekStart};

#[test]
fn config_keeps_unknown_keys() {
    let text = r#"{"format":"diarycraft","formatVersion":1,"weekStart":"monday"}"#;
    let config: DiaryConfig = serde_json::from_str(text).unwrap();
    assert_eq!(config.format_version, 1);
    let again = serde_json::to_string(&config).unwrap();
    assert!(again.contains(r#""weekStart":"monday""#));
}

#[test]
fn week_start_is_read_written_and_survives_unknown_days() {
    let mut config = DiaryConfig::default();
    assert_eq!(config.week_start(), WeekStart::Monday);
    config.set_week_start(WeekStart::Sunday);
    let text = serde_json::to_string(&config).unwrap();
    let again: DiaryConfig = serde_json::from_str(&text).unwrap();
    assert_eq!(again.week_start(), WeekStart::Sunday);
    // A day a newer version may offer reads as Monday here and is written back untouched.
    let text = r#"{"format":"diarycraft","formatVersion":1,"weekStart":"saturday"}"#;
    let config: DiaryConfig = serde_json::from_str(text).unwrap();
    assert_eq!(config.week_start(), WeekStart::Monday);
    assert!(serde_json::to_string(&config).unwrap().contains("saturday"));
}
