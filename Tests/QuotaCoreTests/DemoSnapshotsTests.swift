import Foundation
import XCTest
@testable import QuotaCore

final class DemoSnapshotsTests: XCTestCase {
    func testDeepSeekDemoUsesFixedSyntheticBalance() {
        let demo = DemoSnapshots.deepSeek()
        XCTAssertEqual(demo.provider, "deepseek")
        XCTAssertEqual(demo.source, "demo_synthetic")
        XCTAssertEqual(demo.fetchedAt, DemoSnapshots.anchor)
        XCTAssertTrue(demo.isAvailable)
        XCTAssertEqual(demo.balances.count, 1)
        XCTAssertEqual(demo.balances.first?.currency, "USD")
        XCTAssertEqual(demo.balances.first?.total, Decimal(string: "42.50"))
        XCTAssertEqual(demo.balances.first?.granted, 0)
        XCTAssertEqual(demo.balances.first?.toppedUp, Decimal(string: "42.50"))
    }

    func testCodexDemoUsesFixed68PercentRemaining() {
        let demo = DemoSnapshots.codex()
        XCTAssertEqual(demo.provider, "openai-codex")
        XCTAssertEqual(demo.source, "demo_synthetic")
        XCTAssertEqual(demo.weekly?.usedPercent, 32)
        XCTAssertEqual(demo.weekly?.remainingPercent, 68)
        XCTAssertGreaterThan(demo.weekly?.resetsAt?.timeIntervalSince(DemoSnapshots.anchor) ?? 0, 0)
    }

    func testDemoSnapshotsAreDeterministic() {
        XCTAssertEqual(DemoSnapshots.deepSeek().balances.first?.total,
                       DemoSnapshots.deepSeek().balances.first?.total)
        XCTAssertEqual(DemoSnapshots.codex().weekly?.remainingPercent,
                       DemoSnapshots.codex().weekly?.remainingPercent)
        XCTAssertEqual(DemoSnapshots.anchor, DemoSnapshots.anchor)
    }

    func testDemoSnapshotsReadAsFreshHappyDataAtTheAnchor() {
        let view = QuotaPresentation(snapshot: DemoSnapshots.codex(), now: DemoSnapshots.anchor)
        XCTAssertEqual(view.weeklyRemaining, 68)
        XCTAssertEqual(view.mood, .happy)

        let dsView = DeepSeekPresentation(snapshot: DemoSnapshots.deepSeek(), now: DemoSnapshots.anchor)
        XCTAssertEqual(dsView.balance?.total, Decimal(string: "42.50"))
        XCTAssertEqual(dsView.mood, .happy)
    }

    func testDemoSourceIsNeverMistakenForLiveSource() {
        XCTAssertNotEqual(DemoSnapshots.deepSeek().source, "deepseek_balance_api")
        XCTAssertNotEqual(DemoSnapshots.codex().source, "usage_api")
    }
}
