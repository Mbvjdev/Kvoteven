import Foundation

/// Hermes owns all credentials and network authentication; Kvoteven only gets quota JSON.
public enum HermesQuotaReader {
    public static let arguments = ["--profile", "default", "usage", "--provider", "openai-codex", "--json"]

    public static func executableURL() throws -> URL {
        let home = FileManager.default.homeDirectoryForCurrentUser
        let env = ProcessInfo.processInfo.environment
        var candidates: [String] = []
        if let explicit = env["KVOTEVEN_HERMES"] { candidates.append(explicit) }
        candidates += [
            home.appendingPathComponent(".local/bin/hermes").path,
            home.appendingPathComponent(".hermes/hermes-agent/.hermes/bin/hermes").path
        ]
        candidates += (env["PATH"] ?? "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin")
            .split(separator: ":").map { String($0) + "/hermes" }
        guard let path = candidates.first(where: { FileManager.default.isExecutableFile(atPath: $0) }) else {
            throw QuotaReadError.missingHermes
        }
        return URL(fileURLWithPath: path)
    }

    public static func read() throws -> QuotaSnapshot {
        let data = try CommandRunner.run(executable: executableURL(), arguments: arguments)
        return try QuotaSnapshot(data: data)
    }
}
