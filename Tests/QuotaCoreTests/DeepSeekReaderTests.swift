import Foundation
import XCTest
@testable import QuotaCore

final class DeepSeekReaderTests: XCTestCase {
    func testHermesSourceRootFollowsTheSelectedLauncher() throws {
        let home = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let root = home.appendingPathComponent("custom-install")
        let launcher = root.appendingPathComponent(".hermes/bin/hermes")
        try FileManager.default.createDirectory(at: launcher.deletingLastPathComponent(), withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: root.appendingPathComponent("hermes_cli"), withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: home) }
        try Data().write(to: root.appendingPathComponent("hermes_bootstrap.py"))
        try Data().write(to: root.appendingPathComponent("hermes_cli/config.py"))
        try Data().write(to: launcher)
        XCTAssertEqual(try DeepSeekQuotaReader.sourceRoot(for: launcher, home: home), root.resolvingSymlinksInPath())
        XCTAssertThrowsError(try DeepSeekQuotaReader.sourceRoot(for: home.appendingPathComponent("missing"), home: home))
    }

    func testHelperMustComeFromThePackagedResources() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        XCTAssertThrowsError(try DeepSeekQuotaReader.scriptURL(in: nil))
        XCTAssertThrowsError(try DeepSeekQuotaReader.scriptURL(in: root))
        let expected = root.appendingPathComponent("deepseek_balance.py")
        try Data("# test fixture, never executed".utf8).write(to: expected)
        XCTAssertEqual(try DeepSeekQuotaReader.scriptURL(in: root), expected)
    }
}
