// Unit tests for the status bar's reading of the host line and its clock text.
// Run with `bun run shelf:test`.

import { describe, expect, test } from "bun:test";
import { formatClock, parseStatus, sameStatus, wifiLevel } from "../status.ts";

describe("parseStatus", () => {
  test("reads every field", () => {
    expect(parseStatus("21 47 1 72 0 80 1")).toEqual({
      hour: 21,
      minute: 47,
      clock24: true,
      battery: 72,
      charging: false,
      wifi: 80,
      bluetooth: true,
    });
  });

  test("-1 is a reading the host could not give", () => {
    expect(parseStatus("9 5 -1 -1 1 -1 -1")).toEqual({
      hour: 9,
      minute: 5,
      clock24: null,
      battery: null,
      charging: true,
      wifi: null,
      bluetooth: null,
    });
  });

  test("0 is a reading: no signal, empty battery, Bluetooth off", () => {
    const status = parseStatus("0 0 0 0 0 0 0");
    expect(status?.wifi).toBe(0);
    expect(status?.battery).toBe(0);
    expect(status?.bluetooth).toBe(false);
    expect(status?.clock24).toBe(false);
  });

  test("rejects a line it does not understand", () => {
    expect(parseStatus("")).toBeNull();
    expect(parseStatus("21 47 1 72 0 80")).toBeNull();
    expect(parseStatus("21 47 1 72 0 80 x")).toBeNull();
    expect(parseStatus("-1 -1 1 72 0 80 1")).toBeNull();
    expect(parseStatus("24 0 1 72 0 80 1")).toBeNull();
  });
});

describe("formatClock", () => {
  test("24-hour, and when the format is unknown", () => {
    expect(formatClock(9, 5, true)).toBe("09:05");
    expect(formatClock(21, 47, null)).toBe("21:47");
  });

  test("12-hour", () => {
    expect(formatClock(0, 5, false)).toBe("12:05 AM");
    expect(formatClock(11, 59, false)).toBe("11:59 AM");
    expect(formatClock(12, 0, false)).toBe("12:00 PM");
    expect(formatClock(21, 47, false)).toBe("9:47 PM");
  });
});

test("wifiLevel", () => {
  expect([1, 24, 25, 49, 50, 74, 75, 100].map(wifiLevel)).toEqual([0, 0, 1, 1, 2, 2, 3, 3]);
});

test("sameStatus", () => {
  const a = parseStatus("21 47 1 72 0 80 1");
  expect(sameStatus(a, parseStatus("21 47 1 72 0 80 1"))).toBe(true);
  expect(sameStatus(a, parseStatus("21 48 1 72 0 80 1"))).toBe(false);
  expect(sameStatus(null, null)).toBe(true);
  expect(sameStatus(a, null)).toBe(false);
});
