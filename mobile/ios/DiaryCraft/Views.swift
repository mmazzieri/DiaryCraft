import SwiftUI
import UniformTypeIdentifiers

private let isoDay: DateFormatter = {
    let f = DateFormatter()
    f.locale = Locale(identifier: "en_US_POSIX")
    f.calendar = Calendar(identifier: .gregorian)
    f.dateFormat = "yyyy-MM-dd"
    return f
}()

private func show(_ date: String) -> String {
    isoDay.date(from: date)?.formatted(date: .complete, time: .omitted) ?? date
}

struct RootView: View {
    @EnvironmentObject var diary: Diary
    @State private var picking = false
    @State private var path: [String] = []

    var body: some View {
        Group {
            if !diary.ready {
                ProgressView()
            } else if diary.folder == nil {
                VStack(spacing: 16) {
                    Text("DiaryCraft").font(.largeTitle)
                    Text("Choose your diary folder, the one your computers share, or an empty one to start a diary. It can be in iCloud Drive, in a sync app that offers its folders in Files, or on this device.")
                        .multilineTextAlignment(.center)
                    Button("Choose the folder") { picking = true }.buttonStyle(.borderedProminent)
                }
                .padding(24)
            } else {
                NavigationStack(path: $path) {
                    DaysView(pick: { picking = true }, open: { path = [$0] })
                        .navigationDestination(for: String.self) { EditorView(page: diary.page($0)) }
                }
            }
        }
        .fileImporter(isPresented: $picking, allowedContentTypes: [.folder]) { result in
            if case .success(let url) = result {
                path = []
                diary.choose(url)
            }
        }
        .alert(
            "Something went wrong",
            isPresented: Binding(get: { diary.error != nil }, set: { if !$0 { diary.error = nil } }),
            actions: { Button("Close") { diary.error = nil } },
            message: { Text(diary.error ?? "") }
        )
        .alert(
            "The entry changed elsewhere",
            isPresented: Binding(get: { diary.notice != nil }, set: { if !$0 { diary.notice = nil } }),
            actions: { Button("Close") { diary.notice = nil } },
            message: { Text("Your text is kept in \(diary.notice ?? "").") }
        )
    }
}

struct DaysView: View {
    @EnvironmentObject var diary: Diary
    let pick: () -> Void
    let open: (String) -> Void
    @State private var query = ""
    @State private var hits: [Hit]?
    @State private var choosing = false
    @State private var other = Date()

    var body: some View {
        List {
            if !diary.conflicts.isEmpty {
                Text("Copies left by conflicting changes: \(diary.conflicts.count). Compare them on a computer.").font(.footnote)
            }
            if let hits {
                if hits.isEmpty { Text("No entries.") }
                ForEach(hits) { hit in DayRow(date: hit.date, text: hit.snippet) }
            } else {
                if diary.days.isEmpty && !diary.busy { Text("No entries.") }
                ForEach(diary.days, id: \.self) { TitledRow(date: $0) }
            }
        }
        .navigationTitle("DiaryCraft")
        .searchable(text: $query)
        .task(id: query) {
            if query.trimmingCharacters(in: .whitespaces).isEmpty {
                hits = nil
                return
            }
            try? await Task.sleep(nanoseconds: 400_000_000)
            if Task.isCancelled { return }
            let found = await diary.search(query)
            if !Task.isCancelled { hits = found }
        }
        .refreshable { diary.reload() }
        .toolbar {
            ToolbarItem(placement: .navigationBarLeading) { Button("Change folder", action: pick) }
            ToolbarItemGroup(placement: .navigationBarTrailing) {
                if diary.busy { ProgressView() }
                Button { choosing = true } label: { Label("Another day", systemImage: "calendar") }
                Button { open(isoDay.string(from: Date())) } label: { Label("Today", systemImage: "square.and.pencil") }
            }
        }
        .sheet(isPresented: $choosing) {
            NavigationStack {
                DatePicker("Another day", selection: $other, displayedComponents: .date)
                    .datePickerStyle(.graphical)
                    .padding()
                    .toolbar {
                        ToolbarItem(placement: .cancellationAction) { Button("Close") { choosing = false } }
                        ToolbarItem(placement: .confirmationAction) {
                            Button("Open") {
                                choosing = false
                                open(isoDay.string(from: other))
                            }
                        }
                    }
            }
            .presentationDetents([.medium, .large])
        }
    }
}

struct DayRow: View {
    let date: String
    let text: String

    var body: some View {
        NavigationLink(value: date) {
            VStack(alignment: .leading) {
                Text(show(date)).lineLimit(1)
                if !text.isEmpty { Text(text).font(.footnote).foregroundStyle(.secondary).lineLimit(1) }
            }
        }
    }
}

struct TitledRow: View {
    @EnvironmentObject var diary: Diary
    let date: String
    @State private var title = ""

    var body: some View {
        DayRow(date: date, text: title).task(id: diary.busy) { title = await diary.title(date) }
    }
}

struct EditorView: View {
    @EnvironmentObject var diary: Diary
    @ObservedObject var page: Page
    @Environment(\.scenePhase) private var phase

    var body: some View {
        Group {
            if page.text == nil {
                ProgressView()
            } else {
                TextEditor(text: Binding(get: { page.text ?? "" }, set: { page.text = $0 }))
                    .padding(.horizontal, 8)
                    .accessibilityLabel(show(page.date))
            }
        }
        .navigationTitle(show(page.date))
        .navigationBarTitleDisplayMode(.inline)
        .task(id: diary.busy) { await page.read() }
        .task(id: page.text) {
            try? await Task.sleep(nanoseconds: 1_000_000_000)
            // Not in this task: the next letter typed would stop the save halfway.
            if !Task.isCancelled { Task { await page.save() } }
        }
        .onDisappear { Task { await page.save() } }
        .onChange(of: phase) { now in
            if now != .active { saveInBackground() }
        }
    }

    /// Asks the system for the time to end a save when the app leaves the screen.
    private func saveInBackground() {
        var id = UIBackgroundTaskIdentifier.invalid
        id = UIApplication.shared.beginBackgroundTask { UIApplication.shared.endBackgroundTask(id) }
        Task {
            await page.save()
            UIApplication.shared.endBackgroundTask(id)
        }
    }
}
