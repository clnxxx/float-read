import "./style.css";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
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
  onFontChange: (family, size) => {
    config.fontFamily = family;
    config.fontSize = size;
    applyFont(family, size);
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
  applyFont(config.fontFamily, config.fontSize);
}

async function openFile(path?: string): Promise<void> {
  let target = path;
  if (!target) {
    await invoke("set_suspend_blur_hide", { suspend: true }).catch(() => {});
    try {
      const picked = await openDialog({
        multiple: false,
        filters: [{ name: "纯文本", extensions: ["txt"] }],
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
  // 正文区按住左键即可拖动窗口；按钮/面板不参与拖拽
  for (const el of document.querySelectorAll("#chrome, #panel")) {
    el.addEventListener("mousedown", (e) => e.stopPropagation());
  }
}

async function bootstrap(): Promise<void> {
  mountDragEdges();
  await loadConfig();
  showPlaceholder(true);

  btnOpen.addEventListener("click", () => void openFile());
  btnFont.addEventListener("click", () => settings.togglePanel());
  btnHide.addEventListener("click", hideWindow);

  onScroll((ratio) => persistProgress(ratio));

  // 打开对话框期间暂时关闭「失焦隐藏」在前端的额外动作；窗口层由 Rust 热键豁免兜底
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      if (!settings.isOpen()) hideWindow();
      else settings.togglePanel();
    }
  });

  // 自动恢复上次文件
  const last = config.recentFiles[0];
  if (last) {
    void openFile(last);
  }
}

void bootstrap();
