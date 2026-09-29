import Foundation
import XCTest
@testable import QuotaCore

final class LocalizationTests: XCTestCase {
    func testSystemLanguageFallsBackToEnglishForNonDanishSystem() {
        XCTAssertEqual(AppLanguage.system.resolve(preferredLanguage: "da-DK"), .danish)
        XCTAssertEqual(AppLanguage.system.resolve(preferredLanguage: "da"), .danish)
        XCTAssertEqual(AppLanguage.system.resolve(preferredLanguage: "en-US"), .english)
        XCTAssertEqual(AppLanguage.system.resolve(preferredLanguage: "de-DE"), .english)
        XCTAssertEqual(AppLanguage.system.resolve(preferredLanguage: "en"), .english)
        XCTAssertEqual(AppLanguage.system.resolve(preferredLanguage: ""), .english)
    }

    func testExplicitLanguageOverridesSystemRegardlessOfDeviceLanguage() {
        XCTAssertEqual(AppLanguage.english.resolve(preferredLanguage: "da-DK"), .english)
        XCTAssertEqual(AppLanguage.danish.resolve(preferredLanguage: "en-US"), .danish)
    }

    func testAppLanguageRawValuesRoundTripAndRejectUnknown() {
        XCTAssertEqual(AppLanguage(rawValue: "system"), .system)
        XCTAssertEqual(AppLanguage(rawValue: "en"), .english)
        XCTAssertEqual(AppLanguage(rawValue: "da"), .danish)
        XCTAssertNil(AppLanguage(rawValue: "fr"))
        XCTAssertEqual(AppLanguage.system.rawValue, "system")
        XCTAssertEqual(AppLanguage.english.rawValue, "en")
        XCTAssertEqual(AppLanguage.danish.rawValue, "da")
    }

    func testLanguageCarriesTheLocaleUsedForMoneyAndDates() {
        XCTAssertEqual(UILanguage.english.locale.identifier, "en_US")
        XCTAssertEqual(UILanguage.danish.locale.identifier, "da_DK")
    }

    func testEveryCatalogKeyHasDistinctNonEmptyEnglishAndDanishText() {
        for key in L10n.Key.allCases {
            let en = L10n(language: .english).text(key)
            let da = L10n(language: .danish).text(key)
            XCTAssertFalse(en.isEmpty, "\(key) has empty English")
            XCTAssertFalse(da.isEmpty, "\(key) has empty Danish")
            if key != .petAccessibilityPrefix { // intentionally identical ("Kvoteven: ")
                XCTAssertNotEqual(en, da, "\(key) should differ between languages")
            }
        }
    }

    func testMoodMessagesComeFromLocalizedTableNotCoreDanishConstant() {
        let en = L10n(language: .english)
        XCTAssertEqual(en.moodMessage(.happy), "Ready for more ideas.")
        XCTAssertEqual(en.moodMessage(.asleep), "Sleeping until the next reset.")
        let da = L10n(language: .danish)
        XCTAssertEqual(da.moodMessage(.happy), "Klar på flere idéer.")
        XCTAssertEqual(da.moodMessage(.asleep), "Jeg sover til næste reset.")
        XCTAssertEqual(en.deepSeekMoodMessage(.sleepy), "I'm getting hungry.")
        XCTAssertEqual(da.deepSeekMoodMessage(.sleepy), "Jeg er ved at være sulten.")
        // Every mood localizes in both languages.
        for mood in PetMood.allCases {
            XCTAssertFalse(en.moodMessage(mood).isEmpty)
            XCTAssertFalse(da.moodMessage(mood).isEmpty)
            XCTAssertFalse(en.deepSeekMoodMessage(mood).isEmpty)
            XCTAssertFalse(da.deepSeekMoodMessage(mood).isEmpty)
        }
    }

    func testFormatHelpersLocalizeUnits() {
        XCTAssertEqual(L10n(language: .english).resetInDays(days: 2, hours: 3), "Resets in 2 d 3 h")
        XCTAssertEqual(L10n(language: .danish).resetInDays(days: 2, hours: 3), "Nulstilles om 2 d 3 t")
        XCTAssertEqual(L10n(language: .english).resetInHours(hours: 1, minutes: 5), "Resets in 1 h 5 min")
        XCTAssertEqual(L10n(language: .danish).resetInHours(hours: 1, minutes: 5), "Nulstilles om 1 t 5 min")
        XCTAssertEqual(L10n(language: .english).sleepyThreshold(currency: "USD"), "Sleepy below 5 USD · no automatic top-up")
        XCTAssertEqual(L10n(language: .danish).sleepyThreshold(currency: "USD"), "Søvnig under 5 USD · ingen automatisk optankning")
        XCTAssertEqual(L10n(language: .english).percentLeft(68), "68% left")
        XCTAssertEqual(L10n(language: .danish).percentLeft(68), "68% tilbage")
    }

    func testDemoBannerIsExplicitInBothLanguages() {
        XCTAssertEqual(L10n(language: .english).text(.demoBanner), "Demo · sample data")
        XCTAssertEqual(L10n(language: .danish).text(.demoBanner), "Demo · eksempeldata")
    }
}
