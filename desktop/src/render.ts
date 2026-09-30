import { L10n, type L10nKey } from "./i18n";
import { codexView, deepseekView } from "./aging";
import {
  formatMoneyAmount,
  formatNetChange,
  formatPercent,
  formatTime,
  formatResetDate,
  countdown,
} from "./format";
import { errorMessageKey } from "./errors";
import { petSvg } from "./pet";
import type { Store } from "./store";
import type { AppLanguage, CodexSource, Provider, UILanguage } from "./types";

type Child = Node | string;

function h<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: Record<string, string> = {},
  ...children: Child[]
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
  for (const c of children) node.append(c);
  return node;
}

function t(s: string): Text {
  return document.createTextNode(s);
}

export interface MountOptions {
  store: Store;
  root: HTMLElement;
  uiLanguage: () => UILanguage;
}

export interface MountHandle {
  dispose: () => void;
  setKeyInputValue(v: string): void;
}

const PROVIDERS: { value: Provider; name: string }[] = [
  { value: "deepseek", name: "DeepSeek" },
  { value: "openai-codex", name: "Codex" },
];

const LANGUAGES: { value: AppLanguage; name: string }[] = [
  { value: "system", name: "System" },
  { value: "en", name: "English" },
  { value: "da", name: "Dansk" },
];

const SOURCES: { value: CodexSource; key: L10nKey }[] = [
  { value: "auto", key: "sourceAuto" },
  { value: "codex", key: "sourceCodexCli" },
  { value: "hermes", key: "sourceHermes" },
];

/** Build a radio-style segmented control of native buttons with arrow-key support. */
function segmented(
  items: { value: string; label: string; ariaLabel?: string }[],
  selected: string,
  testid: string,
  onSelect: (value: string) => void,
): HTMLElement {
  const group = h("div", { class: "segmented", role: "radiogroup", "data-testid": testid });
  const selectAndFocus = (value: string) => {
    onSelect(value);
    // Full re-render replaces the buttons; restore focus on the new selection.
    queueMicrotask(() => {
      const next = document.querySelector<HTMLButtonElement>(`[data-testid="${testid}"] button[data-value="${value}"]`);
      next?.focus();
    });
  };
  items.forEach((item, idx) => {
    const selectedHere = item.value === selected;
    const btn = h("button", {
      type: "button",
      role: "radio",
      "aria-checked": selectedHere ? "true" : "false",
      class: "seg-btn",
      "data-value": item.value,
    });
    if (item.ariaLabel) btn.setAttribute("aria-label", item.ariaLabel);
    btn.append(t(item.label));
    btn.addEventListener("click", () => selectAndFocus(item.value));
    btn.addEventListener("keydown", (e) => {
      let next = -1;
      if (e.key === "ArrowRight" || e.key === "ArrowDown") next = (idx + 1) % items.length;
      else if (e.key === "ArrowLeft" || e.key === "ArrowUp") next = (idx - 1 + items.length) % items.length;
      else if (e.key === "Home") next = 0;
      else if (e.key === "End") next = items.length - 1;
      if (next >= 0 && next !== idx) {
        e.preventDefault();
        selectAndFocus(items[next].value);
      }
    });
    group.append(btn);
  });
  return group;
}

/** Quota fetch failure: actionable catalog message, else the provider generic. */
function fetchErrorMessage(provider: Provider, code: string | null, l10n: L10n): string {
  const key = code ? errorMessageKey(code) : "errorGeneric";
  if (key === "errorGeneric") {
    return provider === "deepseek" ? l10n.text("deepseekFetchError") : l10n.text("codexFetchError");
  }
  return l10n.text(key);
}

export function mountApp(opts: MountOptions): MountHandle {
  const { store, root, uiLanguage } = opts;

  let settingsOpen = false;
  let confirmingDelete = false;
  let keyInputValue = "";

  const unsubscribe = store.subscribe(() => render());

  function render(): void {
    const lang = uiLanguage();
    const l10n = new L10n(lang);
    const provider = store.provider;
    const dsView = deepseekView(store.snapshot, store.now, store.fetchFailed);
    const cxView = codexView(store.snapshot, store.now, store.fetchFailed);
    const mood = provider === "deepseek" ? dsView.mood : cxView.mood;
    const hasFresh = provider === "deepseek" ? dsView.balance != null : cxView.remainingPercent != null;

    root.textContent = "";

    const panel = h("div", { class: "panel", "data-provider": provider, "data-mood": mood });

    // Demo banner (permanent while demo is active).
    if (store.demo) {
      panel.append(h("div", { class: "banner", role: "status", "data-testid": "demo-banner" }, t(l10n.text("demoBanner"))));
    }

    // Header.
    const header = h("header", { class: "header" });
    header.append(h("span", { class: "title" }, t("Kvoteven")));
    header.append(
      h("span", { class: "subtitle" }, t(provider === "deepseek" ? l10n.text("apiBalance") : l10n.text("subscription"))),
    );
    panel.append(header);

    // Provider selection.
    const providerGroup = h("fieldset", { class: "fieldset" });
    providerGroup.append(h("legend", { class: "visually-hidden" }, t(l10n.text("providerLabel"))));
    providerGroup.append(
      segmented(
        PROVIDERS.map((p) => ({
          value: p.value,
          label: p.name,
          ariaLabel: l10n.text("showProviderPrefix") + p.name,
        })),
        provider,
        "provider-segmented",
        (v) => store.selectProvider(v as Provider),
      ),
    );
    panel.append(providerGroup);

    // Language row.
    const langRow = h("div", { class: "langrow" });
    langRow.append(h("span", { class: "langlabel", "data-testid": "language-label" }, t(l10n.text("languageLabel"))));
    langRow.append(
      segmented(
        LANGUAGES.map((l) => ({ value: l.value, label: l.name })),
        store.language,
        "language-segmented",
        (v) => store.selectLanguage(v as AppLanguage),
      ),
    );
    panel.append(langRow);

    // Pet scene.
    const scene = h("div", { class: "pet-scene" });
    const pet = h("div", {
      class: "pet" + (mood === "happy" ? " pet--float" : ""),
      role: "img",
      "aria-label": l10n.text("petAccessibilityPrefix") + l10n.moodMessage(mood),
      "data-testid": "pet",
    });
    // Safe: petSvg() builds the SVG from fixed mood data and hardcoded colors
    // only — no provider, credential or user input ever reaches this string.
    pet.innerHTML = petSvg(mood);
    scene.append(pet);
    scene.append(h("div", { class: "pet-shadow" }));
    panel.append(scene);

    // Details.
    panel.append(provider === "deepseek" ? deepseekDetails(l10n, dsView, lang) : codexDetails(l10n, cxView, store.now, lang));

    // Error / stale message.
    const message = h("div", { class: "message", role: "status", "data-testid": "error-message" });
    if (store.fetchFailed) {
      message.classList.add("message--error");
      message.append(t(fetchErrorMessage(provider, store.fetchErrorCode, l10n)));
    } else if (store.snapshot && !hasFresh) {
      message.classList.add("message--muted");
      message.append(t(l10n.text("noFreshData")));
    }
    panel.append(message);

    // Footer.
    panel.append(buildFooter(l10n, lang));

    // Settings.
    const settings = buildSettings(l10n, lang);
    if (!settingsOpen) settings.setAttribute("hidden", "");
    panel.append(settings);

    root.append(panel);
  }

  function buildFooter(l10n: L10n, lang: UILanguage): HTMLElement {
    const footer = h("footer", { class: "footer" });
    const notes = h("div", { class: "footnote" });
    notes.append(
      h("span", { class: "cadence" }, t(store.busy ? l10n.text("fetchingFromPrefix") + (store.provider === "deepseek" ? "DeepSeek" : "Codex") + "…" : l10n.text("refreshCadence"))),
    );
    if (store.snapshot && store.snapshot.fetched_at) {
      notes.append(
        h("span", { class: "last-fetched", "data-testid": "last-fetched" }, t(l10n.text("lastFetchedPrefix") + formatTime(store.snapshot.fetched_at, lang))),
      );
    }
    footer.append(notes);

    const actions = h("div", { class: "actions" });
    const refresh = h("button", {
      type: "button",
      class: "icon-btn",
      "data-testid": "refresh",
      "aria-label": l10n.text("refreshAccessibility"),
      title: l10n.text("refreshHelp"),
    });
    refresh.append(t("↻"));
    refresh.disabled = store.busy;
    refresh.addEventListener("click", () => void store.refresh());
    actions.append(refresh);

    const settingsBtn = h("button", {
      type: "button",
      class: "icon-btn",
      "data-testid": "settings-toggle",
      "aria-label": settingsOpen ? l10n.text("settingsClose") : l10n.text("settingsOpen"),
      "aria-expanded": settingsOpen ? "true" : "false",
    });
    settingsBtn.append(t("⚙"));
    settingsBtn.addEventListener("click", () => {
      settingsOpen = !settingsOpen;
      confirmingDelete = false;
      if (!settingsOpen) keyInputValue = ""; // clear the key input on cancel/close
      render();
    });
    actions.append(settingsBtn);

    const quit = h("button", {
      type: "button",
      class: "icon-btn",
      "data-testid": "quit",
      "aria-label": l10n.text("quitAccessibility"),
      title: l10n.text("quitHelp"),
    });
    quit.append(t("⏻"));
    quit.addEventListener("click", () => store.quit());
    actions.append(quit);

    footer.append(actions);
    return footer;
  }

  function buildSettings(l10n: L10n, lang: UILanguage): HTMLElement {
    const settings = h("section", { class: "settings", "data-testid": "settings-panel", "aria-label": l10n.text("settingsTitle") });
    settings.append(h("h2", { class: "settings-title" }, t(l10n.text("settingsTitle"))));

    // Key management is meaningless in demo mode and would invite pasting a
    // real key into the public browser preview, so it is omitted there.
    if (!store.demo) {
      settings.append(buildKeyGroup(l10n, lang));
    }

    // Codex source.
    const sourceGroup = h("div", { class: "settings-group" });
    sourceGroup.append(h("h3", { class: "group-title" }, t(l10n.text("codexSourceLabel"))));
    sourceGroup.append(
      segmented(
        SOURCES.map((s) => ({ value: s.value, label: l10n.text(s.key) })),
        store.codexSource,
        "codex-source-segmented",
        (v) => store.selectCodexSource(v as CodexSource),
      ),
    );
    sourceGroup.append(h("p", { class: "hint" }, t(l10n.text("codexOnboarding"))));
    settings.append(sourceGroup);

    // Demo toggle.
    const demoGroup = h("div", { class: "settings-group" });
    demoGroup.append(h("h3", { class: "group-title" }, t(l10n.text("demoLabel"))));
    const toggle = h("label", { class: "switch" });
    const checkbox = h("input", { type: "checkbox", "data-testid": "demo-toggle" }) as HTMLInputElement;
    checkbox.checked = store.demo;
    checkbox.disabled = store.demoBusy || store.settingsBusy;
    checkbox.addEventListener("change", () => void store.setDemo(checkbox.checked));
    toggle.append(checkbox, h("span", { class: "switch-track" }), t(store.demo ? l10n.text("demoUseSample") : l10n.text("demoUseReal")));
    demoGroup.append(toggle);
    demoGroup.append(h("p", { class: "hint" }, t(l10n.text("demoHint"))));
    settings.append(demoGroup);

    // Settings feedback (shared by key save/delete and demo toggle errors).
    const msg = h("div", { class: "settings-msg", role: "status", "data-testid": "settings-message" });
    if (store.settingsError) {
      msg.classList.add("message--error");
      msg.append(t(l10n.text(errorMessageKey(store.settingsError))));
    } else if (store.settingsMessage) {
      msg.classList.add("message--ok");
      msg.append(t(l10n.text(store.settingsMessage)));
    }
    settings.append(msg);

    return settings;
  }

  function buildKeyGroup(l10n: L10n, lang: UILanguage): HTMLElement {
    const keyGroup = h("div", { class: "settings-group" });
    keyGroup.append(h("h3", { class: "group-title" }, t(l10n.text("deepseekKeyLabel"))));
    keyGroup.append(
      h(
        "p",
        { class: "hint" },
        t(store.status?.deepseek_configured ? l10n.text("deepseekKeySavedNote") : l10n.text("deepseekKeyMissing")),
      ),
    );
    keyGroup.append(h("p", { class: "hint" }, t(l10n.text("deepseekOnboarding"))));

    const form = h("form", { class: "key-form", "data-testid": "key-form" });
    const input = h("input", {
      type: "password",
      "data-testid": "key-input",
      autocomplete: "off",
      autocapitalize: "off",
      spellcheck: "false",
      "aria-label": l10n.text("deepseekKeyLabel"),
    });
    input.placeholder = l10n.text("deepseekKeyPlaceholder");
    input.value = keyInputValue;
    input.addEventListener("input", () => {
      keyInputValue = input.value;
    });
    const save = h("button", { type: "submit", class: "primary", "data-testid": "key-save" }, t(l10n.text("deepseekKeySave")));
    save.disabled = store.settingsBusy;
    const cancel = h("button", { type: "button", class: "ghost", "data-testid": "key-cancel" }, t(l10n.text("cancel")));
    cancel.disabled = store.settingsBusy;
    cancel.addEventListener("click", () => {
      keyInputValue = "";
      render();
    });
    form.append(input, save, cancel);
    form.addEventListener("submit", (e) => {
      e.preventDefault();
      const value = keyInputValue;
      keyInputValue = "";
      void store.saveKey(value);
    });
    keyGroup.append(form);

    if (store.status?.deepseek_configured) {
      const del = h("button", { type: "button", class: "danger", "data-testid": "key-delete" });
      del.disabled = store.settingsBusy;
      del.append(t(confirmingDelete ? l10n.text("confirm") : l10n.text("deepseekKeyDelete")));
      del.addEventListener("click", () => {
        if (confirmingDelete) {
          confirmingDelete = false;
          void store.deleteKey();
        } else {
          confirmingDelete = true;
        }
        render();
      });
      if (confirmingDelete) {
        const confirmText = h("span", { class: "confirm-text" }, t(l10n.text("deepseekKeyDeleteConfirm")));
        keyGroup.append(confirmText, del);
      } else {
        keyGroup.append(del);
      }
    }

    return keyGroup;
  }

  function deepseekDetails(l10n: L10n, view: ReturnType<typeof deepseekView>, lang: UILanguage): HTMLElement {
    const details = h("div", { class: "details" });
    details.append(h("div", { class: "mood", "data-testid": "mood-message" }, t(l10n.deepSeekMoodMessage(view.mood))));

    const big = h("div", { class: "big-number" });
    big.append(h("span", { class: "amount", "data-testid": "amount" }, t(formatMoneyAmount(view.balance, lang))));
    if (view.currency) {
      big.append(h("span", { class: "currency", "data-testid": "currency" }, t(view.currency)));
    }
    details.append(big);

    if (view.balance != null) {
      details.append(h("div", { class: "caption" }, t(l10n.text("balanceCaption"))));
      const available = store.snapshot?.is_available;
      if (available != null) {
        const avail = h("div", { class: "availability" });
        avail.append(h("span", { class: "dot" }), t(available ? l10n.text("apiAvailable") : l10n.text("apiUnavailable")));
        details.append(avail);
      }
      if (store.snapshot?.net_change != null) {
        const change = h("div", { class: "net-change", "data-testid": "net-change" });
        change.append(t(l10n.text("balanceChangePrefix") + ": " + formatNetChange(store.snapshot.net_change, lang)));
        if (view.currency) change.append(t(" " + view.currency));
        details.append(change);
        details.append(h("div", { class: "net-note" }, t(l10n.text("netChangeNote"))));
      }
      if (view.currency) {
        details.append(h("div", { class: "threshold" }, t(l10n.sleepyThreshold(view.currency))));
      }
    }
    return details;
  }

  function codexDetails(l10n: L10n, view: ReturnType<typeof codexView>, now: Date, lang: UILanguage): HTMLElement {
    const details = h("div", { class: "details" });
    details.append(h("div", { class: "mood", "data-testid": "mood-message" }, t(l10n.moodMessage(view.mood))));

    const big = h("div", { class: "big-number" });
    big.append(h("span", { class: "amount", "data-testid": "percent" }, t(formatPercent(view.remainingPercent))));
    big.append(h("span", { class: "unit" }, t(l10n.text("ofWeeklyQuotaLeft"))));
    details.append(big);

    const bar = h("div", { class: "progress", role: "progressbar", "aria-valuemin": "0", "aria-valuemax": "100" });
    if (view.remainingPercent != null) {
      bar.setAttribute("aria-valuenow", String(Math.round(view.remainingPercent)));
      bar.append(h("div", { class: "progress-fill" }));
      (bar.firstElementChild as HTMLElement).style.width = `${Math.min(100, Math.max(0, view.remainingPercent))}%`;
    } else {
      bar.setAttribute("aria-hidden", "true");
    }
    details.append(bar);

    if (store.snapshot?.resets_at) {
      const cd = countdown(store.snapshot.resets_at, now);
      if (cd.passed) {
        details.append(h("div", { class: "countdown", "data-testid": "countdown" }, t(l10n.text("resetPassed"))));
      } else {
        const label = cd.days > 0 ? l10n.resetInDays(cd.days, cd.hours) : l10n.resetInHours(cd.hours, cd.minutes);
        details.append(h("div", { class: "countdown", "data-testid": "countdown" }, t(label)));
      }
      details.append(h("div", { class: "reset-date" }, t(formatResetDate(store.snapshot.resets_at, lang))));
    }
    return details;
  }

  render();
  return { dispose: unsubscribe, setKeyInputValue: (v) => { keyInputValue = v; } };
}
