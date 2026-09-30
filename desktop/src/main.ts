import "./style.css";
import { detectBridge } from "./bridge";
import { Store, type StorageLike } from "./store";
import { mountApp } from "./render";
import { L10n, resolveLanguage } from "./i18n";
import { runSmokeTest } from "./smoke";
import type { UILanguage } from "./types";

function systemLanguage(): string {
  return navigator.languages?.[0] ?? navigator.language ?? "";
}

function safeStorage(): StorageLike | null {
  try {
    if (typeof localStorage !== "undefined") return localStorage;
  } catch {
    /* storage disabled — run without persistence */
  }
  return null;
}

async function boot(): Promise<void> {
  const root = document.getElementById("app");
  if (!root) return;

  const { bridge, mode } = detectBridge();

  if (mode === "blocked") {
    const lang: UILanguage = systemLanguage().startsWith("da") ? "da" : "en";
    const l10n = new L10n(lang);
    const p = document.createElement("p");
    p.className = "preview-blocked";
    p.textContent = l10n.text("previewRequiresDemo");
    root.append(p);
    return;
  }

  const store = new Store({ bridge, storage: safeStorage() });
  const uiLanguage = (): UILanguage => resolveLanguage(store.language, systemLanguage());
  mountApp({ store, root, uiLanguage });

  await store.init();

  // Native smoke path, active only under CLI `--demo --smoke-test`.
  if (mode === "native") {
    const smokeEnabled = await bridge.smokeContext().catch(() => false);
    if (smokeEnabled) {
      const proof = await runSmokeTest(store, bridge);
      if (proof) {
        // Report proof only on success; on failure the parent's timeout fails the test.
        await bridge.reportSmoke(proof);
      }
    }
  }
}

void boot();
