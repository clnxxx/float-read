export interface LoadedText {
  text: string;
  encoding: string;
  path: string;
}

export type FontColor = "white" | "black";

export interface WindowGeometry {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface AppConfig {
  fontFamily: string;
  fontSize: number;
  fontColor: FontColor;
  recentFiles: string[];
  progress: Record<string, number>;
  window: WindowGeometry | null;
  autoScrollSpeed: number;
}

export const MIN_FONT_SIZE = 12;
export const MAX_FONT_SIZE = 48;
export const MIN_AUTO_SCROLL_SPEED = 10;
export const MAX_AUTO_SCROLL_SPEED = 400;
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
    window: null,
    autoScrollSpeed: 40,
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

/** progress 只进不出会无限膨胀；超过上限时优先保留最近文件的记录 */
const MAX_PROGRESS_ENTRIES = 200;

function pruneProgress(
  progress: Record<string, number>,
  recent: string[],
): Record<string, number> {
  const keys = Object.keys(progress);
  if (keys.length <= MAX_PROGRESS_ENTRIES) return progress;
  const kept: Record<string, number> = {};
  for (const path of recent) {
    if (path in progress) kept[path] = progress[path];
  }
  for (const key of keys) {
    if (Object.keys(kept).length >= MAX_PROGRESS_ENTRIES) break;
    if (!(key in kept)) kept[key] = progress[key];
  }
  return kept;
}

function normalizeWindowGeometry(v: unknown): WindowGeometry | null {
  if (!v || typeof v !== "object") return null;
  const o = v as Record<string, unknown>;
  const x = Number(o.x);
  const y = Number(o.y);
  const width = Number(o.width);
  const height = Number(o.height);
  if (![x, y, width, height].every(Number.isFinite)) return null;
  return {
    x,
    y,
    width: Math.min(10000, Math.max(80, width)),
    height: Math.min(10000, Math.max(18, height)),
  };
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
  const speed = Math.round(Number(input.autoScrollSpeed));
  const autoScrollSpeed = Number.isFinite(speed)
    ? Math.min(MAX_AUTO_SCROLL_SPEED, Math.max(MIN_AUTO_SCROLL_SPEED, speed))
    : base.autoScrollSpeed;
  return {
    fontFamily,
    fontSize,
    fontColor,
    recentFiles,
    progress: pruneProgress(progress, recentFiles),
    window: normalizeWindowGeometry(input.window),
    autoScrollSpeed,
  };
}

export function pushRecent(recent: string[], path: string): string[] {
  const next = [path, ...recent.filter((p) => p !== path)];
  return next.slice(0, 10);
}
