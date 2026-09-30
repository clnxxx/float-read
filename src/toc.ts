export interface TocEntry {
  title: string;
  /** 匹配行在全文中的字符偏移（与 reader.ts 的分块偏移同一体系） */
  pos: number;
}

export const MAX_TOC_ENTRIES = 2000;

/**
 * 只认「整行就是标题」的行，正文里提到“第一章”不会命中。
 * 不含 部/篇/集：正文里“第一部门”“第一篇论文”这类整行误报太容易。
 */
const HEADING_SOURCE =
  "^[ \\t\\u3000]*(?:" +
  "第[ \\t\\u3000]?[0-9０-９零一二三四五六七八九十百千万两]{1,9}[ \\t\\u3000]?[章节回卷][ \\t\\u3000]?[^\\n]{0,40}" +
  "|Chapter[ \\t]+[0-9]{1,4}[^\\n]{0,50}" +
  "|(?:楔子|序章|序幕|引子|前言|序言|后记|尾声|终章|番外)[^\\n]{0,40}" +
  ")[ \\t\\u3000]*$";

export function extractToc(text: string): TocEntry[] {
  const out: TocEntry[] = [];
  const re = new RegExp(HEADING_SOURCE, "gim");
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    const title = m[0].trim();
    if (title) out.push({ title, pos: m.index });
    if (out.length >= MAX_TOC_ENTRIES) break;
  }
  return out;
}
