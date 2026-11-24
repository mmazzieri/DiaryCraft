use super::DiaryConfig;

#[test]
fn config_keeps_unknown_keys() {
    let text = r#"{"format":"diarycraft","formatVersion":1,"weekStart":"monday"}"#;
    let config: DiaryConfig = serde_json::from_str(text).unwrap();
    assert_eq!(config.format_version, 1);
    let again = serde_json::to_string(&config).unwrap();
    assert!(again.contains(r#""weekStart":"monday""#));
}
