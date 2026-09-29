import AppKit
import SwiftUI
import QuotaCore

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    let store: QuotaStore
    private var statusItem: NSStatusItem!
    private let popover = NSPopover()
    private let smokeTest: Bool

    init(store: QuotaStore, smokeTest: Bool) {
        self.store = store
        self.smokeTest = smokeTest
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        popover.contentViewController = NSHostingController(rootView: QuotaPanel(store: store))
        popover.behavior = .transient
        popover.animates = true
        if let button = statusItem.button {
            button.target = self
            button.action = #selector(togglePopover)
            button.font = .monospacedDigitSystemFont(ofSize: 12, weight: .medium)
            button.setAccessibilityLabel(store.l10n.text(.statusAccessibility))
        }
        store.onChange = { [weak self] in self?.updateStatus() }
        updateStatus()
        store.start()
        if smokeTest {
            DispatchQueue.main.asyncAfter(deadline: .now() + 35) {
                FileHandle.standardError.write(Data("UI smoke check timed out.\n".utf8))
                exit(1)
            }
        }
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        if !popover.isShown { togglePopover() }
        return false
    }

    func updateStatus() {
        guard let button = statusItem.button else { return }
        button.title = " " + store.menuText
        let renderer = ImageRenderer(content: PetSprite(mood: store.activeMood, l10n: store.l10n).frame(width: 24, height: 24))
        renderer.scale = 2
        if let image = renderer.nsImage {
            image.size = NSSize(width: 21, height: 21)
            button.image = image
            button.imagePosition = .imageLeading
        }
        button.setAccessibilityLabel(store.l10n.text(.statusAccessibility))
        button.toolTip = store.tooltip
        button.setAccessibilityValue(store.tooltip)
        if popover.isShown { popover.contentSize = popover.contentViewController!.view.fittingSize }
        if smokeTest && store.hasFreshValue && !store.busy {
            let size = popover.contentViewController!.view.fittingSize
            guard size.width >= 300, size.height >= 300, button.image != nil,
                  button.title.contains(store.menuText) else { exit(1) }
            let result: [String: Any] = ["result": "PASS", "provider": store.provider.rawValue,
                "menu_title": button.title, "tooltip": button.toolTip ?? "", "sprite": true,
                "panel_width": size.width, "panel_height": size.height]
            if let data = try? JSONSerialization.data(withJSONObject: result, options: .sortedKeys) {
                print(String(decoding: data, as: UTF8.self))
            }
            NSApp.terminate(nil)
        }
    }

    @objc func togglePopover() {
        if popover.isShown { popover.performClose(nil); return }
        guard let button = statusItem.button else { return }
        store.now = store.demoMode ? DemoSnapshots.anchor : Date()
        updateStatus()
        popover.contentSize = popover.contentViewController!.view.fittingSize
        popover.show(relativeTo: button.bounds, of: button, preferredEdge: .minY)
        if let fetched = store.activeFetchedAt, Date().timeIntervalSince(fetched) < 300 { return }
        store.refresh()
    }
}

@main
@MainActor
struct KvotevenEntry {
    static func main() {
        let args = CommandLine.arguments
        if args.dropFirst().contains(where: { $0 == "--help" || $0 == "-h" }) {
            printHelp()
            return
        }

        var provider: UsageProvider?
        var language: AppLanguage?
        var demo = false
        var dark = false
        var renderPath: String?
        var check = false
        var smoke = false

        var i = 1
        while i < args.count {
            switch args[i] {
            case "--provider":
                guard i + 1 < args.count, let selected = UsageProvider(rawValue: args[i + 1]) else {
                    FileHandle.standardError.write(Data("Provider must be deepseek or openai-codex.\n".utf8)); exit(2)
                }
                provider = selected; i += 2
            case "--language":
                guard i + 1 < args.count, let selected = AppLanguage(rawValue: args[i + 1]) else {
                    FileHandle.standardError.write(Data("Language must be en, da, or system.\n".utf8)); exit(2)
                }
                language = selected; i += 2
            case "--demo": demo = true; i += 1
            case "--dark": dark = true; i += 1
            case "--render":
                guard i + 1 < args.count else {
                    FileHandle.standardError.write(Data("--render requires a path.\n".utf8)); exit(2)
                }
                renderPath = args[i + 1]; i += 2
            case "--check": check = true; i += 1
            case "--ui-smoke": smoke = true; i += 1
            default:
                FileHandle.standardError.write(Data("Unknown argument. Use --help for supported options.\n".utf8))
                exit(2)
            }
        }

        if check && demo {
            FileHandle.standardError.write(Data("--check cannot be combined with --demo; demo fixtures must never be reported as a live check.\n".utf8))
            exit(2)
        }

        // Language override from --language is in-memory only: the constructor
        // skips persisting when an explicit language is passed.
        let store = QuotaStore(provider: provider, demoMode: demo, language: language)

        if check {
            do {
                try store.loadSynchronously()
                var result: [String: Any] = ["provider": store.provider.rawValue, "mood": store.activeMood.rawValue,
                    "fetched_at": store.activeFetchedAt.map { ISO8601DateFormatter().string(from: $0) } ?? ""]
                if store.provider == .deepseek {
                    guard let balance = store.balancePresentation.balance else { throw QuotaReadError.commandFailed }
                    result["source"] = store.deepseek?.source
                    result["currency"] = balance.currency
                    result["balance"] = NSDecimalNumber(decimal: balance.total).stringValue
                    result["is_available"] = store.deepseek?.isAvailable
                } else {
                    guard let snapshot = store.snapshot else { throw QuotaReadError.commandFailed }
                    result["source"] = snapshot.source
                    result["plan"] = snapshot.plan
                    result["weekly_remaining"] = store.presentation.weeklyRemaining.map { $0 as Any } ?? NSNull()
                    result["resets_at"] = snapshot.weekly?.resetsAt.map { ISO8601DateFormatter().string(from: $0) }
                }
                let data = try JSONSerialization.data(withJSONObject: result, options: [.sortedKeys, .prettyPrinted])
                print(String(decoding: data, as: UTF8.self))
            } catch {
                FileHandle.standardError.write(Data("Live account check failed. No estimated data used.\n".utf8)); exit(1)
            }
            return
        }

        let app = NSApplication.shared
        app.setActivationPolicy(.accessory)

        if let path = renderPath {
            do {
                try store.loadSynchronously()
                let renderer = ImageRenderer(content: QuotaPanel(store: store)
                    .environment(\.colorScheme, dark ? .dark : .light))
                renderer.scale = 2
                guard let tiff = renderer.nsImage?.tiffRepresentation,
                      let bitmap = NSBitmapImageRep(data: tiff),
                      let png = bitmap.representation(using: .png, properties: [:]) else { throw QuotaReadError.commandFailed }
                try png.write(to: URL(fileURLWithPath: path))
                print("Rendered \(demo ? "demo" : "live") \(store.provider.name): \(store.menuText) → \(path)")
            } catch {
                FileHandle.standardError.write(Data("\(demo ? "Demo" : "Live") render failed.\n".utf8)); exit(1)
            }
            return
        }

        if !smoke, let bundleID = Bundle.main.bundleIdentifier,
           NSRunningApplication.runningApplications(withBundleIdentifier: bundleID).count > 1 { return }
        let delegate = AppDelegate(store: store, smokeTest: smoke)
        app.delegate = delegate
        withExtendedLifetime(delegate) { app.run() }
    }

    static func printHelp() {
        print("""
        Kvoteven — macOS menu bar quota & balance monitor

        Usage:
          Kvoteven [options]

        Options:
          --provider deepseek|openai-codex   Provider to show (default: remembered, else DeepSeek)
          --language en|da|system            UI language override; does not change the stored preference
          --demo                             Run in demo mode with fixed synthetic sample data (no network)
          --check                            Print LIVE account data as JSON and exit (rejects --demo)
          --render PATH                      Render the panel to a PNG at PATH and exit
          --dark                             Use dark appearance with --render
          --ui-smoke                         Run an automated UI smoke check and exit
          --help                             Show this help

        Examples:
          Kvoteven                                          Launch the menu bar app
          Kvoteven --demo                                   Launch with synthetic sample data
          Kvoteven --check --provider deepseek              Print live DeepSeek balance
          Kvoteven --check --provider openai-codex          Print live Codex quota
          Kvoteven --render shot.png --demo --provider deepseek --language en
          Kvoteven --render shot.png --demo --provider openai-codex --language da --dark
          Kvoteven --ui-smoke --demo --provider deepseek    Offline UI smoke check

        --check, --ui-smoke and --render use live account data unless --demo is given.
        --demo is explicit and never falls back automatically.
        """)
    }
}
