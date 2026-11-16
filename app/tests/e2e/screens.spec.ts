import { join } from "node:path";
import { expect, type Page, type TestInfo, test } from "@playwright/test";
import en from "../../../locales/en/ui.json" with { type: "json" };
import hu from "../../../locales/hu/ui.json" with { type: "json" };

const LANGUAGES = [
  { lang: "en", locale: "en-US", ui: en },
  { lang: "hu", locale: "hu-HU", ui: hu },
] as const;

const SCREENSHOTS = join(import.meta.dirname, "..", "screenshots");
const DISPLAYLINK = "DisplayLink Manager";

function fill(message: string, value: string, key = "app"): string {
  return message.replace(`{{${key}}}`, value);
}

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
      await expect(page.getByRole("button", { name: ui.activity.restore })).toHaveCount(0);
      const restoreRow = page.getByRole("article").first();
      await expect(
        restoreRow.getByRole("heading", {
          name: new RegExp(`^${fill(ui.activity.restore_of, "", "trigger")}`),
        }),
      ).toBeVisible();
      await expect(restoreRow.getByText(ui.restore_past.hide).first()).toBeVisible();
      expect(errors).toEqual([]);
    });

    test("status when config.toml was rejected", async ({ page }) => {
      const errors = await open(page, "/?state=config_problem");
      const banner = page.getByRole("alert").filter({ hasText: ui.config.problem.title });
      await expect(banner).toBeVisible();
      await expect(banner.getByText(ui.config.problem.unparsable)).toBeVisible();
      await page.goto("/?state=config_problem#rules");
      await expect(page.getByText(ui.config.problem.kept)).toBeVisible();
      expect(errors).toEqual([]);
    });

    test("settings", async ({ page }, info) => {
      const errors = await open(page, "/#settings");
      await expect(page.getByRole("heading", { level: 1, name: ui.settings.title })).toBeVisible();
      await expect(page.getByRole("heading", { name: ui.settings.capabilities })).toBeVisible();
      await shot(page, info, "settings", lang);
      expect(errors).toEqual([]);
    });

    test("settings: ignored capture and displays", async ({ page }, info) => {
      const errors = await open(page, "/#settings");
      const section = page.getByRole("region", { name: ui.settings.ignore.title });
      await expect(section).toBeVisible();
      const apps = section.getByRole("list").first();
      await expect(apps.getByText(DISPLAYLINK, { exact: true })).toBeVisible();
      await expect(apps.getByText(new RegExp(`^${ui.settings.ignore.running} ·`))).toBeVisible();

      const picker = section.getByLabel(ui.settings.ignore.displays_label);
      await picker.selectOption({ label: "LG HDR WFHD (1)" });
      await picker.locator("xpath=..").getByRole("button", { name: ui.settings.list_add }).click();
      const remove = section.getByRole("button", {
        name: fill(ui.settings.list_remove, "LG HDR WFHD (1)", "item"),
      });
      await expect(remove).toBeVisible();
      await expect(
        section.getByText(
          fill(ui.settings.ignore.display_model, "GSM", "vendor").replace("{{model}}", "23482"),
        ),
      ).toBeVisible();
      await expect(section.getByText(new RegExp(`^${ui.settings.ignore.connected} ·`))).toBeVisible();

      const treat = section.getByRole("switch", { name: ui.settings.ignore.treat_connected });
      await expect(treat).not.toBeChecked();
      await treat.click();
      await expect(treat).toBeChecked();
      await section.scrollIntoViewIfNeeded();
      await shot(page, info, "settings-ignore", lang);

      await remove.click();
      await expect(remove).toHaveCount(0);
      expect(errors).toEqual([]);
    });

    test("status while a dock holds the share watcher", async ({ page }, info) => {
      const errors = await open(page, "/?state=docked");
      await expect(page.getByRole("heading", { level: 1, name: ui.home.title.idle })).toBeVisible();
      await expect(page.getByText(fill(ui.home.ignored_capture, DISPLAYLINK))).toBeVisible();
      await expect(page.getByText(ui.reason.capture_ignored)).toBeVisible();
      await shot(page, info, "status-docked", lang);
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
      await expect(
        page.getByRole("heading", { level: 1, name: fill(ui.onboarding.capture.title, DISPLAYLINK) }),
      ).toBeVisible();
      await expect(page.getByRole("switch", { name: ui.onboarding.capture.toggle })).toBeChecked();
      await shot(page, info, "onboarding-capture", lang);
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
