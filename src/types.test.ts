import { describe, expect, it } from "vitest";
import {
  clampFontSize,
  clampRatio,
  normalizeConfig,
  pushRecent,
  defaultConfig,
} from "./types";

describe("clampRatio", () => {
  it("clamps into unit interval", () => {
    expect(clampRatio(-1)).toBe(0);
    expect(clampRatio(0.25)).toBe(0.25);
    expect(clampRatio(2)).toBe(1);
    expect(clampRatio(Number.NaN)).toBe(0);
  });
});

describe("clampFontSize", () => {
  it("keeps size in 12..48", () => {
    expect(clampFontSize(5)).toBe(12);
    expect(clampFontSize(18)).toBe(18);
    expect(clampFontSize(90)).toBe(48);
    expect(clampFontSize(Number.NaN)).toBe(18);
  });
});

describe("normalizeConfig", () => {
  it("fills defaults and filters junk", () => {
    const cfg = normalizeConfig({
      fontFamily: "  ",
      fontSize: 999,
      fontColor: "red",
      recentFiles: ["a", "", "b"],
      progress: { a: 0.5, b: 9, c: Number.NaN },
    });
    expect(cfg.fontFamily).toBe(defaultConfig().fontFamily);
    expect(cfg.fontSize).toBe(48);
    expect(cfg.fontColor).toBe("white");
    expect(cfg.recentFiles).toEqual(["a", "b"]);
    expect(cfg.progress.a).toBe(0.5);
    expect(cfg.progress.b).toBe(1);
    expect(cfg.progress.c).toBe(0);
  });

  it("keeps black font color", () => {
    const cfg = normalizeConfig({ fontColor: "black" });
    expect(cfg.fontColor).toBe("black");
  });
});

describe("pushRecent", () => {
  it("moves path to front and caps at 10", () => {
    let recent: string[] = [];
    for (let i = 0; i < 12; i++) recent = pushRecent(recent, `/f${i}`);
    expect(recent[0]).toBe("/f11");
    expect(recent).toHaveLength(10);
    recent = pushRecent(recent, "/f5");
    expect(recent[0]).toBe("/f5");
    expect(recent.filter((p) => p === "/f5")).toHaveLength(1);
  });
});
