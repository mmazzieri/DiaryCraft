import Foundation

/// An error answered by DiaryCraft's engine.
struct CoreError: LocalizedError {
    let message: String
    var errorDescription: String? { message }
}

/// DiaryCraft's engine, the same code as on the desktop (crates/ffi): one call, JSON in and JSON
/// out. The requests are listed in docs/mobile.md.
enum Core {
    /// Runs a request and gives back what it answers.
    @discardableResult
    static func ask(_ op: String, _ fields: [String: Any] = [:]) throws -> Any? {
        var request = fields
        request["op"] = op
        let text = String(decoding: try JSONSerialization.data(withJSONObject: request), as: UTF8.self)
        guard let raw = text.withCString({ diarycraft_call($0) }) else {
            throw CoreError(message: "the engine gave no answer")
        }
        defer { diarycraft_free(raw) }
        let data = Data(String(cString: raw).utf8)
        guard let answer = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw CoreError(message: "the engine's answer could not be read")
        }
        if let error = answer["error"] as? String { throw CoreError(message: error) }
        return answer["ok"]
    }
}
