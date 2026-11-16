import { beforeAll, describe, expect, test } from "vitest";
import type { Evidence } from "../../src/bindings/Evidence";
import { describeEvidence, failureMessage, issueMessage, keyedMessage, ruleName } from "../../src/evidence";
import { i18next, initI18n } from "../../src/i18n";

const banking: Evidence = { kind: "category", id: "banking", via: { kind: "domain", domain: "otpbank.hu" } };

beforeAll(async () => {
  await initI18n("en");
});

describe("describeEvidence", () => {
  test("names the category and the matcher that hit, in English", async () => {
    await i18next.changeLanguage("en");
    expect(describeEvidence(banking, i18next.t)).toBe("Banking, via domain otpbank.hu");
    expect(
      describeEvidence(
        { kind: "category", id: "invoices", via: { kind: "title_contains", keyword: "számla" } },
        i18next.t,
      ),
    ).toBe("Invoices and payslips, via title contains “számla”");
  });

  test("reads naturally in Hungarian", async () => {
    await i18next.changeLanguage("hu");
    expect(describeEvidence(banking, i18next.t)).toBe("Bank (domain: otpbank.hu)");
    expect(describeEvidence({ kind: "private" }, i18next.t)).toBe("privát ablak");
  });

  test("falls back to the raw id for an unknown category", async () => {
    await i18next.changeLanguage("en");
    expect(
      describeEvidence(
        { kind: "category", id: "payroll", via: { kind: "app", app: "Kulcs-Bér" } },
        i18next.t,
      ),
    ).toBe("payroll, via app Kulcs-Bér");
  });
});

describe("engine message keys", () => {
  test("validation codes resolve with or without the namespace", async () => {
    await i18next.changeLanguage("en");
    const bare = issueMessage(i18next.t, "invalid_domain", "match.any[0]");
    expect(bare).toBe(issueMessage(i18next.t, "config.error.invalid_domain", "match.any[0]"));
    expect(bare).toContain("otpbank.hu");
  });

  test("unknown keys still render something readable", async () => {
    await i18next.changeLanguage("en");
    expect(issueMessage(i18next.t, "brand_new_code", "when")).toBe("Problem with when (brand_new_code).");
    expect(keyedMessage(i18next.t, "reason.brand_new", "reason")).toBe("Reason code: reason.brand_new");
    expect(keyedMessage(i18next.t, "unsaved_hidden", "reason")).toMatch(/unsaved changes/);
  });

  test("preset rules get their translated name, user rules keep theirs", async () => {
    await i18next.changeLanguage("hu");
    expect(ruleName(i18next.t, { id: "away-close-banking", name: null })).toBe(
      "Banki lapok bezárása 15 perc távollét után",
    );
    expect(ruleName(i18next.t, { id: "rule-7", name: "Munkahelyi bérportál" })).toBe("Munkahelyi bérportál");
    expect(
      ruleName(i18next.t, { id: "private-windows-on-share", name: "preset.private-windows-on-share" }),
    ).toBe("Privát és inkognitó lapok bezárása megosztáskor");
    expect(ruleName(i18next.t, { id: "gone", name: "preset.gone" })).toBe("gone");
  });
});

describe("failureMessage", () => {
  test("renders a failed step's message key like any other reason", async () => {
    await i18next.changeLanguage("en");
    expect(failureMessage(i18next.t, "reason.timeout")).toBe("The app didn't respond in time.");
    expect(failureMessage(i18next.t, "reason.macos.health.no_input_device")).toBe(
      "No microphone is connected.",
    );
    await i18next.changeLanguage("hu");
    expect(failureMessage(i18next.t, "reason.timeout")).toBe("Az alkalmazás nem válaszolt időben.");
  });

  test("shows anything that is not a key as it came", async () => {
    await i18next.changeLanguage("en");
    expect(failureMessage(i18next.t, "could not conceal the window")).toBe("could not conceal the window");
  });
});
