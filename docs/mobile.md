# The phone apps

`mobile/android` (Kotlin, Jetpack Compose) and `mobile/ios` (SwiftUI) are thin apps on DiaryCraft's own engine: which files are entries, how they are encoded and what a save may write are decided by the same Rust code as on the desktop. Their first version chooses the diary folder, lists the days, searches, reads and writes entries. Not yet: attachments, comparing conflict copies, a Markdown preview.

- A text is kept in the app's own storage before it is written, and leaves it only when the folder holds it. A write that stopped is tried again at the next reading of the folder.
- An entry that changed elsewhere is never overwritten: the text goes to a conflict copy, as on the desktop.
- A `.txt` entry is written back under its own name; a computer moves it to `.md`.
- **Android** reaches the folder through the system's document access. It has no atomic rename: a file is written in place and read back.
- **iOS** reads and writes through file coordination, and replaces files atomically.

## The bridge

`crates/ffi` builds a shared library for Android (`Core.call` through JNI) and a static one for iOS (`diarycraft_call`, `diarycraft_free`). One request in JSON, one answer: `{"ok": …}` or `{"error": "…"}`. File contents travel as text with one character (U+0000 to U+00FF) for each byte.

| `op` | Fields | Answers |
|---|---|---|
| `scan` | `files`: paths `YYYY/MM/name` | `{entries, conflicts}`, each `{date, path}` |
| `decode` | `raw` | `{text, encoding, version}` |
| `plan` | `date`, `path`, `base`, `encoding`, `disk`, `text`, `label`, `stamp`, `names` | `{do: skip}`, `{do: write, path, raw, version}` or `{do: conflict, path, raw}` |
| `search` | `entries`: `{date, text}`; `query` | `{date, snippet, count}` |
| `summary` | `text` | `{title, words}` |

## Building

Android, with the SDK and NDK: `rustup target add aarch64-linux-android x86_64-linux-android`, `cargo install cargo-ndk`, then `cd mobile/android && ./gradlew assembleDebug`.

iOS, on a Mac with Xcode: `rustup target add aarch64-apple-ios aarch64-apple-ios-sim`, `brew install xcodegen`, then `cd mobile/ios && xcodegen generate && open DiaryCraft.xcodeproj`.

The `Mobile` workflow builds both; the Android build is kept as an artifact.
