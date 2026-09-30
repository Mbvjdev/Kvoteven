import type { AppLanguage, Mood, UILanguage } from "./types";

/** Resolve a user language choice against an explicit device language. */
export function resolveLanguage(lang: AppLanguage, preferredLanguage: string): UILanguage {
  if (lang === "en" || lang === "da") return lang;
  return preferredLanguage.startsWith("da") ? "da" : "en";
}

/** Resolve `.system` against the device language list (first preference wins, like the Swift app). */
export function resolveSystemLanguage(preferred: readonly string[]): UILanguage {
  return (preferred[0] ?? "").startsWith("da") ? "da" : "en";
}

export const en = {
  statusAccessibility: "Kvoteven balance and quota",
  apiBalance: "API balance",
  subscription: "Subscription",
  showProviderPrefix: "Show ",
  noFreshData: "No fresh data. No estimated amount is shown.",
  fetchingFromPrefix: "Fetching from ",
  refreshCadence: "Updates every 5 minutes",
  lastFetchedPrefix: "Last fetched ",
  refreshHelp: "Refresh",
  refreshAccessibility: "Refresh balance or quota",
  quitHelp: "Quit Kvoteven",
  quitAccessibility: "Quit Kvoteven",
  deepseekTooltipPrefix: "DeepSeek API balance: ",
  codexTooltipPrefix: "Codex weekly quota left: ",
  deepseekFetchError: "Could not fetch the balance. Check the connection and the DeepSeek key.",
  codexFetchError: "Could not fetch the quota. Check the connection and your Codex login.",
  moodHappy: "Ready for more ideas.",
  moodSteady: "Steady and calm.",
  moodSleepy: "Time for a nap soon.",
  moodAsleep: "Sleeping until the next reset.",
  moodUnknown: "Waiting for fresh data.",
  deepseekMoodHappy: "Plenty of energy for more ideas.",
  deepseekMoodSteady: "There is still energy on the account.",
  deepseekMoodSleepy: "I'm getting hungry.",
  deepseekMoodAsleep: "Time to top up a little.",
  deepseekMoodUnknown: "Waiting for a fresh balance.",
  balanceCaption: "left on your DeepSeek account",
  apiAvailable: "API calls are available",
  apiUnavailable: "Balance is not sufficient for API calls",
  balanceChangePrefix: "Balance change since app start",
  netChangeNote: "Net change — top-ups and bonuses can also change the balance.",
  sleepyThresholdFormat: "Sleepy below 5 %@ · no automatic top-up",
  ofWeeklyQuotaLeft: "of weekly quota left",
  resetInDaysFormat: "Resets in %d d %d h",
  resetInHoursFormat: "Resets in %d h %d min",
  resetPassed: "Reset time passed · waiting for new data",
  percentLeftFormat: "%d% left",
  demoBanner: "Demo · sample data",
  languageLabel: "Language",
  petAccessibilityPrefix: "Kvoteven: ",
  providerLabel: "Provider",
  codexSourceLabel: "Codex source",
  sourceAuto: "Auto",
  sourceCodexCli: "Codex CLI",
  sourceHermes: "Hermes",
  settingsTitle: "Settings",
  settingsOpen: "Open settings",
  settingsClose: "Close settings",
  deepseekKeyLabel: "DeepSeek API key",
  deepseekKeyPlaceholder: "Paste key",
  deepseekKeySave: "Save key",
  deepseekKeyDelete: "Delete key",
  deepseekKeyDeleteConfirm: "Delete the stored DeepSeek key?",
  deepseekKeySavedNote: "Stored in the OS keyring.",
  deepseekKeyMissing: "No key stored.",
  deepseekKeySaved: "Key saved.",
  deepseekKeyDeleted: "Key deleted.",
  keyringUnavailable: "The OS keyring is unavailable.",
  deepseekOnboarding: "DeepSeek uses your OS keyring (Keychain / Credential Manager / Secret Service).",
  codexOnboarding:
    "Install the official Codex CLI and run `codex login` outside the app, or use your existing Hermes default profile.",
  codexUnavailable: "Codex CLI not found.",
  hermesUnavailable: "Hermes default profile not found.",
  demoLabel: "Demo mode",
  demoUseSample: "Use sample data",
  demoUseReal: "Use real data",
  demoHint: "Network-free sample data. Never a fallback for a failed live read.",
  cancel: "Cancel",
  confirm: "Delete",
  errorGeneric: "Something went wrong.",
  errorInternal: "Something went wrong.",
  errorTimeout: "The provider took too long to respond.",
  errorNotConfigured: "DeepSeek key is not configured.",
  errorDemoRejected: "Demo mode is active; keys cannot be changed here.",
  errorInvalidKey: "That doesn't look like a valid key.",
  errorKeyringWriteFailed: "The key could not be saved to the OS keyring.",
  errorKeyringDeleteFailed: "The key could not be removed from the OS keyring.",
  errorDeepseekRequestFailed: "DeepSeek did not respond. Check your connection and key.",
  errorDeepseekBadResponse: "DeepSeek returned an unexpected response.",
  errorCodexRequestFailed: "Codex did not respond. Check your connection and login.",
  errorCodexBadResponse: "Codex returned an unexpected response.",
  errorHermesRequestFailed: "Hermes did not respond. Check your connection.",
  errorHermesBadResponse: "Hermes returned an unexpected response.",
  errorBusy: "Already checking. Please wait a moment.",
  previewRequiresDemo:
    "Browser preview needs ?demo=1. Live data is never shown without a native runtime.",
} as const;

export type L10nKey = keyof typeof en;

export const da: Record<L10nKey, string> = {
  statusAccessibility: "Kvoteven saldo og kvote",
  apiBalance: "API-saldo",
  subscription: "Abonnement",
  showProviderPrefix: "Vis ",
  noFreshData: "Ingen friske tal. Der vises ikke et gættet beløb.",
  fetchingFromPrefix: "Henter fra ",
  refreshCadence: "Opdateres hvert 5. minut",
  lastFetchedPrefix: "Sidst hentet ",
  refreshHelp: "Hent igen",
  refreshAccessibility: "Opdatér saldo eller kvote",
  quitHelp: "Afslut Kvoteven",
  quitAccessibility: "Afslut Kvoteven",
  deepseekTooltipPrefix: "DeepSeek API-saldo: ",
  codexTooltipPrefix: "Codex-ugekvote tilbage: ",
  deepseekFetchError: "Saldoen kunne ikke hentes. Kontrollér forbindelsen og DeepSeek-nøglen.",
  codexFetchError: "Kvoten kunne ikke hentes. Kontrollér forbindelsen og dit Codex-login.",
  moodHappy: "Klar på flere idéer.",
  moodSteady: "Stille og roligt.",
  moodSleepy: "Snart tid til en lur.",
  moodAsleep: "Jeg sover til næste reset.",
  moodUnknown: "Venter på friske tal.",
  deepseekMoodHappy: "Masser af energi til flere idéer.",
  deepseekMoodSteady: "Der er stadig energi på kontoen.",
  deepseekMoodSleepy: "Jeg er ved at være sulten.",
  deepseekMoodAsleep: "Tid til at fylde lidt på.",
  deepseekMoodUnknown: "Venter på frisk saldo.",
  balanceCaption: "tilbage på din DeepSeek-konto",
  apiAvailable: "API-kald er tilgængelige",
  apiUnavailable: "Saldoen er ikke tilstrækkelig til API-kald",
  balanceChangePrefix: "Saldoændring siden appstart",
  netChangeNote: "Nettoændring — indbetalinger og bonus kan også ændre saldoen.",
  sleepyThresholdFormat: "Søvnig under 5 %@ · ingen automatisk optankning",
  ofWeeklyQuotaLeft: "af ugekvoten tilbage",
  resetInDaysFormat: "Nulstilles om %d d %d t",
  resetInHoursFormat: "Nulstilles om %d t %d min",
  resetPassed: "Reset-tid passeret · afventer nye tal",
  percentLeftFormat: "%d% tilbage",
  demoBanner: "Demo · eksempeldata",
  languageLabel: "Sprog",
  petAccessibilityPrefix: "Kvoteven: ",
  providerLabel: "Udbyder",
  codexSourceLabel: "Codex-kilde",
  sourceAuto: "Auto",
  sourceCodexCli: "Codex CLI",
  sourceHermes: "Hermes",
  settingsTitle: "Indstillinger",
  settingsOpen: "Åbn indstillinger",
  settingsClose: "Luk indstillinger",
  deepseekKeyLabel: "DeepSeek API-nøgle",
  deepseekKeyPlaceholder: "Indsæt nøgle",
  deepseekKeySave: "Gem nøgle",
  deepseekKeyDelete: "Slet nøgle",
  deepseekKeyDeleteConfirm: "Slet den gemte DeepSeek-nøgle?",
  deepseekKeySavedNote: "Gemt i systemets nøglering.",
  deepseekKeyMissing: "Ingen nøgle gemt.",
  deepseekKeySaved: "Nøgle gemt.",
  deepseekKeyDeleted: "Nøgle slettet.",
  keyringUnavailable: "Systemets nøglering er ikke tilgængelig.",
  deepseekOnboarding: "DeepSeek bruger dit OS' nøglering (Keychain / Credential Manager / Secret Service).",
  codexOnboarding:
    "Installér den officielle Codex CLI og kør `codex login` uden for appen, eller brug din eksisterende Hermes standardprofil.",
  codexUnavailable: "Codex CLI blev ikke fundet.",
  hermesUnavailable: "Hermes standardprofil blev ikke fundet.",
  demoLabel: "Demotilstand",
  demoUseSample: "Brug eksempeldata",
  demoUseReal: "Brug rigtige data",
  demoHint: "Eksempeldata uden netværk. Aldrig et alternativ til en mislykket live-aflæsning.",
  cancel: "Annullér",
  confirm: "Slet",
  errorGeneric: "Noget gik galt.",
  errorInternal: "Noget gik galt.",
  errorTimeout: "Udbyderen svarede ikke i tide.",
  errorNotConfigured: "DeepSeek-nøglen er ikke konfigureret.",
  errorDemoRejected: "Demotilstand er aktiv; nøgler kan ikke ændres her.",
  errorInvalidKey: "Det ligner ikke en gyldig nøgle.",
  errorKeyringWriteFailed: "Nøglen kunne ikke gemmes i systemets nøglering.",
  errorKeyringDeleteFailed: "Nøglen kunne ikke fjernes fra systemets nøglering.",
  errorDeepseekRequestFailed: "DeepSeek svarede ikke. Tjek forbindelsen og nøglen.",
  errorDeepseekBadResponse: "DeepSeek returnerede et uventet svar.",
  errorCodexRequestFailed: "Codex svarede ikke. Tjek forbindelsen og dit login.",
  errorCodexBadResponse: "Codex returnerede et uventet svar.",
  errorHermesRequestFailed: "Hermes svarede ikke. Tjek din forbindelse.",
  errorHermesBadResponse: "Hermes returnerede et uventet svar.",
  errorBusy: "Der hentes allerede. Vent venligst et øjeblik.",
  previewRequiresDemo: "Browser-forhåndsvisning kræver ?demo=1. Live-data vises aldrig uden et native runtime.",
};

export class L10n {
  constructor(readonly language: UILanguage) {}

  text(key: L10nKey): string {
    return (this.language === "en" ? en : da)[key];
  }

  moodMessage(mood: Mood): string {
    switch (mood) {
      case "happy": return this.text("moodHappy");
      case "steady": return this.text("moodSteady");
      case "sleepy": return this.text("moodSleepy");
      case "asleep": return this.text("moodAsleep");
      case "unknown": return this.text("moodUnknown");
    }
  }

  deepSeekMoodMessage(mood: Mood): string {
    switch (mood) {
      case "happy": return this.text("deepseekMoodHappy");
      case "steady": return this.text("deepseekMoodSteady");
      case "sleepy": return this.text("deepseekMoodSleepy");
      case "asleep": return this.text("deepseekMoodAsleep");
      case "unknown": return this.text("deepseekMoodUnknown");
    }
  }

  resetInDays(days: number, hours: number): string {
    return this.text("resetInDaysFormat").replace("%d", String(days)).replace("%d", String(hours));
  }

  resetInHours(hours: number, minutes: number): string {
    return this.text("resetInHoursFormat").replace("%d", String(hours)).replace("%d", String(minutes));
  }

  sleepyThreshold(currency: string): string {
    return this.text("sleepyThresholdFormat").replace("%@", currency);
  }

  percentLeft(value: number): string {
    return this.text("percentLeftFormat").replace("%d", String(Math.round(value)));
  }
}
