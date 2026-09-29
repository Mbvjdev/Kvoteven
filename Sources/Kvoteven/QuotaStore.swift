import AppKit
import Combine
import QuotaCore

@MainActor
final class QuotaStore: ObservableObject {
    let demoMode: Bool
    @Published private(set) var provider: UsageProvider
    @Published private(set) var language: AppLanguage
    @Published var snapshot: QuotaSnapshot?
    @Published var deepseek: DeepSeekSnapshot?
    @Published var baseline: DeepSeekSnapshot?
    @Published var busy = false
    @Published var now = Date()
    private var failed: Set<UsageProvider> = []
    var onChange: (() -> Void)?
    private var timer: Timer?
    private var attempts: [UsageProvider: Date] = [:]
    private let languageOverride: Bool

    init(provider: UsageProvider? = nil, demoMode: Bool = false, language: AppLanguage? = nil) {
        self.demoMode = demoMode
        self.provider = provider ?? UsageProvider(rawValue: UserDefaults.standard.string(forKey: "provider") ?? "deepseek") ?? .deepseek
        if let language {
            self.language = language
            self.languageOverride = true
        } else {
            self.language = AppLanguage(rawValue: UserDefaults.standard.string(forKey: "language") ?? "system") ?? .system
            self.languageOverride = false
        }
        if demoMode {
            now = DemoSnapshots.anchor
            populateDemo()
        }
    }

    /// Resolved UI language and its localized string table.
    var resolvedLanguage: UILanguage { language.resolved }
    var l10n: L10n { L10n(language: resolvedLanguage) }

    /// Populates both providers with fixed synthetic data; never touches the
    /// network or any credential store.
    private func populateDemo() {
        deepseek = DemoSnapshots.deepSeek()
        snapshot = DemoSnapshots.codex()
        baseline = deepseek
        now = DemoSnapshots.anchor
    }

    var errorText: String? {
        failed.contains(provider)
            ? (provider == .deepseek ? l10n.text(.deepseekFetchError) : l10n.text(.codexFetchError))
            : nil
    }
    var presentation: QuotaPresentation {
        QuotaPresentation(snapshot: snapshot, now: now, fetchFailed: failed.contains(.codex))
    }
    var balancePresentation: DeepSeekPresentation {
        DeepSeekPresentation(snapshot: deepseek, now: now, fetchFailed: failed.contains(.deepseek))
    }
    var activeMood: PetMood { provider == .deepseek ? balancePresentation.mood : presentation.mood }
    var activeFetchedAt: Date? { provider == .deepseek ? deepseek?.fetchedAt : snapshot?.fetchedAt }
    var hasFreshValue: Bool { provider == .deepseek ? balancePresentation.balance != nil : presentation.weeklyRemaining != nil }
    var percentText: String {
        guard let value = presentation.weeklyRemaining else { return "—" }
        return value > 0 && value < 1 ? "<1%" : String(format: "%.0f%%", value)
    }
    func money(_ amount: Decimal, maxDigits: Int = 2) -> String {
        let formatter = NumberFormatter()
        formatter.locale = resolvedLanguage.locale
        formatter.numberStyle = .decimal
        formatter.minimumFractionDigits = 2
        formatter.maximumFractionDigits = maxDigits
        return formatter.string(from: NSDecimalNumber(decimal: amount)) ?? "—"
    }
    /// Shortened time in the selected language's locale (`.formatted` is eager
    /// and ignores the SwiftUI environment, so the locale is passed explicitly).
    func time(_ date: Date) -> String {
        date.formatted(Date.FormatStyle(date: .omitted, time: .shortened).locale(resolvedLanguage.locale))
    }
    func resetDate(_ date: Date) -> String {
        date.formatted(Date.FormatStyle()
            .weekday(.abbreviated).day().month(.abbreviated).hour().minute()
            .locale(resolvedLanguage.locale))
    }
    var menuText: String {
        guard provider == .deepseek else { return percentText }
        guard let value = balancePresentation.balance else { return "—" }
        return money(value.total) + " " + value.currency
    }
    var tooltip: String {
        provider == .deepseek
            ? l10n.text(.deepseekTooltipPrefix) + menuText
            : l10n.text(.codexTooltipPrefix) + percentText
    }

    func select(_ value: UsageProvider) {
        guard value != provider else { return }
        provider = value
        if !demoMode { UserDefaults.standard.set(value.rawValue, forKey: "provider") }
        now = demoMode ? DemoSnapshots.anchor : Date()
        onChange?()
        refresh()
    }

    func selectLanguage(_ value: AppLanguage) {
        guard value != language else { return }
        language = value
        if !languageOverride && !demoMode { UserDefaults.standard.set(value.rawValue, forKey: "language") }
        onChange?()
    }

    func start() {
        if demoMode {
            now = DemoSnapshots.anchor
            onChange?()
            return
        }
        refresh()
        timer = Timer.scheduledTimer(withTimeInterval: 30, repeats: true) { [weak self] _ in
            Task { @MainActor in
                guard let self else { return }
                self.now = Date()
                self.onChange?()
                if self.now.timeIntervalSince(self.attempts[self.provider] ?? .distantPast) >= 300 { self.refresh() }
            }
        }
        NSWorkspace.shared.notificationCenter.addObserver(self,
            selector: #selector(wokeUp), name: NSWorkspace.didWakeNotification, object: nil)
    }

    @objc private func wokeUp() { refresh() }

    // Used by the live verification/render commands. In demo mode it repopulates
    // the fixed fixtures instead of reading any provider.
    func loadSynchronously() throws {
        if demoMode {
            populateDemo()
            return
        }
        if provider == .deepseek {
            deepseek = try DeepSeekQuotaReader.read()
            baseline = deepseek
        } else {
            snapshot = try HermesQuotaReader.read()
        }
        now = Date()
    }

    func refresh() {
        guard !demoMode else { return } // demo never fetches
        let requested = provider
        guard !busy, Date().timeIntervalSince(attempts[requested] ?? .distantPast) >= 15 else { return }
        busy = true
        attempts[requested] = Date()
        Task {
            do {
                if requested == .deepseek {
                    let value = try await Task.detached(priority: .utility) { try DeepSeekQuotaReader.read() }.value
                    deepseek = value
                    if baseline == nil { baseline = value }
                } else {
                    snapshot = try await Task.detached(priority: .utility) { try HermesQuotaReader.read() }.value
                }
                failed.remove(requested)
            } catch {
                failed.insert(requested)
            }
            busy = false
            now = Date()
            onChange?()
            if provider != requested { refresh() }
        }
    }
}
