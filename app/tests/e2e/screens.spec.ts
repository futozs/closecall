import { join } from "node:path";
import { expect, type Page, type TestInfo, test } from "@playwright/test";
import en from "../../../locales/en/ui.json" with { type: "json" };
import hu from "../../../locales/hu/ui.json" with { type: "json" };

const LANGUAGES = [
  { lang: "en", locale: "en-US", ui: en },
  { lang: "hu", locale: "hu-HU", ui: hu },
] as const;

const SCREENSHOTS = join(import.meta.dirname, "..", "screenshots");

async function open(page: Page, url: string) {
  const errors: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(url);
  return errors;
}

async function shot(page: Page, info: TestInfo, name: string, lang: string) {
  // A sticky sidebar follows the scroll position, so capture from the top.
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.evaluate(() => document.fonts.ready);
  // The production CSP forbids inline style attributes; a stray one would break the real app.
  expect(await page.locator("[style]").count()).toBe(0);
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow, "page scrolls sideways").toBeLessThanOrEqual(0);
  await page.screenshot({
    path: join(SCREENSHOTS, `${name}-${lang}-${info.project.name}.png`),
    fullPage: true,
  });
}

for (const { lang, locale, ui } of LANGUAGES) {
  test.describe(`${lang}`, () => {
    test.use({ locale });

    test("status", async ({ page }, info) => {
      const errors = await open(page, "/");
      await expect(page.getByRole("heading", { level: 1, name: ui.home.title.idle })).toBeVisible();
      await expect(page.getByRole("button", { name: ui.home.clean_now })).toBeVisible();
      await expect(page.getByText(ui.home.check.access, { exact: true })).toBeVisible();
      await shot(page, info, "status", lang);
      expect(errors).toEqual([]);
    });

    test("status while sharing", async ({ page }, info) => {
      const errors = await open(page, "/?state=sharing");
      await expect(page.getByRole("heading", { level: 1, name: ui.home.title.sharing })).toBeVisible();
      await shot(page, info, "status-sharing", lang);
      expect(errors).toEqual([]);
    });

    test.describe("dark", () => {
      test.use({ colorScheme: "dark" });
      test("status in dark mode", async ({ page }, info) => {
        const errors = await open(page, "/");
        await expect(page.getByRole("heading", { level: 1, name: ui.home.title.idle })).toBeVisible();
        await shot(page, info, "status-dark", lang);
        expect(errors).toEqual([]);
      });
    });

    test("simulate a screen share: preview, then act", async ({ page }, info) => {
      const errors = await open(page, "/");
      await page.getByRole("button", { name: ui.home.simulate }).click();
      await expect(page.getByRole("heading", { name: ui.home.sim.title })).toBeVisible();
      await shot(page, info, "simulate-preview", lang);
      await page.getByRole("button", { name: ui.home.sim.run }).click();
      await expect(page.getByRole("heading", { name: ui.home.sim.done })).toBeVisible();
      await shot(page, info, "simulate-result", lang);
      await page.getByRole("link", { name: ui.home.see_activity }).first().click();
      await expect(page.getByRole("heading", { level: 1, name: ui.activity.title })).toBeVisible();
      expect(errors).toEqual([]);
    });

    test("rules: list, editor and inline validation", async ({ page }, info) => {
      const errors = await open(page, "/#rules");
      await expect(page.getByRole("heading", { level: 1, name: ui.rules.title })).toBeVisible();
      await shot(page, info, "rules", lang);

      await page.getByRole("link", { name: new RegExp(ui.preset["share-close-sensitive-tabs"]) }).click();
      await expect(
        page.getByRole("heading", { level: 1, name: ui.preset["share-close-sensitive-tabs"] }),
      ).toBeVisible();
      await shot(page, info, "rule-editor", lang);

      await page.goto("/#rules/new");
      await expect(page.getByRole("heading", { level: 1, name: ui.editor.title_new })).toBeVisible();
      await page.getByRole("checkbox", { name: ui.when.panic, exact: true }).uncheck();
      await page.getByRole("checkbox", { name: new RegExp(`^${ui.when.screen_share}`) }).uncheck();
      await page.getByRole("button", { name: ui.editor.save }).click();
      await expect(page.getByText(ui.config.error.empty_when)).toBeVisible();
      await shot(page, info, "rule-validation", lang);
      expect(errors).toEqual([]);
    });

    test("categories", async ({ page }, info) => {
      const errors = await open(page, "/#categories");
      await expect(page.getByRole("heading", { level: 1, name: ui.categories.title })).toBeVisible();
      await page.locator("summary").first().click();
      await expect(page.getByRole("button", { name: "HaGeZi NSFW" })).toBeVisible();
      await expect(page.getByText(ui.categories.opt_in, { exact: true })).toHaveCount(3);
      await shot(page, info, "categories", lang);
      await page.getByRole("link", { name: ui.categories.new }).click();
      await expect(
        page.getByRole("heading", { level: 1, name: ui.categories.editor.title_new }),
      ).toBeVisible();
      await shot(page, info, "category-editor", lang);
      expect(errors).toEqual([]);
    });

    test("activity", async ({ page }, info) => {
      const errors = await open(page, "/#activity");
      await expect(page.getByRole("heading", { level: 1, name: ui.activity.title })).toBeVisible();
      await expect(page.getByText(ui.reason.firefox_no_tab_close)).toBeVisible();
      await shot(page, info, "activity", lang);
      await page.getByRole("button", { name: ui.activity.restore }).click();
      await expect(page.getByText(ui.activity.restored, { exact: true })).toBeVisible();
      expect(errors).toEqual([]);
    });

    test("settings", async ({ page }, info) => {
      const errors = await open(page, "/#settings");
      await expect(page.getByRole("heading", { level: 1, name: ui.settings.title })).toBeVisible();
      await expect(page.getByRole("heading", { name: ui.settings.capabilities })).toBeVisible();
      await shot(page, info, "settings", lang);
      expect(errors).toEqual([]);
    });

    test("onboarding", async ({ page }, info) => {
      const errors = await open(page, "/?onboarding");
      await expect(page.getByRole("heading", { level: 1, name: ui.onboarding.welcome.title })).toBeVisible();
      await expect(page.getByRole("radio", { name: lang === "hu" ? "Magyar" : "English" })).toBeChecked();
      await shot(page, info, "onboarding-welcome", lang);
      const next = page.getByRole("button", { name: ui.onboarding.next });

      await next.click();
      await expect(
        page.getByRole("heading", { level: 1, name: ui.onboarding.accessibility.title }),
      ).toBeVisible();
      await next.click();
      await expect(
        page.getByRole("heading", { level: 1, name: ui.onboarding.automation.title }),
      ).toBeVisible();
      await shot(page, info, "onboarding-automation", lang);
      await next.click();
      await next.click();
      await expect(page.getByRole("heading", { level: 1, name: ui.onboarding.os.title })).toBeVisible();
      await shot(page, info, "onboarding-os", lang);
      await next.click();
      await expect(page.getByRole("heading", { level: 1, name: ui.onboarding.dry_run.title })).toBeVisible();
      await expect(page.getByText(ui.preset["share-close-sensitive-tabs"]).first()).toBeVisible();
      await shot(page, info, "onboarding-dry-run", lang);
      await next.click();
      await page.getByRole("button", { name: ui.onboarding.finish }).click();
      await expect(page.getByRole("heading", { level: 1, name: ui.home.title.idle })).toBeVisible();
      expect(errors).toEqual([]);
    });
  });
}
