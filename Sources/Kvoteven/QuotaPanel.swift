import AppKit
import SwiftUI
import QuotaCore

struct QuotaPanel: View {
    @ObservedObject var store: QuotaStore

    @Environment(\.colorScheme) private var colorScheme

    private var tint: Color {
        switch store.activeMood {
        case .happy, .steady:
            return colorScheme == .dark ? Color(red: 0.61, green: 0.83, blue: 0.65)
                : Color(red: 0.27, green: 0.51, blue: 0.34)
        case .sleepy: return .orange
        case .asleep: return .purple
        case .unknown: return .secondary
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if store.demoMode {
                Text(store.l10n.text(.demoBanner))
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(.orange)
                    .frame(maxWidth: .infinity, alignment: .center)
                    .padding(.vertical, 4)
                    .background(Color.orange.opacity(0.12), in: RoundedRectangle(cornerRadius: 6))
                    .padding(.bottom, 10)
            }

            HStack(alignment: .firstTextBaseline) {
                Text("Kvoteven").font(.system(size: 16, weight: .semibold))
                Spacer()
                Text(store.provider == .deepseek ? store.l10n.text(.apiBalance) : store.l10n.text(.subscription))
                    .font(.system(size: 11)).foregroundStyle(.secondary)
            }
            .padding(.bottom, 14)

            HStack(spacing: 4) {
                ForEach(UsageProvider.allCases, id: \.self) { provider in
                    Button { store.select(provider) } label: {
                        Text(provider.name).font(.system(size: 12, weight: store.provider == provider ? .semibold : .regular))
                            .frame(maxWidth: .infinity).padding(.vertical, 7)
                            .background(store.provider == provider ? Color(nsColor: .controlBackgroundColor) : .clear,
                                        in: RoundedRectangle(cornerRadius: 6))
                    }
                    .buttonStyle(.plain).accessibilityLabel(store.l10n.text(.showProviderPrefix) + provider.name)
                    .accessibilityAddTraits(store.provider == provider ? .isSelected : [])
                }
            }
            .padding(3).background(Color.primary.opacity(0.06), in: RoundedRectangle(cornerRadius: 9))
            .padding(.bottom, 10)

            HStack(spacing: 8) {
                Text(store.l10n.text(.languageLabel)).font(.system(size: 10)).foregroundStyle(.secondary)
                Spacer()
                HStack(spacing: 2) {
                    ForEach(AppLanguage.allCases, id: \.self) { language in
                        Button { store.selectLanguage(language) } label: {
                            Text(language == .system ? "System" : language == .english ? "English" : "Dansk")
                                .font(.system(size: 10, weight: store.language == language ? .semibold : .regular))
                                .padding(.horizontal, 9).padding(.vertical, 5)
                                .background(store.language == language ? Color(nsColor: .controlBackgroundColor) : .clear,
                                            in: RoundedRectangle(cornerRadius: 5))
                        }
                        .buttonStyle(.plain)
                        .accessibilityAddTraits(store.language == language ? .isSelected : [])
                    }
                }
                .padding(2).background(Color.primary.opacity(0.06), in: RoundedRectangle(cornerRadius: 7))
            }
            .padding(.bottom, 12)

            PetScene(mood: store.activeMood, l10n: store.l10n)
                .background(Color.primary.opacity(0.025), in: RoundedRectangle(cornerRadius: 12))

            Group {
                if store.provider == .deepseek { DeepSeekDetails(store: store, tint: tint) }
                else { CodexDetails(store: store, tint: tint) }
            }
            .padding(.vertical, 16)

            if let error = store.errorText {
                Text(error).font(.system(size: 11)).foregroundStyle(.orange).fixedSize(horizontal: false, vertical: true)
                    .padding(.bottom, 10)
            } else if store.activeFetchedAt != nil && !store.hasFreshValue {
                Text(store.l10n.text(.noFreshData))
                    .font(.system(size: 11)).foregroundStyle(.secondary).padding(.bottom, 10)
            }

            Divider()
            HStack {
                VStack(alignment: .leading, spacing: 3) {
                    Text(store.busy ? store.l10n.text(.fetchingFromPrefix) + store.provider.name + "…"
                                    : store.l10n.text(.refreshCadence))
                    if let fetched = store.activeFetchedAt {
                        Text(store.l10n.text(.lastFetchedPrefix) + store.time(fetched))
                    }
                }
                .font(.system(size: 10)).foregroundStyle(.secondary)
                Spacer()
                Button(action: store.refresh) {
                    Image(systemName: "arrow.clockwise").frame(width: 28, height: 28).contentShape(Rectangle())
                }
                .buttonStyle(.plain).disabled(store.busy)
                .help(store.l10n.text(.refreshHelp)).accessibilityLabel(store.l10n.text(.refreshAccessibility))
                Button { NSApp.terminate(nil) } label: {
                    Image(systemName: "power").frame(width: 28, height: 28).contentShape(Rectangle())
                }
                .buttonStyle(.plain).help(store.l10n.text(.quitHelp)).accessibilityLabel(store.l10n.text(.quitAccessibility))
            }
            .padding(.top, 12)
        }
        .padding(20)
        .frame(width: 340)
        .background(Color(nsColor: .windowBackgroundColor))
        .environment(\.locale, store.resolvedLanguage.locale)
    }
}

struct DeepSeekDetails: View {
    @ObservedObject var store: QuotaStore
    let tint: Color
    private var message: String { store.l10n.deepSeekMoodMessage(store.activeMood) }
    var body: some View {
        VStack(alignment: .leading, spacing: 9) {
            Text(message).font(.system(size: 13, weight: .medium)).foregroundStyle(tint)
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(store.balancePresentation.balance.map { store.money($0.total) } ?? "—")
                    .font(.system(size: 44, weight: .semibold, design: .rounded)).monospacedDigit()
                    .lineLimit(1).minimumScaleFactor(0.6)
                Text(store.balancePresentation.balance?.currency ?? "")
                    .font(.system(size: 16, weight: .medium)).foregroundStyle(.secondary)
            }
            Text(store.l10n.text(.balanceCaption)).font(.system(size: 12)).foregroundStyle(.secondary)

            if let balance = store.balancePresentation.balance {
                HStack {
                    Circle().fill(tint).frame(width: 6, height: 6)
                    Text(store.deepseek?.isAvailable == true ? store.l10n.text(.apiAvailable) : store.l10n.text(.apiUnavailable))
                }
                .font(.system(size: 11)).padding(.vertical, 3)
                if balance.granted > 0 {
                    Text(store.l10n.text(.bonusPrefix) + store.money(balance.granted) + " " + balance.currency)
                        .font(.system(size: 11)).foregroundStyle(.secondary)
                }
                if let earlier = store.baseline,
                   let original = earlier.balances.first(where: { $0.currency == balance.currency }),
                   let delta = balance.change(since: original) {
                    HStack(alignment: .firstTextBaseline) {
                        Text(store.l10n.text(.balanceChangePrefix) + store.time(earlier.fetchedAt))
                        Spacer()
                        Text((delta > 0 ? "+" : "") + store.money(delta, maxDigits: 6) + " " + balance.currency).monospacedDigit()
                    }
                    .font(.system(size: 11)).padding(.top, 5)
                }
                Text(store.l10n.text(.netChangeNote))
                    .font(.system(size: 10)).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                Text(store.l10n.sleepyThreshold(currency: balance.currency))
                    .font(.system(size: 10)).foregroundStyle(.secondary).padding(.top, 3)
                if let snapshot = store.deepseek {
                    ForEach(Array(snapshot.balances.enumerated()), id: \.offset) { _, extra in
                        if extra.currency != balance.currency {
                            Text(store.l10n.text(.alsoAvailablePrefix) + store.money(extra.total) + " " + extra.currency)
                                .font(.system(size: 11)).foregroundStyle(.secondary)
                        }
                    }
                }
            }
        }
    }
}

struct CodexDetails: View {
    @ObservedObject var store: QuotaStore
    let tint: Color
    private var countdown: String? {
        guard let reset = store.snapshot?.weekly?.resetsAt else { return nil }
        let seconds = Int(reset.timeIntervalSince(store.now))
        guard seconds > 0 else { return store.l10n.text(.resetPassed) }
        let days = seconds / 86_400, hours = (seconds % 86_400) / 3_600, minutes = (seconds % 3_600) / 60
        return days > 0 ? store.l10n.resetInDays(days: days, hours: hours)
                        : store.l10n.resetInHours(hours: hours, minutes: minutes)
    }
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(store.l10n.moodMessage(store.activeMood)).font(.system(size: 13, weight: .medium)).foregroundStyle(tint)
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(store.percentText).font(.system(size: 44, weight: .semibold, design: .rounded)).monospacedDigit()
                Text(store.l10n.text(.ofWeeklyQuotaLeft)).font(.system(size: 12)).foregroundStyle(.secondary)
            }
            GeometryReader { geometry in
                ZStack(alignment: .leading) {
                    Capsule().fill(Color.primary.opacity(0.07))
                    if let value = store.presentation.weeklyRemaining {
                        Capsule().fill(tint).frame(width: geometry.size.width * value / 100)
                    }
                }
            }.frame(height: 7)
            if let countdown { Text(countdown).font(.system(size: 12, weight: .medium)) }
            if let reset = store.snapshot?.weekly?.resetsAt {
                Text(store.resetDate(reset))
                    .font(.system(size: 11)).foregroundStyle(.secondary)
            }
            if store.hasFreshValue, let snapshot = store.snapshot {
                ForEach(Array(snapshot.windows.enumerated()), id: \.offset) { _, window in
                    if window.label.lowercased() != "weekly", let value = window.remainingPercent,
                       window.resetsAt.map({ $0 > store.now }) ?? true {
                        HStack { Text(window.label); Spacer(); Text(store.l10n.percentLeft(value)) }
                            .font(.system(size: 11)).foregroundStyle(.secondary)
                    }
                }
            }
        }
    }
}
