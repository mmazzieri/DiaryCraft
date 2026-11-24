import Foundation
import UIKit

/// A day as it was read: what a save compares the folder with.
struct Loaded: Equatable {
    let date: String
    var path: String?
    var base: String?
    var encoding: String?
    var text: String
}

struct Hit: Identifiable {
    var id: String { date }
    let date: String
    let snippet: String
}

/// The diary folder. File work runs on a queue of its own, one task at a time; what the screens
/// show is published on the main thread. Before a text is written it is kept in the app's own
/// storage, and leaves it only when the folder holds it.
@MainActor
final class Diary: ObservableObject {
    @Published private(set) var ready = false
    @Published private(set) var folder: URL?
    @Published private(set) var busy = false
    /// Days with an entry, newest first.
    @Published private(set) var days: [String] = []
    @Published private(set) var conflicts: [String] = []
    @Published var error: String?
    /// A conflict copy that a save wrote.
    @Published var notice: String?

    private let work = DispatchQueue(label: "diarycraft.folder")
    private let worker = Worker()
    private let defaults = UserDefaults.standard
    /// The pages opened, so that coming back to one finds the text it had, saved or not.
    private var pages: [String: Page] = [:]

    init() {
        guard let saved = defaults.data(forKey: "folder") else {
            ready = true
            return
        }
        var stale = false
        if let url = try? URL(resolvingBookmarkData: saved, bookmarkDataIsStale: &stale), url.startAccessingSecurityScopedResource() {
            if stale, let fresh = try? url.bookmarkData() { defaults.set(fresh, forKey: "folder") }
            folder = url
            reload()
        }
        ready = true
    }

    func choose(_ url: URL) {
        guard url.startAccessingSecurityScopedResource() else {
            error = String(localized: "The folder cannot be opened.")
            return
        }
        if let data = try? url.bookmarkData() { defaults.set(data, forKey: "folder") }
        folder?.stopAccessingSecurityScopedResource()
        folder = url
        pages = [:]
        days = []
        reload()
    }

    /// Looks at the folder again: other computers may have changed it.
    func reload() {
        guard let root = folder else { return }
        busy = true
        Task {
            let failures = await on { w in w.load(root) }
            busy = false
            if let first = failures?.first { error = first }
            publish()
        }
    }

    func page(_ date: String) -> Page {
        if let page = pages[date] { return page }
        let page = Page(date: date, diary: self)
        pages[date] = page
        return page
    }

    /// Reads a day, or nil when it cannot be read: it must not look like an empty one.
    func read(_ date: String) async -> Loaded? {
        await on { w in try w.read(date) }
    }

    /// Saves a text. Nil when it could not be written: the draft stays, and is tried again.
    func save(_ loaded: Loaded, _ text: String) async -> Loaded? {
        let label = UIDevice.current.name
        let saved = await on { w in try w.save(loaded, text, label: label) }
        guard let saved else { return nil }
        if let copy = saved.copy { notice = copy }
        publish()
        return saved.loaded
    }

    func title(_ date: String) async -> String {
        await on(quiet: true) { w in try w.title(date) } ?? ""
    }

    func search(_ query: String) async -> [Hit] {
        await on { w in try w.search(query) } ?? []
    }

    /// Runs `body` on the work queue; a failure is shown.
    private func on<T>(quiet: Bool = false, _ body: @escaping (Worker) throws -> T) async -> T? {
        let worker = self.worker
        let result: Result<T, Error> = await withCheckedContinuation { done in
            work.async { done.resume(returning: Result { try body(worker) }) }
        }
        switch result {
        case .success(let value): return value
        case .failure(let e):
            if !quiet { error = e.localizedDescription }
            return nil
        }
    }

    private func publish() {
        Task {
            guard let lists = await on({ w in (w.days, w.conflicts) }) else { return }
            (days, conflicts) = lists
        }
    }
}

/// What the editor of a day holds; it outlives the screen, so that leaving it still saves.
@MainActor
final class Page: ObservableObject {
    let date: String
    @Published var text: String?
    private var loaded: Loaded?
    private var saving = false
    private unowned let diary: Diary

    init(date: String, diary: Diary) {
        self.date = date
        self.diary = diary
    }

    /// Reads the day again, unless something is waiting to be saved.
    func read() async {
        let before = loaded
        guard before == nil || text == before?.text else { return }
        guard let fresh = await diary.read(date) else { return }
        guard loaded == before, text == before?.text else { return }
        loaded = fresh
        text = fresh.text
    }

    func save() async {
        guard !saving else { return }
        saving = true
        defer { saving = false }
        // What is written during a save is saved by the next turn.
        while let base = loaded, let written = text, written != base.text {
            guard let saved = await diary.save(base, written) else { return }
            loaded = saved
            // After a conflict the editor goes on from the folder's text, unless more was written.
            if text == written { text = saved.text }
        }
    }
}

/// The folder's file work, done on one queue.
final class Worker: @unchecked Sendable {
    private(set) var days: [String] = []
    private(set) var conflicts: [String] = []
    private var root: URL?
    private var paths: [String: String] = [:]
    /// Texts read for the list and the search, with the time of the file.
    private var texts: [String: (Date, String)] = [:]
    private let manager = FileManager.default

    /// Looks at the folder, after writing the texts an earlier save could not. Answers what failed.
    func load(_ url: URL) -> [String] {
        if root != url { texts = [:] }
        root = url
        var failures = recover()
        do { try scan() } catch { failures.append(error.localizedDescription) }
        return failures
    }

    private func folder() throws -> URL {
        guard let root else { throw CoreError(message: "no folder is open") }
        return root
    }

    private func names(_ dir: URL) throws -> [String] {
        try manager.contentsOfDirectory(at: dir, includingPropertiesForKeys: nil).map { Self.name($0) }
    }

    /// The name of a file; one that iCloud has not downloaded is `.name.icloud`.
    private static func name(_ url: URL) -> String {
        let n = url.lastPathComponent
        guard n.hasPrefix("."), n.hasSuffix(".icloud") else { return n }
        return String(n.dropFirst().dropLast(7))
    }

    private func scan() throws {
        let root = try folder()
        var files: [String] = []
        for y in try names(root) where y.count == 4 && Int(y) != nil {
            let year = root.appendingPathComponent(y)
            for m in try names(year) where m.count == 2 && Int(m) != nil {
                for f in try names(year.appendingPathComponent(m)) { files.append("\(y)/\(m)/\(f)") }
            }
        }
        let answer = try Core.ask("scan", ["files": files]) as? [String: Any] ?? [:]
        let entries = answer["entries"] as? [[String: String]] ?? []
        paths = [:]
        for e in entries { if let d = e["date"], let p = e["path"] { paths[d] = p } }
        days = entries.compactMap { $0["date"] }
        conflicts = (answer["conflicts"] as? [[String: String]] ?? []).compactMap { $0["path"] }
    }

    private func raw(_ data: Data) -> String { String(data: data, encoding: .isoLatin1) ?? "" }

    private func bytes(_ raw: String) throws -> Data {
        guard let data = raw.data(using: .isoLatin1) else { throw CoreError(message: "the text could not be encoded") }
        return data
    }

    /// The content of a file, downloaded if it has to be; nil only when there is no such file.
    private func content(_ path: String) throws -> Data? {
        let url = try folder().appendingPathComponent(path)
        guard let there = try? names(url.deletingLastPathComponent()), there.contains(url.lastPathComponent) else {
            // A folder that cannot be listed must not look like a missing file.
            let dir = url.deletingLastPathComponent()
            if manager.fileExists(atPath: dir.path) { _ = try names(dir) }
            return nil
        }
        var result: Result<Data, Error> = .failure(CoreError(message: "not read"))
        var coordination: NSError?
        NSFileCoordinator().coordinate(readingItemAt: url, options: [], error: &coordination) { at in
            result = Result { try Data(contentsOf: at) }
        }
        if let coordination { throw coordination }
        return try result.get()
    }

    private func put(_ data: Data, _ path: String, new: Bool) throws {
        let url = try folder().appendingPathComponent(path)
        try manager.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        var failure: Error?
        var coordination: NSError?
        NSFileCoordinator().coordinate(writingItemAt: url, options: .forReplacing, error: &coordination) { at in
            do {
                // A new file never takes the place of one that appeared meanwhile.
                try data.write(to: at, options: new ? .withoutOverwriting : .atomic)
            } catch { failure = error }
        }
        if let coordination { throw coordination }
        if let failure { throw failure }
        guard try content(path) == data else { throw CoreError(message: "the entry was not written whole") }
    }

    func read(_ date: String) throws -> Loaded {
        guard let path = paths[date], let data = try content(path) else { return Loaded(date: date, text: "") }
        let d = try Core.ask("decode", ["raw": raw(data)]) as? [String: String] ?? [:]
        guard let text = d["text"], let version = d["version"] else { throw CoreError(message: "the entry could not be read") }
        return Loaded(date: date, path: path, base: version, encoding: d["encoding"], text: text)
    }

    func save(_ loaded: Loaded, _ text: String, label: String) throws -> (loaded: Loaded, copy: String?) {
        let draft = try keep(loaded, text)
        let saved = try write(loaded, text, label: label)
        try? manager.removeItem(at: draft)
        return saved
    }

    private func write(_ loaded: Loaded, _ text: String, label: String) throws -> (loaded: Loaded, copy: String?) {
        let parts = loaded.date.split(separator: "-")
        guard parts.count == 3 else { throw CoreError(message: "not a day") }
        let month = "\(parts[0])/\(parts[1])"
        let path = loaded.path ?? "\(month)/\(loaded.date).md"
        let disk = try content(path)
        let stamp = DateFormatter()
        stamp.locale = Locale(identifier: "en_US_POSIX")
        stamp.dateFormat = "yyyyMMdd-HHmmss"
        var request: [String: Any] = [
            "date": loaded.date, "path": path, "text": text, "label": label, "stamp": stamp.string(from: Date()),
            "names": (try? names(try folder().appendingPathComponent(month))) ?? [],
        ]
        request["base"] = loaded.base
        request["encoding"] = loaded.encoding
        request["disk"] = disk.map(raw)
        let plan = try Core.ask("plan", request) as? [String: String] ?? [:]
        switch plan["do"] {
        case "write":
            try put(try bytes(plan["raw"] ?? ""), path, new: disk == nil)
            if loaded.base == nil { try scan() }
            var now = loaded
            now.path = path
            now.base = plan["version"]
            now.text = text
            return (now, nil)
        case "conflict":
            guard let copy = plan["path"] else { throw CoreError(message: "no name for the copy") }
            try put(try bytes(plan["raw"] ?? ""), copy, new: true)
            try scan()
            return (try read(loaded.date), copy)
        case "skip":
            // Blank and new, or written already by a save that did not end.
            return (disk == nil ? loaded : try read(loaded.date), nil)
        default:
            throw CoreError(message: "the save could not be planned")
        }
    }

    private func drafts() throws -> URL {
        let base = try manager.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true)
        let dir = base.appendingPathComponent("drafts")
        try manager.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir
    }

    /// The text in the app's own storage until the folder holds it.
    private func keep(_ loaded: Loaded, _ text: String) throws -> URL {
        let place = try folder().path
        var draft = ["folder": place, "date": loaded.date, "text": text]
        draft["path"] = loaded.path
        draft["base"] = loaded.base
        draft["encoding"] = loaded.encoding
        let file = try drafts().appendingPathComponent("\(loaded.date) \(abs(stable(place))).json")
        try JSONSerialization.data(withJSONObject: draft).write(to: file, options: .atomic)
        return file
    }

    /// A number for a text that is the same at each start, unlike `hashValue`.
    private func stable(_ s: String) -> Int { s.utf8.reduce(5381) { ($0 &* 33) ^ Int($1) } >> 1 }

    private func recover() -> [String] {
        var failures: [String] = []
        guard let dir = try? drafts(), let place = root?.path else { return failures }
        for file in (try? manager.contentsOfDirectory(at: dir, includingPropertiesForKeys: nil)) ?? [] where file.pathExtension == "json" {
            // One that cannot be written now does not stop the others.
            do {
                guard let d = try JSONSerialization.jsonObject(with: Data(contentsOf: file)) as? [String: String],
                      d["folder"] == place, let date = d["date"], let text = d["text"] else { continue }
                let loaded = Loaded(date: date, path: d["path"], base: d["base"], encoding: d["encoding"], text: text)
                _ = try write(loaded, text, label: "DiaryCraft")
                try manager.removeItem(at: file)
            } catch { failures.append(error.localizedDescription) }
        }
        return failures
    }

    private func text(_ date: String) throws -> String? {
        guard let path = paths[date] else { return nil }
        let url = try folder().appendingPathComponent(path)
        let time = (try? url.resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate) ?? .distantPast
        if let (at, text) = texts[path], at == time { return text }
        let text = try read(date).text
        texts[path] = (time, text)
        return text
    }

    func title(_ date: String) throws -> String {
        let d = try Core.ask("summary", ["text": try text(date) ?? ""]) as? [String: Any]
        return d?["title"] as? String ?? ""
    }

    func search(_ query: String) throws -> [Hit] {
        let entries = days.compactMap { date in (try? text(date)).flatMap { $0 }.map { ["date": date, "text": $0] } }
        let hits = try Core.ask("search", ["entries": entries, "query": query]) as? [[String: Any]] ?? []
        return hits.compactMap { h in
            guard let date = h["date"] as? String else { return nil }
            return Hit(date: date, snippet: h["snippet"] as? String ?? "")
        }
    }
}
