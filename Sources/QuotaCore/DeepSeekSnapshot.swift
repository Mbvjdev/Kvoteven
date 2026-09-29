import Foundation

public struct DeepSeekBalance: Decodable {
    public let currency: String
    public let total: Decimal
    public let granted: Decimal
    public let toppedUp: Decimal

    /// A net balance movement, not a claim about billed consumption.
    public func change(since earlier: DeepSeekBalance) -> Decimal? {
        currency == earlier.currency ? total - earlier.total : nil
    }

    enum CodingKeys: String, CodingKey {
        case currency
        case total = "total_balance"
        case granted = "granted_balance"
        case toppedUp = "topped_up_balance"
    }

    public init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        currency = try values.decode(String.self, forKey: .currency)
        guard ["USD", "CNY"].contains(currency) else {
            throw DecodingError.dataCorruptedError(forKey: .currency, in: values, debugDescription: "Unexpected currency")
        }
        func money(_ key: CodingKeys) throws -> Decimal {
            let text = try values.decode(String.self, forKey: key)
            guard text.range(of: #"^-?[0-9]+(?:\.[0-9]+)?$"#, options: .regularExpression) == text.startIndex..<text.endIndex,
                  text.count <= 38,
                  let value = Decimal(string: text, locale: Locale(identifier: "en_US_POSIX")), !value.isNaN else {
                throw DecodingError.dataCorruptedError(forKey: key, in: values, debugDescription: "Invalid money value")
            }
            return value
        }
        total = try money(.total)
        granted = try money(.granted)
        toppedUp = try money(.toppedUp)
    }
}

public struct DeepSeekSnapshot: Decodable {
    public let provider: String
    public let source: String
    public let fetchedAt: Date
    public let isAvailable: Bool
    public let balances: [DeepSeekBalance]

    enum CodingKeys: String, CodingKey {
        case provider, source
        case fetchedAt = "fetched_at"
        case isAvailable = "is_available"
        case balances = "balance_infos"
    }

    public init(data: Data) throws {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        self = try decoder.decode(Self.self, from: data)
        guard provider == "deepseek", source == "deepseek_balance_api", !balances.isEmpty,
              Set(balances.map(\.currency)).count == balances.count else {
            throw DecodingError.dataCorrupted(.init(codingPath: [], debugDescription: "Invalid DeepSeek balance response"))
        }
    }
}
