import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, test } from "vitest";
import type { DisplayInfo } from "../../src/bindings/DisplayInfo";
import * as ipc from "../../src/ipc";
import { installMock } from "../../src/mock";
import {
  appSelectorMatches,
  displaySelectorFor,
  displaySelectorMatches,
  KNOWN_CAPTURE_APPS,
  pnpVendor,
} from "../../src/model";

const lg: DisplayInfo = {
  id: 2,
  name: "LG HDR WFHD (2)",
  vendor: 7789,
  model: 23481,
  serial: 22073,
  builtin: false,
  mirrored: false,
};

describe("ignore list helpers", () => {
  test("display selectors match like crates/core", () => {
    expect(displaySelectorMatches({ name: "lg hdr" }, lg)).toBe(true);
    expect(displaySelectorMatches({ name: "  " }, lg)).toBe(false);
    expect(displaySelectorMatches({ id: { vendor: 7789, model: 23481, serial: null } }, lg)).toBe(true);
    expect(displaySelectorMatches({ id: { vendor: 7789, model: 23481, serial: 1 } }, lg)).toBe(false);
    expect(displaySelectorMatches({ id: { vendor: 7789, model: 23482, serial: null } }, lg)).toBe(false);
  });

  test("a display is added by model when it has an EDID vendor, else by name", () => {
    expect(displaySelectorFor(lg)).toEqual({ id: { vendor: 7789, model: 23481, serial: null } });
    expect(displaySelectorFor({ ...lg, vendor: 0x756e6b6e })).toEqual({ name: "LG HDR WFHD (2)" });
    expect(displaySelectorFor({ ...lg, vendor: 0, name: "" })).toBeNull();
  });

  test("EDID vendor numbers read as their three letters", () => {
    expect(pnpVendor(7789)).toBe("GSM");
    expect(pnpVendor(1552)).toBe("APP");
    expect(pnpVendor(0x756e6b6e)).toBe("1970170734");
  });

  test("app selectors match a running agent by bundle id or folded name", () => {
    const agent = { id: "com.displaylink.DisplayLinkUserAgent", name: "DisplayLink Manager", pid: 612 };
    expect(appSelectorMatches({ bundle_id: "com.displaylink.DisplayLinkUserAgent" }, agent)).toBe(true);
    expect(appSelectorMatches({ name: "displaylink manager" }, agent)).toBe(true);
    expect(appSelectorMatches({ exe: "displaylinkuseragent" }, agent)).toBe(false);
    expect(KNOWN_CAPTURE_APPS).toContain(agent.id);
  });
});

describe("browser mock", () => {
  beforeEach(() => installMock());
  afterEach(() => clearMocks());

  test("lists running agents and displays, and validates the ignore list", async () => {
    expect((await ipc.listRunningApps()).map((app) => app.id)).toContain(
      "com.displaylink.DisplayLinkUserAgent",
    );
    expect((await ipc.listDisplays()).filter((display) => display.builtin)).toHaveLength(1);

    const config = await ipc.getConfig();
    expect(config.ignore.capture_apps).toEqual([{ bundle_id: "com.displaylink.DisplayLinkUserAgent" }]);
    const issues = await ipc.saveConfig({
      ...config,
      ignore: { capture_apps: [{ name: " " }], displays: [{ id: { vendor: 0, model: 1, serial: null } }] },
    });
    expect(issues.map((issue) => [issue.field, issue.code])).toEqual([
      ["ignore.capture_apps[0]", "empty_app"],
      ["ignore.displays[0]", "invalid_display_vendor"],
    ]);
  });
});
