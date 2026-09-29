import Foundation
import XCTest
@testable import QuotaCore

final class QuotaSnapshotTests: XCTestCase {
    // Synthetic fixture; never used by the running app.
    func document(used: String = "23.0", label: String = "Weekly",
                  reset: String = "2026-10-03T18:41:18+00:00") -> Data {
        Data("""
        {"provider":"openai-codex","source":"usage_api","plan":"Prolite",
         "fetched_at":"2026-09-29T19:05:02.086492+00:00",
         "windows":[{"label":"\(label)","used_percent":\(used),"resets_at":"\(reset)"}],
         "unavailable_reason":null}
        """.utf8)
    }

    func testRejectsOutOfRangePercentInsteadOfInventingAQuota() {
        XCTAssertThrowsError(try QuotaSnapshot(data: document(used: "-1")))
        XCTAssertThrowsError(try QuotaSnapshot(data: document(used: "100.5")))
    }

    func testPetMoodTracksRemainingWeeklyQuota() throws {
        let expectations: [(String, PetMood)] = [
            ("0", .happy), ("23", .happy), ("50", .steady),
            ("79", .steady), ("80", .sleepy), ("99", .sleepy), ("100", .asleep)
        ]
        for (used, expected) in expectations {
            let snapshot = try QuotaSnapshot(data: document(used: used))
            let view = QuotaPresentation(snapshot: snapshot, now: snapshot.fetchedAt)
            XCTAssertEqual(view.mood, expected)
        }
    }

    func testOldFailedOrResetDataCannotLookLikeLiveQuota() throws {
        let snapshot = try QuotaSnapshot(data: document())
        let cases = [
            QuotaPresentation(snapshot: snapshot, now: snapshot.fetchedAt.addingTimeInterval(601)),
            QuotaPresentation(snapshot: snapshot, now: snapshot.fetchedAt, fetchFailed: true),
            QuotaPresentation(snapshot: snapshot, now: snapshot.fetchedAt.addingTimeInterval(-61))
        ]
        for view in cases {
            XCTAssertEqual(view.mood, .unknown)
            XCTAssertNil(view.weeklyRemaining)
        }
        let expired = try QuotaSnapshot(data: document(reset: "2026-09-29T19:00:00+00:00"))
        XCTAssertNil(QuotaPresentation(snapshot: expired, now: expired.fetchedAt).weeklyRemaining)
    }

    func testWrongAccountSourceOrUnavailableResponseIsRejected() throws {
        let original = String(data: document(), encoding: .utf8)!
        let wrong = original.replacingOccurrences(of: "openai-codex", with: "anthropic")
        XCTAssertThrowsError(try QuotaSnapshot(data: Data(wrong.utf8)))
        let unavailable = original.replacingOccurrences(of: "\"unavailable_reason\":null",
            with: "\"unavailable_reason\":\"login required\"")
        XCTAssertThrowsError(try QuotaSnapshot(data: Data(unavailable.utf8)))
    }

    func testWeeklyRemainderUsesProviderValue() throws {
        let snapshot = try QuotaSnapshot(data: document())
        XCTAssertEqual(snapshot.weekly?.remainingPercent, 77)
        XCTAssertEqual(snapshot.plan, "Prolite")
        XCTAssertNotNil(snapshot.weekly?.resetsAt)
    }
}
