import { expect, test } from "bun:test";

import { normalizeHost, parseList } from "./snapshot.ts";

test("hosts files lose addresses, comments and local names", () => {
  const hosts = [
    "# Title: example",
    "127.0.0.1 localhost",
    "::1 localhost",
    "255.255.255.255 broadcasthost",
    "0.0.0.0 Ads.Example.COM  # trailing comment",
    "0.0.0.0 b.example.com c.example.com",
    "0.0.0.0 ads.example.com",
    "",
  ].join("\n");
  expect(parseList(hosts)).toEqual(["ads.example.com", "b.example.com", "c.example.com"]);
});

test("plain domain lists are lowercased, deduplicated and sorted", () => {
  expect(parseList("zeta.hu\r\nAlpha.hu\nalpha.hu.\n*.wild.hu\n")).toEqual(["alpha.hu", "wild.hu", "zeta.hu"]);
});

test("names are punycoded and invalid ones dropped", () => {
  expect(normalizeHost("bücher.de")).toBe("xn--bcher-kva.de");
  expect(normalizeHost("192.168.1.1")).toBeNull();
  expect(normalizeHost("localhost")).toBeNull();
  expect(normalizeHost("-bad.example.com")).toBeNull();
  expect(normalizeHost("bad-.example.com")).toBeNull();
  expect(normalizeHost("example.c0m")).toBeNull();
  expect(normalizeHost(`${"a".repeat(64)}.com`)).toBeNull();
  expect(normalizeHost("under_score.example.com")).toBe("under_score.example.com");
});
