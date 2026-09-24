import "./style.css";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

type ResizeDir =
  | "East"
  | "North"
  | "NorthEast"
  | "NorthWest"
  | "South"
  | "SouthEast"
  | "SouthWest"
  | "West";
import { applyFont, onScroll, renderText, restoreScroll, showPlaceholder } from "./reader";
import { initSettingsUi } from "./settings";
import {
  defaultConfig,
  normalizeConfig,
  pushRecent,
  type AppConfig,
  type LoadedText,
} from "./types";

let config: AppConfig = defaultConfig();
let currentPath: string | null = null;
let saveTimer: number | undefined;

const btnOpen = document.getElementById("btn-open") as HTMLButtonElement;
const btnFont = document.getElementById("btn-font") as HTMLButtonElement;
const btnHide = document.getElementById("btn-hide") as HTMLButtonElement;
const appRoot = document.getElementById("app") as HTMLElement;

const settings = initSettingsUi({
  onFontChange: (family, size, color) => {
    config.fontFamily = family;
    config.fontSize = size;
    config.fontColor = color;
    applyFont(family, size, color);
    scheduleSave();
  },
  onOpenRecent: (path) => {
    void openFile(path);
  },
});

function scheduleSave(): void {
  window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    void invoke("save_config_cmd", { config }).catch((err) => {
      console.error("save config failed", err);
    });
  }, 200);
}

function persistProgress(ratio: number): void {
  if (!currentPath) return;
  config.progress = { ...config.progress, [currentPath]: ratio };
  scheduleSave();
}

async function loadConfig(): Promise<void> {
  try {
    const raw = await invoke<AppConfig>("load_config_cmd");
    config = normalizeConfig(raw);
  } catch (err) {
    console.error("load config failed", err);
    config = defaultConfig();
  }
  settings.setConfig(config);
  applyFont(config.fontFamily, config.fontSize, config.fontColor);
}

async function openFile(path?: string): Promise<void> {
  let target = path;
  if (!target) {
    await invoke("set_suspend_blur_hide", { suspend: true }).catch(() => {});
    try {
      const picked = await openDialog({
        multiple: false,
        filters: [{ name: "书籍", extensions: ["txt", "epub", "pdf"] }],
      });
      if (typeof picked === "string") target = picked;
    } finally {
      await invoke("set_suspend_blur_hide", { suspend: false }).catch(() => {});
    }
  }
  if (!target) return;

  try {
    const loaded = await invoke<LoadedText>("load_text", { path: target });
    currentPath = loaded.path;
    renderText(loaded);
    config.recentFiles = pushRecent(config.recentFiles, loaded.path);
    settings.setConfig(config);
    scheduleSave();
    const ratio = config.progress[loaded.path] ?? 0;
    // 等一帧确保布局完成
    requestAnimationFrame(() => restoreScroll(ratio));
  } catch (err) {
    console.error("open file failed", err);
    // 失败时不清空当前正文，只在还没有书时回到占位
    if (!currentPath) showPlaceholder(true);
  }
}

function hideWindow(): void {
  void getCurrentWindow().hide();
}

function mountDragEdges(): void {
  for (const side of ["top", "bottom", "left", "right"]) {
    const el = document.createElement("div");
    el.className = `drag-edge ${side}`;
    el.setAttribute("data-tauri-drag-region", "");
    appRoot.appendChild(el);
  }
  for (const el of document.querySelectorAll("#chrome, #panel")) {
    el.addEventListener("mousedown", (e) => e.stopPropagation());
  }
}

function mountResizeHandles(): void {
  for (const el of document.querySelectorAll<HTMLElement>(".resize-handle")) {
    const dir = el.dataset.dir as ResizeDir | undefined;
    if (!dir) continue;
    el.addEventListener("mousedown", (e) => {
      e.preventDefault();
      e.stopPropagation();
      void getCurrentWindow().startResizeDragging(dir);
    });
  }
}

function mountSlowWheel(): void {
  const reader = document.getElementById("reader")!;
  // 系统滚轮步进偏大，改用手动小步滚动
  reader.addEventListener(
    "wheel",
    (e) => {
      e.preventDefault();
      const dy = e.deltaMode === 1 ? e.deltaY * 24 : e.deltaY;
      reader.scrollTop += dy * 0.35;
    },
    { passive: false },
  );
}

async function bootstrap(): Promise<void> {
  mountDragEdges();
  mountResizeHandles();
  mountSlowWheel();
  await loadConfig();
  showPlaceholder(true);

  btnOpen.addEventListener("click", () => void openFile());
  btnFont.addEventListener("click", () => settings.togglePanel());
  btnHide.addEventListener("click", hideWindow);

  onScroll((ratio) => persistProgress(ratio));

  // 左手键区翻页：W/S 滚动，A/D 按页；焦点在表单控件时不抢键
  document.addEventListener("keydown", (e) => {
    const t = e.target as HTMLElement | null;
    if (t && (t.tagName === "INPUT" || t.tagName === "SELECT" || t.tagName === "TEXTAREA")) {
      return;
    }
    if (e.metaKey || e.ctrlKey || e.altKey) return;

    const reader = document.getElementById("reader")!;
    const line = 48;
    const page = Math.max(80, reader.clientHeight * 0.9);

    switch (e.key) {
      case "w":
      case "W":
      case "ArrowUp":
        e.preventDefault();
        reader.scrollTop = Math.max(0, reader.scrollTop - line);
        break;
      case "s":
      case "S":
      case "ArrowDown":
        e.preventDefault();
        reader.scrollTop = reader.scrollTop + line;
        break;
      case "a":
      case "A":
      case "PageUp":
        e.preventDefault();
        reader.scrollTop = Math.max(0, reader.scrollTop - page);
        break;
      case "d":
      case "D":
      case "PageDown":
      case " ":
        e.preventDefault();
        reader.scrollTop = reader.scrollTop + page;
        break;
      case "Home":
        e.preventDefault();
        reader.scrollTop = 0;
        break;
      case "End":
        e.preventDefault();
        reader.scrollTop = reader.scrollHeight;
        break;
      case "Escape":
        if (!settings.isOpen()) hideWindow();
        else settings.togglePanel();
        break;
      default:
        break;
    }
  });

  // 自动恢复上次文件
  const last = config.recentFiles[0];
  if (last) {
    void openFile(last);
  }
}

void bootstrap();
