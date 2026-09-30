import { test, expect } from "@playwright/test";

test("demo preview is labelled and makes no external requests", async ({ page }) => {
  const external: string[] = [];
  page.on("request", (req) => {
    const host = new URL(req.url()).hostname;
    if (host !== "127.0.0.1" && host !== "localhost") external.push(req.url());
  });
  await page.goto("/?demo=1");
  await expect(page.getByTestId("demo-banner")).toHaveText("Demo · sample data");
  await expect(page.getByTestId("amount")).toHaveText("42.50");
  await expect(page.getByTestId("currency")).toHaveText("USD");
  await page.locator('[data-testid="provider-segmented"] button[data-value="openai-codex"]').click();
  await expect(page.getByTestId("percent")).toHaveText("68%");
  expect(external).toEqual([]);
});

test("browser preview requires the demo flag", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator(".preview-blocked")).toBeVisible();
  // No data is ever rendered in the blocked preview.
  await expect(page.getByTestId("amount")).toHaveCount(0);
});

test("demo data is clearly synthetic and never looks live", async ({ page }) => {
  await page.goto("/?demo=1");
  // The banner is permanent while demo is active.
  await expect(page.getByTestId("demo-banner")).toBeVisible();
  await expect(page.getByTestId("demo-banner")).toContainText("Demo");
  // No live-only chrome (e.g. a plausible "Last fetched 12:00" with real times).
  await expect(page.getByTestId("last-fetched")).toContainText("Last fetched");
});

test("settings are closed by default and open/close on the gear", async ({ page }) => {
  await page.goto("/?demo=1");
  const panel = page.getByTestId("settings-panel");
  await expect(panel).toBeHidden();
  await page.getByTestId("settings-toggle").click();
  await expect(panel).toBeVisible();
  await page.getByTestId("settings-toggle").click();
  await expect(panel).toBeHidden();
});

test("the public demo offers no key management", async ({ page }) => {
  await page.goto("/?demo=1");
  await page.getByTestId("settings-toggle").click();
  await expect(page.getByTestId("settings-panel")).toBeVisible();
  await expect(page.getByTestId("key-input")).toHaveCount(0);
  await expect(page.getByTestId("key-save")).toHaveCount(0);
  await expect(page.getByTestId("key-form")).toHaveCount(0);
});
