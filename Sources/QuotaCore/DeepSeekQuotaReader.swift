import Foundation

public enum DeepSeekQuotaReader {
    public static func scriptURL(in resources: URL?) throws -> URL {
        guard let resources else { throw QuotaReadError.commandFailed }
        let script = resources.appendingPathComponent("deepseek_balance.py")
        guard FileManager.default.isReadableFile(atPath: script.path) else { throw QuotaReadError.commandFailed }
        return script
    }
    public static func sourceRoot(for executable: URL, home: URL) throws -> URL {
        var candidate = executable.resolvingSymlinksInPath().deletingLastPathComponent()
        var candidates: [URL] = []
        for _ in 0..<6 {
            candidates.append(candidate)
            candidate.deleteLastPathComponent()
        }
        candidates.append(home.appendingPathComponent(".hermes/hermes-agent"))
        guard let root = candidates.first(where: {
            FileManager.default.isReadableFile(atPath: $0.appendingPathComponent("hermes_bootstrap.py").path)
                && FileManager.default.isReadableFile(atPath: $0.appendingPathComponent("hermes_cli/config.py").path)
        }) else { throw QuotaReadError.missingHermes }
        return root.resolvingSymlinksInPath()
    }

    public static func read() throws -> DeepSeekSnapshot {
        let hermes = try HermesQuotaReader.executableURL()
        let runtimeData = try CommandRunner.run(executable: hermes,
            arguments: ["--print-runtime-command"], timeout: 8)
        let runtime = try JSONDecoder().decode([String].self, from: runtimeData)
        guard let python = runtime.first, python.hasPrefix("/"),
              FileManager.default.isExecutableFile(atPath: python) else { throw QuotaReadError.missingHermes }
        // A distributed app must never execute a helper from the builder's source path.
        let script = try scriptURL(in: Bundle.main.resourceURL)
        let userHome = FileManager.default.homeDirectoryForCurrentUser
        let home = userHome.appendingPathComponent(".hermes")
        let root = try sourceRoot(for: hermes, home: userHome)
        // Use Hermes' interpreter and resolver, not a second credential store.
        let bootstrap = "import os,sys,runpy; os.environ['HERMES_HOME']=sys.argv[1]; sys.path.insert(0,sys.argv[2]); import hermes_bootstrap; runpy.run_path(sys.argv[3],run_name='__main__')"
        let data = try CommandRunner.run(executable: URL(fileURLWithPath: python), arguments:
            ["-I", "-c", bootstrap, home.path, root.path, script.path], timeout: 22)
        return try DeepSeekSnapshot(data: data)
    }
}

public enum UsageProvider: String, CaseIterable {
    case deepseek
    case codex = "openai-codex"
    public var name: String { self == .deepseek ? "DeepSeek" : "Codex" }
}
