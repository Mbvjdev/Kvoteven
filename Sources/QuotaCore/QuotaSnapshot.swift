import Foundation

public struct QuotaWindow: Decodable, Equatable {
    public let label: String
    public let usedPercent: Double?
    public let resetsAt: Date?
    public var remainingPercent: Double? { usedPercent.map { 100 - $0 } }

    enum CodingKeys: String, CodingKey {
        case label
        case usedPercent = "used_percent"
        case resetsAt = "resets_at"
    }
}

public struct QuotaSnapshot: Decodable {
    public let provider: String
    public let source: String
    public let plan: String?
    public let fetchedAt: Date
    public let windows: [QuotaWindow]
    public let unavailableReason: String?
    public var weekly: QuotaWindow? { windows.first { $0.label.lowercased() == "weekly" } }

    enum CodingKeys: String, CodingKey {
        case provider, source, plan, windows
        case fetchedAt = "fetched_at"
        case unavailableReason = "unavailable_reason"
    }

    public init(data: Data) throws {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .custom { decoder in
            let value = try decoder.singleValueContainer().decode(String.self)
            let formatter = ISO8601DateFormatter()
            formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
            if let date = formatter.date(from: value) { return date }
            formatter.formatOptions = [.withInternetDateTime]
            if let date = formatter.date(from: value) { return date }
            throw DecodingError.dataCorrupted(.init(codingPath: decoder.codingPath,
                debugDescription: "Invalid provider timestamp"))
        }
        self = try decoder.decode(Self.self, from: data)
        guard provider == "openai-codex", unavailableReason == nil else {
            throw DecodingError.dataCorrupted(.init(codingPath: [],
                debugDescription: "No usable Codex quota response"))
        }
        guard windows.allSatisfy({ window in
            guard let used = window.usedPercent else { return true }
            return used.isFinite && (0...100).contains(used)
        }) else {
            throw DecodingError.dataCorrupted(.init(codingPath: [],
                debugDescription: "Provider percentage outside 0...100"))
        }
    }
}
