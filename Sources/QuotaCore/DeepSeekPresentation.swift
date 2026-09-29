import Foundation

public struct DeepSeekPresentation {
    public let balance: DeepSeekBalance?
    public let mood: PetMood

    public init(snapshot: DeepSeekSnapshot?, now: Date, fetchFailed: Bool = false) {
        let age = snapshot.map { now.timeIntervalSince($0.fetchedAt) }
        guard !fetchFailed, age.map({ (-60...600).contains($0) }) == true else {
            balance = nil
            mood = .unknown
            return
        }
        balance = snapshot?.balances.first(where: { $0.currency == "USD" }) ?? snapshot?.balances.first
        if balance != nil && snapshot?.isAvailable == false { mood = .asleep; return }
        switch balance?.total {
        case .none: mood = .unknown
        case let .some(value) where value > 20: mood = .happy
        case let .some(value) where value >= 5: mood = .steady
        case let .some(value) where value > 0: mood = .sleepy
        default: mood = .asleep
        }
    }
}
