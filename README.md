# DiaryCraft

DiaryCraft is a diary that keeps each day as a plain text file. You choose a folder; every entry is a Markdown file under `year/month/` named after its day (`2026/11/2026-11-24.md`), so the diary stays readable and editable with any text editor, today and in twenty years, with or without DiaryCraft.

It is a local desktop app written in Rust with egui. The folder can be on this computer, on a shared drive, or in a folder synced by OneDrive, Google Drive, Dropbox or Syncthing to write from several computers.

> **Status:** 0.1, in development. The plan is in [docs/plan.md](docs/plan.md) (in Italian).

## Using it

- **File › Open diary folder…** opens a folder as a diary. An empty folder starts a new one; DiaryCraft adds a small `.diarycraft/config.json` with the format version and nothing else.
- The calendar on the left shows the month, with the days that have an entry highlighted. Click a day to read or write it; **Today** goes back to today.
- Under the calendar are the latest entries, or the results of the search box at the top: every word typed must appear in the entry, ignoring case.
- What you write is saved one second after you stop typing, when you move to another day and when you close the app. A day with nothing written creates no file.
- **Attach file…** copies files into the day's attachments folder (`2026/11/2026-11-24/`) and adds a link to them in the text.
- Entries from the 2014 LogBook (`year/month/DD.txt` or `year/month/YYYYMMDD.txt`, also in Windows-1252) are read and written back in place.
- Italian and English, from **File › Language**.

## Your text is never lost

Every save goes to a temporary file that is then renamed over the entry, so a crash or a full disk never leaves half a page. Before saving, DiaryCraft checks that the file on disk is still the one it read. If another computer or another program changed it meanwhile, DiaryCraft does not overwrite it: it saves your text beside it as `2026-11-24 (conflict <computer> <time>).md`, shows a notice, and lets you compare the two versions and keep either. If the entry on screen has no unsaved changes, a version changed elsewhere is simply reloaded. DiaryCraft never deletes files.

## Building

```
cargo run --release -p diarycraft -- path/to/diary
cargo xtask ci   # fmt, clippy -D warnings, tests
```

## License

MIT OR Apache-2.0, at your option. See [ATTRIBUTION.md](ATTRIBUTION.md) for third-party material.
