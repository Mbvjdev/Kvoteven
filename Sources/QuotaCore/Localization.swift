import Foundation

/// The two UI languages Kvoteven actually renders. Selection is done via
/// `AppLanguage` (which adds a `.system` choice); this is the resolved result.
public enum UILanguage: String, Equatable {
    case english = "en"
    case danish = "da"

    /// Locale used for money, dates, and other locale-aware formatting, so a
    /// language change swaps the number/date conventions immediately.
    public var locale: Locale {
        switch self {
        case .english: return Locale(identifier: "en_US")
        case .danish: return Locale(identifier: "da_DK")
        }
    }
}

/// User-facing language choice. Persisted as its raw value; the CLI `--language`
/// flag creates an in-memory override that must never write this back to
/// UserDefaults.
public enum AppLanguage: String, CaseIterable {
    case system
    case english = "en"
    case danish = "da"

    /// Resolves a language against an explicit device language, so the
    /// fallback rule is testable without depending on the host machine.
    public func resolve(preferredLanguage: String) -> UILanguage {
        switch self {
        case .english: return .english
        case .danish: return .danish
        case .system: return preferredLanguage.hasPrefix("da") ? .danish : .english
        }
    }

    /// Resolved language for the current machine. Any non-Danish system
    /// language (including English) falls back to English.
    public var resolved: UILanguage {
        resolve(preferredLanguage: Locale.preferredLanguages.first ?? "")
    }
}

/// Immutable, language-resolved string table. The app must read every piece of
/// user-visible text from here instead of relying on the Danish-only
/// `PetMood.message` in the core model.
public struct L10n {
    public let language: UILanguage

    public init(language: UILanguage) {
        self.language = language
    }

    /// Every localized string in the app, keyed identically across languages.
    public enum Key: CaseIterable {
        case statusAccessibility
        case apiBalance
        case subscription
        case showProviderPrefix
        case noFreshData
        case fetchingFromPrefix
        case refreshCadence
        case lastFetchedPrefix
        case refreshHelp
        case refreshAccessibility
        case quitHelp
        case quitAccessibility
        case deepseekTooltipPrefix
        case codexTooltipPrefix
        case deepseekFetchError
        case codexFetchError
        case moodHappy
        case moodSteady
        case moodSleepy
        case moodAsleep
        case moodUnknown
        case deepseekMoodHappy
        case deepseekMoodSteady
        case deepseekMoodSleepy
        case deepseekMoodAsleep
        case deepseekMoodUnknown
        case balanceCaption
        case apiAvailable
        case apiUnavailable
        case bonusPrefix
        case balanceChangePrefix
        case netChangeNote
        case sleepyThresholdFormat
        case alsoAvailablePrefix
        case ofWeeklyQuotaLeft
        case resetInDaysFormat
        case resetInHoursFormat
        case resetPassed
        case percentLeftFormat
        case demoBanner
        case languageLabel
        case petAccessibilityPrefix
    }

    private static let english: [Key: String] = [
        .statusAccessibility: "Kvoteven balance and quota",
        .apiBalance: "API balance",
        .subscription: "Subscription",
        .showProviderPrefix: "Show ",
        .noFreshData: "No fresh data. No estimated amount is shown.",
        .fetchingFromPrefix: "Fetching from ",
        .refreshCadence: "Updates every 5 minutes",
        .lastFetchedPrefix: "Last fetched ",
        .refreshHelp: "Refresh",
        .refreshAccessibility: "Refresh balance or quota",
        .quitHelp: "Quit Kvoteven",
        .quitAccessibility: "Quit Kvoteven",
        .deepseekTooltipPrefix: "DeepSeek API balance: ",
        .codexTooltipPrefix: "Codex weekly quota left: ",
        .deepseekFetchError: "Could not fetch the balance. Check the connection and the DeepSeek key in Hermes.",
        .codexFetchError: "Could not fetch the quota. Check the connection and your Codex login in Hermes.",
        .moodHappy: "Ready for more ideas.",
        .moodSteady: "Steady and calm.",
        .moodSleepy: "Time for a nap soon.",
        .moodAsleep: "Sleeping until the next reset.",
        .moodUnknown: "Waiting for fresh data.",
        .deepseekMoodHappy: "Plenty of energy for more ideas.",
        .deepseekMoodSteady: "There is still energy on the account.",
        .deepseekMoodSleepy: "I'm getting hungry.",
        .deepseekMoodAsleep: "Time to top up a little.",
        .deepseekMoodUnknown: "Waiting for a fresh balance.",
        .balanceCaption: "left on your DeepSeek account",
        .apiAvailable: "API calls are available",
        .apiUnavailable: "Balance is not sufficient for API calls",
        .bonusPrefix: "Of which bonus: ",
        .balanceChangePrefix: "Balance change since ",
        .netChangeNote: "Net change — top-ups and bonuses can also change the balance.",
        .sleepyThresholdFormat: "Sleepy below 5 %@ · no automatic top-up",
        .alsoAvailablePrefix: "Also available: ",
        .ofWeeklyQuotaLeft: "of weekly quota left",
        .resetInDaysFormat: "Resets in %d d %d h",
        .resetInHoursFormat: "Resets in %d h %d min",
        .resetPassed: "Reset time passed · waiting for new data",
        .percentLeftFormat: "%.0f%% left",
        .demoBanner: "Demo · sample data",
        .languageLabel: "Language",
        .petAccessibilityPrefix: "Kvoteven: ",
    ]

    private static let danish: [Key: String] = [
        .statusAccessibility: "Kvoteven saldo og kvote",
        .apiBalance: "API-saldo",
        .subscription: "Abonnement",
        .showProviderPrefix: "Vis ",
        .noFreshData: "Ingen friske tal. Der vises ikke et gættet beløb.",
        .fetchingFromPrefix: "Henter fra ",
        .refreshCadence: "Opdateres hvert 5. minut",
        .lastFetchedPrefix: "Sidst hentet ",
        .refreshHelp: "Hent igen",
        .refreshAccessibility: "Opdatér saldo eller kvote",
        .quitHelp: "Afslut Kvoteven",
        .quitAccessibility: "Afslut Kvoteven",
        .deepseekTooltipPrefix: "DeepSeek API-saldo: ",
        .codexTooltipPrefix: "Codex-ugekvote tilbage: ",
        .deepseekFetchError: "Saldoen kunne ikke hentes. Kontrollér forbindelsen og DeepSeek-nøglen i Hermes.",
        .codexFetchError: "Kvoten kunne ikke hentes. Kontrollér forbindelsen og dit Codex-login i Hermes.",
        .moodHappy: "Klar på flere idéer.",
        .moodSteady: "Stille og roligt.",
        .moodSleepy: "Snart tid til en lur.",
        .moodAsleep: "Jeg sover til næste reset.",
        .moodUnknown: "Venter på friske tal.",
        .deepseekMoodHappy: "Masser af energi til flere idéer.",
        .deepseekMoodSteady: "Der er stadig energi på kontoen.",
        .deepseekMoodSleepy: "Jeg er ved at være sulten.",
        .deepseekMoodAsleep: "Tid til at fylde lidt på.",
        .deepseekMoodUnknown: "Venter på frisk saldo.",
        .balanceCaption: "tilbage på din DeepSeek-konto",
        .apiAvailable: "API-kald er tilgængelige",
        .apiUnavailable: "Saldoen er ikke tilstrækkelig til API-kald",
        .bonusPrefix: "Heraf bonus: ",
        .balanceChangePrefix: "Saldoændring fra ",
        .netChangeNote: "Nettoændring — indbetalinger og bonus kan også ændre saldoen.",
        .sleepyThresholdFormat: "Søvnig under 5 %@ · ingen automatisk optankning",
        .alsoAvailablePrefix: "Også til rådighed: ",
        .ofWeeklyQuotaLeft: "af ugekvoten tilbage",
        .resetInDaysFormat: "Nulstilles om %d d %d t",
        .resetInHoursFormat: "Nulstilles om %d t %d min",
        .resetPassed: "Reset-tid passeret · afventer nye tal",
        .percentLeftFormat: "%.0f%% tilbage",
        .demoBanner: "Demo · eksempeldata",
        .languageLabel: "Sprog",
        .petAccessibilityPrefix: "Kvoteven: ",
    ]

    public func text(_ key: Key) -> String {
        let table = language == .english ? Self.english : Self.danish
        return table[key] ?? ""
    }

    // MARK: Moods (UI-localized, independent of the core's Danish PetMood.message)

    public func moodMessage(_ mood: PetMood) -> String {
        switch mood {
        case .happy: return text(.moodHappy)
        case .steady: return text(.moodSteady)
        case .sleepy: return text(.moodSleepy)
        case .asleep: return text(.moodAsleep)
        case .unknown: return text(.moodUnknown)
        }
    }

    public func deepSeekMoodMessage(_ mood: PetMood) -> String {
        switch mood {
        case .happy: return text(.deepseekMoodHappy)
        case .steady: return text(.deepseekMoodSteady)
        case .sleepy: return text(.deepseekMoodSleepy)
        case .asleep: return text(.deepseekMoodAsleep)
        case .unknown: return text(.deepseekMoodUnknown)
        }
    }

    // MARK: Interpolated strings

    public func resetInDays(days: Int, hours: Int) -> String {
        String(format: text(.resetInDaysFormat), days, hours)
    }

    public func resetInHours(hours: Int, minutes: Int) -> String {
        String(format: text(.resetInHoursFormat), hours, minutes)
    }

    public func sleepyThreshold(currency: String) -> String {
        String(format: text(.sleepyThresholdFormat), currency)
    }

    public func percentLeft(_ value: Double) -> String {
        String(format: text(.percentLeftFormat), value)
    }
}
