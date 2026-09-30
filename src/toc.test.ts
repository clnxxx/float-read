import { describe, expect, it } from "vitest";
import { extractToc, MAX_TOC_ENTRIES } from "./toc";

describe("extractToc", () => {
  it("finds chapter headings with offsets", () => {
    const text = "开头正文。\n第一章 起点\n他起了个大早。\n第二天出门。\n第十二章 峰回路转\n结尾";
    const toc = extractToc(text);
    expect(toc.map((e) => e.title)).toEqual(["第一章 起点", "第十二章 峰回路转"]);
    expect(text.slice(toc[0].pos, toc[0].pos + 6)).toBe("第一章 起点");
  });

  it("ignores in-sentence mentions", () => {
    const text = "他在第一章里提到过这件事。\n第二章 真标题\n正文第二天";
    expect(extractToc(text).map((e) => e.title)).toEqual(["第二章 真标题"]);
  });

  it("supports english chapters and prologue words", () => {
    const text = "Chapter 1: Begin\nsome text\n楔子\nChapter 12: The End";
    expect(extractToc(text).map((e) => e.title)).toEqual([
      "Chapter 1: Begin",
      "楔子",
      "Chapter 12: The End",
    ]);
  });

  it("returns empty when no headings", () => {
    expect(extractToc("随便写点正文\n没有标题行")).toEqual([]);
  });

  it("caps entries", () => {
    const text = Array.from({ length: 3000 }, (_, i) => `第${i + 1}章 x`).join("\n");
    expect(extractToc(text)).toHaveLength(MAX_TOC_ENTRIES);
  });
});
