import { describe, expect, test } from "vitest";
import { acceleratorFromEvent, formatDur, hotkeyKeys, parseDur } from "../../src/format";
import { slug } from "../../src/model";

const keyEvent = (
  code: string,
  mods: Partial<Record<"ctrlKey" | "altKey" | "shiftKey" | "metaKey", boolean>>,
) => ({
  code,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
});

describe("hotkeys", () => {
  test("draws macOS symbols in the platform's modifier order", () => {
    expect(hotkeyKeys("Shift+Alt+Ctrl+B", true)).toEqual(["⌃", "⌥", "⇧", "B"]);
    expect(hotkeyKeys("CommandOrControl+Shift+K", true)).toEqual(["⇧", "⌘", "K"]);
  });

  test("keeps names elsewhere", () => {
    expect(hotkeyKeys("Ctrl+Alt+Shift+B", false)).toEqual(["Ctrl", "Alt", "Shift", "B"]);
  });

  test("records by physical key, so ⌥ on a Hungarian layout still gives B", () => {
    expect(acceleratorFromEvent(keyEvent("KeyB", { ctrlKey: true, altKey: true, shiftKey: true }))).toBe(
      "Ctrl+Alt+Shift+B",
    );
    expect(acceleratorFromEvent(keyEvent("Digit5", { metaKey: true }))).toBe("Super+5");
  });

  test("refuses a key without a modifier, or a lone modifier", () => {
    expect(acceleratorFromEvent(keyEvent("KeyB", {}))).toBeNull();
    expect(acceleratorFromEvent(keyEvent("ShiftLeft", { shiftKey: true }))).toBeNull();
  });
});

describe("durations", () => {
  test("parse the config format", () => {
    expect(parseDur("15m")).toEqual({ value: 15, unit: "m" });
    expect(parseDur("1.5h")).toBeNull();
  });

  test("format with the singular noun after a number in Hungarian", () => {
    expect(formatDur("15m", "hu")).toBe("15 perc");
    expect(formatDur("2h", "hu")).toBe("2 óra");
    expect(formatDur("2h", "en")).toBe("2 hours");
  });
});

describe("category ids", () => {
  test("are suggested from the name without accents or punctuation", () => {
    expect(slug("Bérportál (munka)")).toBe("berportal_munka");
    expect(slug("  Őrült ÜGYEK  ")).toBe("orult_ugyek");
  });
});
