import Foundation
import XCTest
@testable import QuotaCore

final class CommandRunnerTests: XCTestCase {
    func testNonzeroExitIsNeverTreatedAsValidOutput() {
        XCTAssertThrowsError(try CommandRunner.run(executable: URL(fileURLWithPath: "/usr/bin/false"), arguments: []))
    }

    func testStalledReadIsTerminatedWithinItsDeadline() {
        let started = Date()
        XCTAssertThrowsError(try CommandRunner.run(executable: URL(fileURLWithPath: "/bin/sleep"),
                                                   arguments: ["2"], timeout: 0.1))
        XCTAssertLessThan(Date().timeIntervalSince(started), 1.8)
    }

    func testHermesAdapterIsPinnedToReadOnlyCodexUsage() {
        XCTAssertEqual(HermesQuotaReader.arguments,
            ["--profile", "default", "usage", "--provider", "openai-codex", "--json"])
    }

    func testReadsOnlyStandardOutputFromARealProcess() throws {
        let data = try CommandRunner.run(executable: URL(fileURLWithPath: "/usr/bin/printf"),
                                         arguments: ["verified-output"])
        XCTAssertEqual(String(data: data, encoding: .utf8), "verified-output")
    }
}
