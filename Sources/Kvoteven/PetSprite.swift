import SwiftUI
import QuotaCore

struct PetSprite: View {
    let mood: PetMood
    let l10n: L10n
    var body: some View {
        Canvas { context, size in
            let unit = min(size.width, size.height) / 24
            let ink = Color(red: 0.19, green: 0.28, blue: 0.23)
            let body: Color = {
                switch mood {
                case .happy: return Color(red: 0.61, green: 0.83, blue: 0.65)
                case .steady: return Color(red: 0.72, green: 0.82, blue: 0.62)
                case .sleepy: return Color(red: 0.89, green: 0.80, blue: 0.56)
                case .asleep: return Color(red: 0.75, green: 0.74, blue: 0.84)
                case .unknown: return Color(red: 0.76, green: 0.78, blue: 0.77)
                }
            }()
            func box(_ x: Int, _ y: Int, _ w: Int, _ h: Int, _ color: Color) {
                context.fill(Path(CGRect(x: CGFloat(x) * unit, y: CGFloat(y) * unit,
                                         width: CGFloat(w) * unit, height: CGFloat(h) * unit)), with: .color(color))
            }
            box(5, 3, 3, 6, ink); box(16, 3, 3, 6, ink)
            box(6, 6, 12, 3, ink); box(4, 8, 16, 12, ink)
            box(3, 10, 18, 8, ink); box(6, 19, 12, 3, ink)
            box(6, 21, 4, 2, ink); box(14, 21, 4, 2, ink)
            box(20, 15, 3, 4, ink)
            box(6, 5, 1, 4, body); box(17, 5, 1, 4, body)
            box(7, 7, 10, 3, body); box(5, 9, 14, 10, body)
            box(4, 11, 16, 6, body); box(7, 19, 10, 2, body)
            box(20, 15, 1, 3, body)
            box(7, 9, 5, 1, .white.opacity(0.42))
            box(6, 16, 2, 1, Color(red: 0.90, green: 0.61, blue: 0.56))
            box(16, 16, 2, 1, Color(red: 0.90, green: 0.61, blue: 0.56))
            if mood == .asleep || mood == .sleepy {
                box(7, 14, 4, 1, ink); box(13, 14, 4, 1, ink)
                if mood == .sleepy { box(8, 13, 2, 1, ink); box(14, 13, 2, 1, ink) }
                box(11, 17, 2, 1, ink)
            } else {
                box(8, 12, 2, 3, ink); box(14, 12, 2, 3, ink)
                if mood == .unknown { box(11, 17, 2, 1, ink) }
                else {
                    box(10, 16, 1, 1, ink); box(13, 16, 1, 1, ink)
                    box(11, 17, 2, 1, ink)
                }
            }
        }
        .accessibilityLabel(l10n.text(.petAccessibilityPrefix) + l10n.moodMessage(mood))
    }
}

struct PetScene: View {
    let mood: PetMood
    let l10n: L10n
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var float = false

    var body: some View {
        VStack(spacing: 8) {
            PetSprite(mood: mood, l10n: l10n)
                .frame(width: 120, height: 120)
                .offset(y: float && !reduceMotion && mood == .happy ? -4 : 0)
                .animation(reduceMotion ? nil : .easeInOut(duration: 1.8).repeatForever(autoreverses: true), value: float)
            Ellipse().fill(Color.primary.opacity(0.08)).frame(width: 70, height: 7)
        }
        .frame(maxWidth: .infinity)
        .frame(height: 157)
        .onAppear { if !reduceMotion { float = true } }
    }
}
