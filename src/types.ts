export interface LoadedText {
  text: string;
  encoding: string;
  path: string;
}

export type FontColor = "white" | "black";

export interface AppConfig {
  fontFamily: string;
  fontSize: number;
  fontColor: FontColor;
  recentFiles: string[];
  progress: Record<string, number>;
}

export const MIN_FONT_SIZE = 12;
export const MAX_FONT_SIZE = 48;
export const DEFAULT_FONT_FAMILY = "PingFang SC";
export const DEFAULT_FONT_COLOR: FontColor = "white";

export const FONT_OPTIONS = [
  "PingFang SC",
  "Songti SC",
  "STKaiti",
  "Heiti SC",
  "Hiragino Sans GB",
  "Menlo",
  "Georgia",
] as const;

export function defaultConfig(): AppConfig {
  return {
    fontFamily: DEFAULT_FONT_FAMILY,
    fontSize: 18,
    fontColor: DEFAULT_FONT_COLOR,
    recentFiles: [],
    progress: {},
  };
}

export function clampRatio(ratio: number): number {
  if (!Number.isFinite(ratio)) return 0;
  return Math.min(1, Math.max(0, ratio));
}

export function clampFontSize(size: number): number {
  if (!Number.isFinite(size)) return 18;
  return Math.min(MAX_FONT_SIZE, Math.max(MIN_FONT_SIZE, Math.round(size)));
}

export function normalizeFontColor(value: unknown): FontColor {
  return value === "black" ? "black" : "white";
}

export function normalizeConfig(input: Partial<AppConfig> | null | undefined): AppConfig {
  const base = defaultConfig();
  if (!input || typeof input !== "object") return base;
  const fontFamily =
    typeof input.fontFamily === "string" && input.fontFamily.trim()
      ? input.fontFamily.trim()
      : base.fontFamily;
  const fontSize = clampFontSize(Number(input.fontSize));
  const fontColor = normalizeFontColor(input.fontColor);
  const recentFiles = Array.isArray(input.recentFiles)
    ? input.recentFiles.filter((x): x is string => typeof x === "string" && x.length > 0).slice(0, 10)
    : [];
  const progress: Record<string, number> = {};
  if (input.progress && typeof input.progress === "object") {
    for (const [k, v] of Object.entries(input.progress)) {
      const r = clampRatio(Number(v));
      if (Number.isFinite(r)) progress[k] = r;
    }
  }
  return { fontFamily, fontSize, fontColor, recentFiles, progress };
}

export function pushRecent(recent: string[], path: string): string[] {
  const next = [path, ...recent.filter((p) => p !== path)];
  return next.slice(0, 10);
}
