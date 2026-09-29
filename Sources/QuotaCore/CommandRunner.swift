import Foundation
import Darwin

public enum QuotaReadError: LocalizedError {
    case commandFailed
    case missingHermes
    public var errorDescription: String? {
        switch self {
        case .commandFailed: return "Kunne ikke hente kvoten. Kontrollér forbindelsen og dit Hermes-login."
        case .missingHermes: return "Hermes blev ikke fundet på denne Mac."
        }
    }
}

public enum CommandRunner {
    public static func run(executable: URL, arguments: [String], timeout: TimeInterval = 25) throws -> Data {
        let process = Process()
        process.executableURL = executable
        process.arguments = arguments
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = FileHandle.nullDevice
        process.standardInput = FileHandle.nullDevice
        try process.run()
        let watchdog = DispatchWorkItem {
            guard process.isRunning else { return }
            process.terminate()
            DispatchQueue.global().asyncAfter(deadline: .now() + 0.5) {
                if process.isRunning { kill(process.processIdentifier, SIGKILL) }
            }
        }
        DispatchQueue.global().asyncAfter(deadline: .now() + timeout, execute: watchdog)
        defer { watchdog.cancel() }
        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        guard process.terminationStatus == 0 else { throw QuotaReadError.commandFailed }
        return data
    }
}
