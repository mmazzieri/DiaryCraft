# Attribution

## Provenance of the design

DiaryCraft starts from the 2014 LogBook idea of its author (a root folder, one text file per day, calendar and editor, automatic saving). No code, text or assets of other diary programs were used.

## Third-party material

Rust crates are listed with their licenses by `cargo deny list`; `cargo deny check licenses` keeps them on the allowlist in `deny.toml`. Direct dependencies:

| Crate | Version | License |
|---|---|---|
| egui, eframe (and egui-wgpu, egui-winit) | 0.37 | MIT OR Apache-2.0 |
| rfd | 0.17.2 | MIT |
| winresource (build only, Windows: the icon inside the executable) | 0.1 | MIT |
| egui_commonmark (with pulldown-cmark, egui_extras) | 0.26 | MIT OR Apache-2.0 (pulldown-cmark: MIT) |
| serde, serde_json | 1 | MIT OR Apache-2.0 |
| chrono | 0.4 | MIT OR Apache-2.0 |
| sha2 | 0.10 | MIT OR Apache-2.0 |
| sys-locale | 0.3 | MIT OR Apache-2.0 |
| craft-mcp-server (storytold/craft-libs) | 0.1 | MIT OR Apache-2.0 |
| clap | 4 | MIT OR Apache-2.0 |
| jni (Android only) | 0.21 | MIT OR Apache-2.0 |
| AndroidX: Core, Activity, Lifecycle, Compose with Material 3 and its core icons (Gradle, not in this repository) | | Apache-2.0 |
| `mobile/android/gradlew`, `gradlew.bat`, `gradle/wrapper/` (Gradle wrapper) | 8.14 | Apache-2.0 |
| thiserror | 2 | MIT OR Apache-2.0 |
| tempfile (tests only) | 3 | MIT OR Apache-2.0 |

Fonts bundled in the app through `epaint_default_fonts` 0.37 (egui's `default_fonts` feature):

| Font | License |
|---|---|
| Ubuntu Light | Ubuntu Font Licence 1.0 (`Ubuntu-font-1.0`) |
| Hack Regular | MIT, with Bitstream Vera parts |
| Noto Emoji Regular | SIL Open Font License 1.1 (`OFL-1.1`) |
| emoji-icon-font | MIT |
| egui-icons (subset of emoji-icon-font) | MIT |

The license texts ship with the crate (`fonts/UFL.txt`, `fonts/OFL.txt`, `fonts/Hack-Regular.txt`, `fonts/emoji-icon-font-mit-license.txt`, `fonts/egui-icons.txt`). They are collected, unchanged, in [`THIRD-PARTY-LICENSES.txt`](THIRD-PARTY-LICENSES.txt), which every release package carries together with this file, `NOTICE` and the two license texts.

The app icon (an engraved octopus on a terracotta tile, `assets/app-icon/diarycraft-source.png`) was made by Mauro Mazzieri with Google Gemini (image generation, November 2026) for DiaryCraft, and is released under MIT OR Apache-2.0 like the code. `tools/icons.py` scales it to every size in `assets/app-icon/` and `mobile/`. The packaging scripts in `packaging/` are adapted from `storytold/craft-launcher` (MIT OR Apache-2.0), as their headers say.
