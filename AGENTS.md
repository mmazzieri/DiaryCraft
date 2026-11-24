# AGENTS.md

Notes for people and coding agents working on DiaryCraft.

- Plan and decisions: [docs/plan.md](docs/plan.md) (Italian). Read it before changing the folder layout or how files are written.
- Layout: `crates/model` (types, pure), `crates/engine` (file names, front matter, search, calendar; pure, no I/O, no clock, must build for wasm32), `crates/file` (the diary folder: reading, atomic writes, conflict copies), `crates/ui-egui` (interface), `apps/diarycraft` (desktop shell), `xtask/`.
- **The owner's text is never lost.** Writes are atomic; a file that changed on disk since it was read is never overwritten (the text goes to a conflict copy); DiaryCraft never deletes files. A change to `crates/file` comes with a test in `crates/file/src/tests.rs`.
- Checks: `cargo xtask ci` (fmt, clippy with `-D warnings`, tests). `unsafe` is forbidden; `unwrap`, `expect`, `panic`, `todo`, `unreachable` and `unimplemented` are denied outside tests.
- Licenses: dependencies and assets only under the license allowlist of the Craft apps (MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unicode-3.0, BSL-1.0, 0BSD, Unlicense, CC0-1.0, OFL-1.1, …), checked by `cargo deny check`. No GPL, LGPL or AGPL code, not even read. Record every addition in `ATTRIBUTION.md`.
- Clean room: no code, text, icons or assets from proprietary products. Do not name other products in the interface.
- Entries are the owner's files: never rename, move or re-encode one except when the owner edits it (a Windows-1252 entry becomes UTF-8 at its first save, and only then).
