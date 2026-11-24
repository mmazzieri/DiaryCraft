use serde_json::{Value, json};

fn ask(request: Value) -> Value {
    let answer: Value = serde_json::from_str(&crate::call(&request.to_string())).unwrap();
    answer.get("ok").cloned().unwrap_or_else(|| panic!("{request}: {answer}"))
}

fn plan(more: Value) -> Value {
    let mut p = json!({"op": "plan", "date": "2026-11-24", "text": "new", "label": "Phone", "stamp": "s"});
    for (k, v) in more.as_object().unwrap() {
        p[k] = v.clone();
    }
    ask(p)
}

#[test]
fn entries_and_copies_are_told_apart() {
    let files =
        ["2026/11/2026-11-24.md", "2026/11/24.txt", "2026/11/2026-11-24 (1).md", "2014/12/16.txt", "notes/x.md"];
    let found = ask(json!({"op": "scan", "files": files}));
    assert_eq!(found["entries"][0], json!({"date": "2026-11-24", "path": "2026/11/2026-11-24.md"}));
    assert_eq!(found["entries"][1]["path"], "2014/12/16.txt");
    assert_eq!(found["conflicts"].as_array().unwrap().len(), 2, "the sync copy and the older naming");
}

#[test]
fn text_is_never_written_over_a_change() {
    let old = ask(json!({"op": "decode", "raw": "old"}));
    let same = plan(json!({"path": "2026/11/2026-11-24.md", "base": old["version"], "disk": "old"}));
    assert_eq!((&same["do"], &same["raw"]), (&json!("write"), &json!("new")));
    let name = "2026-11-24 (conflict Phone s).md";
    for (base, disk) in
        [(&old["version"], json!("changed")), (&old["version"], Value::Null), (&Value::Null, json!("x"))]
    {
        let p = plan(json!({"path": "2026/11/2026-11-24.md", "base": base, "disk": disk, "names": [name]}));
        assert_eq!(p["do"], "conflict");
        assert_eq!(p["path"], "2026/11/2026-11-24 (conflict Phone s-2).md", "a copy never replaces a copy");
    }
    assert_eq!(plan(json!({"text": " \n"}))["do"], "skip");
    assert_eq!(plan(json!({}))["path"], "2026/11/2026-11-24.md");
    // A save that was interrupted after writing has nothing left to do.
    assert_eq!(plan(json!({"base": old["version"], "disk": "new"}))["do"], "skip");
}

#[test]
fn windows_text_keeps_its_letters() {
    let d = ask(json!({"op": "decode", "raw": "perch\u{e9} \u{80}"}));
    assert_eq!((&d["text"], &d["encoding"]), (&json!("perché €"), &json!("windows1252")));
    let w = plan(json!({"text": "è", "base": d["version"], "disk": "perch\u{e9} \u{80}", "encoding": d["encoding"]}));
    assert_eq!(w["raw"], "\u{c3}\u{a8}", "written as UTF-8");
}

/// Junk requests are answered with errors, never a panic, through every way in.
#[test]
fn junk_never_panics() {
    for junk in [
        "",
        "null",
        "{}",
        "{\"op\":\"nothing\"}",
        "{\"op\":\"decode\",\"raw\":\"€\"}",
        "{\"op\":\"scan\",\"files\":[\"\",\"/\",\"a/b/c/d\",\"2026/11/é\"]}",
        "{\"op\":\"plan\",\"date\":\"2026-11-24\",\"path\":\"../../x.md\",\"text\":\"\",\"label\":\"\",\"stamp\":\"\"}",
        "{\"op\":\"plan\",\"date\":\"2026-11-24\",\"path\":\"2026/11/2026-11-25.md\",\"text\":\"\",\"label\":\"\",\"stamp\":\"\"}",
        "{\"op\":\"search\",\"entries\":[],\"query\":\"\\u0000\"}",
    ] {
        let answer: Value = serde_json::from_str(&crate::call(junk)).unwrap();
        assert!(answer.get("ok").is_some() != answer.get("error").is_some(), "{junk:?}");
    }
    let request = std::ffi::CString::new("{\"op\":\"summary\",\"text\":\"# A\"}").unwrap();
    // SAFETY: a valid string in, and the answer freed once.
    #[allow(unsafe_code)]
    unsafe {
        let answer = crate::c::diarycraft_call(request.as_ptr());
        assert!(std::ffi::CStr::from_ptr(answer).to_str().unwrap().contains("\"ok\""));
        crate::c::diarycraft_free(answer);
        assert!(!crate::c::diarycraft_call(std::ptr::null()).is_null());
    }
}
