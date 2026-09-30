import { test, expect } from "@playwright/test";
import { injectBridge } from "./bridge";

test("switches between DeepSeek balance and Codex quota", async ({ page }) => {
  await injectBridge(page, { demo: true });
  await page.goto("/");
  await expect(page.getByTestId("amount")).toHaveText("42.50");
  await expect(page.getByTestId("currency")).toHaveText("USD");
  await page.locator('[data-testid="provider-segmented"] button[data-value="openai-codex"]').click();
  await expect(page.getByTestId("percent")).toHaveText("68%");
});

test("switches language immediately and reformats numbers", async ({ page }) => {
  await injectBridge(page, { demo: true });
  await page.goto("/");
  await expect(page.getByTestId("language-label")).toHaveText("Language");
  await expect(page.getByTestId("amount")).toHaveText("42.50");
  await page.locator('[data-testid="language-segmented"] button[data-value="da"]').click();
  await expect(page.getByTestId("language-label")).toHaveText("Sprog");
  await expect(page.getByTestId("amount")).toHaveText("42,50");
  await expect(page.getByTestId("demo-banner")).toHaveText("Demo · eksempeldata");
});

test("shows no-key onboarding and clears the key after save", async ({ page }) => {
  await injectBridge(page, { demo: false, deepseekConfigured: false });
  await page.goto("/");
  await page.getByTestId("settings-toggle").click();
  await expect(page.getByTestId("settings-panel")).toBeVisible();
  await expect(page.getByText("No key stored.")).toBeVisible();
  const input = page.getByTestId("key-input");
  await input.fill("sk-secret-123");
  await page.getByTestId("key-save").click();
  await expect(input).toHaveValue("");
  await expect(page.getByTestId("settings-message")).toHaveText("Key saved.");
});

test("clears the key input on cancel", async ({ page }) => {
  await injectBridge(page, { demo: false });
  await page.goto("/");
  await page.getByTestId("settings-toggle").click();
  await page.getByTestId("key-input").fill("sk-secret");
  await page.getByTestId("key-cancel").click();
  await expect(page.getByTestId("key-input")).toHaveValue("");
});

test("key input is a masked password field with no reveal", async ({ page }) => {
  await injectBridge(page, { demo: false });
  await page.goto("/");
  await page.getByTestId("settings-toggle").click();
  await expect(page.getByTestId("key-input")).toHaveAttribute("type", "password");
});

test("shows a clear error when the OS keyring is unavailable", async ({ page }) => {
  await injectBridge(page, { demo: false, keyringFail: true });
  await page.goto("/");
  await page.getByTestId("settings-toggle").click();
  await page.getByTestId("key-input").fill("sk-secret");
  await page.getByTestId("key-save").click();
  await expect(page.getByTestId("key-input")).toHaveValue("");
  await expect(page.getByTestId("settings-message")).toContainText("keyring is unavailable");
});

test("saves a key and offers deletion with confirmation", async ({ page }) => {
  await injectBridge(page, { demo: false, deepseekConfigured: false });
  await page.goto("/");
  await page.getByTestId("settings-toggle").click();
  await page.getByTestId("key-input").fill("sk-secret");
  await page.getByTestId("key-save").click();
  await expect(page.getByTestId("settings-message")).toHaveText("Key saved.");
  const del = page.getByTestId("key-delete");
  await expect(del).toBeVisible();
  await expect(del).toHaveText("Delete key");
  await del.click();
  await expect(del).toHaveText("Delete");
  await expect(page.getByText("Delete the stored DeepSeek key?")).toBeVisible();
  await del.click();
  await expect(page.getByTestId("settings-message")).toHaveText("Key deleted.");
});

test("masks the number and shows an error when a read fails", async ({ page }) => {
  await injectBridge(page, { demo: false, fail: { provider: "deepseek", code: "network" } });
  await page.goto("/");
  await expect(page.getByTestId("amount")).toHaveText("—");
  await expect(page.getByTestId("error-message")).toContainText("Could not fetch the balance");
  await expect(page.locator(".panel")).toHaveAttribute("data-mood", "unknown");
});

test("masks a stale reading as unknown", async ({ page }) => {
  const stale = new Date(Date.now() - 11 * 60_000).toISOString();
  await injectBridge(page, { demo: false, deepseek: { fetched_at: stale } });
  await page.goto("/");
  await expect(page.getByTestId("amount")).toHaveText("—");
  await expect(page.getByTestId("error-message")).toContainText("No fresh data");
});

test("masks a codex quota whose reset has passed", async ({ page }) => {
  const past = new Date(Date.now() - 60_000).toISOString();
  await injectBridge(page, { demo: false, codex: { resets_at: past } });
  await page.goto("/");
  await page.locator('[data-testid="provider-segmented"] button[data-value="openai-codex"]').click();
  await expect(page.getByTestId("percent")).toHaveText("—");
  await expect(page.getByTestId("countdown")).toContainText("Reset time passed");
});

test("provider and language controls are keyboard operable", async ({ page }) => {
  await injectBridge(page, { demo: true });
  await page.goto("/");
  const codexBtn = page.locator('[data-testid="provider-segmented"] button[data-value="openai-codex"]');
  await codexBtn.focus();
  await codexBtn.press("Enter");
  await expect(page.getByTestId("percent")).toHaveText("68%");
  const daBtn = page.locator('[data-testid="language-segmented"] button[data-value="da"]');
  await daBtn.focus();
  await daBtn.press("Enter");
  await expect(page.getByTestId("language-label")).toHaveText("Sprog");
});

test("provider buttons expose radio semantics and arrow keys", async ({ page }) => {
  await injectBridge(page, { demo: true });
  await page.goto("/");
  const ds = page.locator('[data-testid="provider-segmented"] button[data-value="deepseek"]');
  await expect(ds).toHaveAttribute("role", "radio");
  await expect(ds).toHaveAttribute("aria-checked", "true");
  const cx = page.locator('[data-testid="provider-segmented"] button[data-value="openai-codex"]');
  await expect(cx).toHaveAttribute("aria-checked", "false");
  // ArrowRight moves selection to Codex.
  await ds.focus();
  await ds.press("ArrowRight");
  await expect(page.getByTestId("percent")).toHaveText("68%");
  await expect(page.locator('[data-testid="provider-segmented"] button[data-value="openai-codex"]')).toHaveAttribute("aria-checked", "true");
});

test("demo clock follows the snapshot's fetched_at, not a fixed anchor", async ({ page }) => {
  // A demo snapshot one hour after DEMO_ANCHOR: a fixed anchor clock would
  // read it as far-future and mask it. The store must pin its clock to the
  // snapshot's own fetched_at so it renders fresh.
  await injectBridge(page, { demo: true, deepseek: { fetched_at: "2026-09-29T20:05:02Z" } });
  await page.goto("/");
  await expect(page.getByTestId("amount")).toHaveText("42.50");
});

test("key management is never rendered in demo mode", async ({ page }) => {
  await injectBridge(page, { demo: true });
  await page.goto("/");
  await page.getByTestId("settings-toggle").click();
  await expect(page.getByTestId("settings-panel")).toBeVisible();
  await expect(page.getByTestId("key-form")).toHaveCount(0);
  await expect(page.getByTestId("key-input")).toHaveCount(0);
  await expect(page.getByTestId("key-save")).toHaveCount(0);
});
