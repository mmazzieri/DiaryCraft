import SwiftUI

@main
struct DiaryCraftApp: App {
    @StateObject private var diary = Diary()
    @Environment(\.scenePhase) private var phase

    var body: some Scene {
        WindowGroup {
            RootView().environmentObject(diary)
        }
        .onChange(of: phase) { now in
            if now == .active { diary.reload() }
        }
    }
}
