import Foundation

/// Fixed, clearly synthetic fixtures for demo mode. They never touch the
/// network or any credentials, and their source label makes them impossible to
/// mistake for a live provider response.
public enum DemoSnapshots {
    /// Fixed instant so demo renders are deterministic: the same dates, the
    /// same "last fetched" and reset countdown on every run.
    public static let anchor: Date = {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime]
        return formatter.date(from: "2026-09-29T19:05:02Z")!
    }()

    /// Fixed synthetic DeepSeek balance: 42.50 USD.
    public static func deepSeek() -> DeepSeekSnapshot {
        let balance = Decimal(string: "42.50") ?? 0
        return DeepSeekSnapshot(
            provider: "deepseek",
            source: "demo_synthetic",
            fetchedAt: anchor,
            isAvailable: true,
            balances: [DeepSeekBalance(currency: "USD", total: balance, granted: 0, toppedUp: balance)])
    }

    /// Fixed synthetic Codex quota: 68% of the weekly window remaining.
    public static func codex() -> QuotaSnapshot {
        QuotaSnapshot(
            provider: "openai-codex",
            source: "demo_synthetic",
            plan: "Demo",
            fetchedAt: anchor,
            windows: [QuotaWindow(label: "Weekly", usedPercent: 32,
                                  resetsAt: anchor.addingTimeInterval(6 * 86_400))])
    }
}

// MARK: - Synthetic constructors (kept internal; demo fixtures only)

extension DeepSeekBalance {
    init(currency: String, total: Decimal, granted: Decimal, toppedUp: Decimal) {
        self.currency = currency
        self.total = total
        self.granted = granted
        self.toppedUp = toppedUp
    }
}

extension DeepSeekSnapshot {
    init(provider: String, source: String, fetchedAt: Date, isAvailable: Bool, balances: [DeepSeekBalance]) {
        self.provider = provider
        self.source = source
        self.fetchedAt = fetchedAt
        self.isAvailable = isAvailable
        self.balances = balances
    }
}

extension QuotaSnapshot {
    init(provider: String, source: String, plan: String?, fetchedAt: Date,
         windows: [QuotaWindow], unavailableReason: String? = nil) {
        self.provider = provider
        self.source = source
        self.plan = plan
        self.fetchedAt = fetchedAt
        self.windows = windows
        self.unavailableReason = unavailableReason
    }
}
