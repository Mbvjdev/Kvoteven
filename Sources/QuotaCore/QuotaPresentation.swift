import Foundation

public enum PetMood: String, CaseIterable {
    case happy, steady, sleepy, asleep, unknown

    public var message: String {
        switch self {
        case .happy: return "Klar på flere idéer."
        case .steady: return "Stille og roligt."
        case .sleepy: return "Snart tid til en lur."
        case .asleep: return "Jeg sover til næste reset."
        case .unknown: return "Venter på friske tal."
        }
    }
}

public struct QuotaPresentation {
    public let weeklyRemaining: Double?
    public let mood: PetMood

    public init(snapshot: QuotaSnapshot?, now: Date, fetchFailed: Bool = false) {
        let age = snapshot.map { now.timeIntervalSince($0.fetchedAt) }
        let fresh = age.map { (-60...600).contains($0) } ?? false
        let resetPassed = snapshot?.weekly?.resetsAt.map { $0 <= now } ?? false
        weeklyRemaining = fresh && !fetchFailed && !resetPassed
            ? snapshot?.weekly?.remainingPercent : nil
        switch weeklyRemaining {
        case .none: mood = .unknown
        case let .some(value) where value > 50: mood = .happy
        case let .some(value) where value > 20: mood = .steady
        case let .some(value) where value > 0: mood = .sleepy
        default: mood = .asleep
        }
    }
}
