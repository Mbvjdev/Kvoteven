import { test } from "@playwright/test";

/**
 * Renders the synthetic demo (never a real account) for both providers, both
 * languages and both color schemes, and saves screenshots into the ignored
 * `verification/` directory (outside public assets).
 */
const combos: { provider: "deepseek" | "openai-codex"; lang: "en" | "da"; scheme: "light" | "dark" }[] = [
  { provider: "deepseek", lang: "en", scheme: "light" },
  { provider: "deepseek", lang: "en", scheme: "dark" },
  { provider: "deepseek", lang: "da", scheme: "light" },
  { provider: "deepseek", lang: "da", scheme: "dark" },
  { provider: "openai-codex", lang: "en", scheme: "light" },
  { provider: "openai-codex", lang: "en", scheme: "dark" },
  { provider: "openai-codex", lang: "da", scheme: "light" },
  { provider: "openai-codex", lang: "da", scheme: "dark" },
];

for (const c of combos) {
  test(`demo ${c.provider} ${c.lang} ${c.scheme}`, async ({ page }) => {
    await page.setViewportSize({ width: 420, height: 730 });
    await page.emulateMedia({ colorScheme: c.scheme });
    await page.goto("/?demo=1");
    if (c.lang === "da") {
      await page.locator('[data-testid="language-segmented"] button[data-value="da"]').click();
    }
    if (c.provider === "openai-codex") {
      await page.locator('[data-testid="provider-segmented"] button[data-value="openai-codex"]').click();
    }
    await page.screenshot({
      path: `verification/demo-${c.provider}-${c.lang}-${c.scheme}.png`,
    });
  });
}
