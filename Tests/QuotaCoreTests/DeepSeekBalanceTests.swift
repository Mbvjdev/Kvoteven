import Foundation
import XCTest
@testable import QuotaCore

final class DeepSeekBalanceTests: XCTestCase {
    // Synthetic fixtures exercise the documented schema, not the user's balance.
    func document(total: String = "42.37", available: Bool = true,
                  currency: String = "USD") -> Data {
        Data("""
        {"provider":"deepseek","source":"deepseek_balance_api",
         "fetched_at":"2026-09-29T19:20:00Z","is_available":\(available),
         "balance_infos":[{"currency":"\(currency)","total_balance":"\(total)",
         "granted_balance":"0.00","topped_up_balance":"\(total)"}]}
        """.utf8)
    }

    func testAcceptsThePythonBridgesUTCOffsetTimestamp() throws {
        let original = String(decoding: document(), as: UTF8.self)
        let shifted = original.replacingOccurrences(of: "19:20:00Z", with: "19:20:00+00:00")
        XCTAssertEqual(try DeepSeekSnapshot(data: document()).fetchedAt,
                       try DeepSeekSnapshot(data: Data(shifted.utf8)).fetchedAt)
    }

    func testRejectsMalformedMoneyAndUnexpectedCurrency() {
        for value in ["NaN", "1.2junk", "Infinity", "", "1,25"] {
            XCTAssertThrowsError(try DeepSeekSnapshot(data: document(total: value)), value)
        }
        XCTAssertThrowsError(try DeepSeekSnapshot(data: document(currency: "EUR")))
    }

    func testPetReflectsConfiguredLowBalanceThresholdNotAnImaginaryQuota() throws {
        for (total, expected) in [("88.00", PetMood.happy), ("20", .steady), ("5", .steady), ("4.99", .sleepy), ("0", .asleep), ("-0.01", .asleep)] {
            let snapshot = try DeepSeekSnapshot(data: document(total: total))
            let view = DeepSeekPresentation(snapshot: snapshot, now: snapshot.fetchedAt)
            XCTAssertEqual(view.mood, expected)
            XCTAssertEqual(view.balance?.total, Decimal(string: total))
        }
    }

    func testStaleFailedOrUnavailableApiNeverLooksHealthy() throws {
        let snapshot = try DeepSeekSnapshot(data: document())
        XCTAssertNil(DeepSeekPresentation(snapshot: snapshot, now: snapshot.fetchedAt.addingTimeInterval(601)).balance)
        XCTAssertNil(DeepSeekPresentation(snapshot: snapshot, now: snapshot.fetchedAt.addingTimeInterval(-61)).balance)
        XCTAssertNil(DeepSeekPresentation(snapshot: snapshot, now: snapshot.fetchedAt, fetchFailed: true).balance)
        let blocked = try DeepSeekSnapshot(data: document(available: false))
        XCTAssertEqual(DeepSeekPresentation(snapshot: blocked, now: blocked.fetchedAt).mood, .asleep)
    }

    func testBalanceChangeKeepsTopUpsAndSpendingDistinctWithoutMixingCurrencies() throws {
        let start = try XCTUnwrap(DeepSeekSnapshot(data: document(total: "40.00")).balances.first)
        let spent = try XCTUnwrap(DeepSeekSnapshot(data: document(total: "39.97")).balances.first)
        let toppedUp = try XCTUnwrap(DeepSeekSnapshot(data: document(total: "50.00")).balances.first)
        let otherCurrency = try XCTUnwrap(DeepSeekSnapshot(data: document(total: "50", currency: "CNY")).balances.first)
        XCTAssertEqual(spent.change(since: start), Decimal(string: "-0.03"))
        XCTAssertEqual(toppedUp.change(since: start), Decimal(string: "10.00"))
        XCTAssertNil(otherCurrency.change(since: start))
    }

    func testRejectsWrongProviderAndEmptyWallets() throws {
        let original = String(decoding: document(), as: UTF8.self)
        XCTAssertThrowsError(try DeepSeekSnapshot(data: Data(original.replacingOccurrences(of: "\"deepseek\"", with: "\"other\"").utf8)))
        let empty = Data("""
        {"provider":"deepseek","source":"deepseek_balance_api","fetched_at":"2026-09-29T19:20:00Z",
         "is_available":true,"balance_infos":[]}
        """.utf8)
        XCTAssertThrowsError(try DeepSeekSnapshot(data: empty))
    }

    func testReadsExactMoneyInsteadOfInventingAWeeklyPercentage() throws {
        let value = try DeepSeekSnapshot(data: document())
        XCTAssertEqual(value.balances.first?.total, Decimal(string: "42.37"))
        XCTAssertEqual(value.balances.first?.currency, "USD")
        XCTAssertTrue(value.isAvailable)
    }
}
