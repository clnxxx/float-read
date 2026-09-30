import "./style.css";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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
import { initAutoScroll } from "./autoScroll";
import { initSettingsUi } from "./settings";
import { initTocPanel } from "./tocPanel";
import { extractToc } from "./toc";
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
/** 文件对话框打开期间挂起「鼠标移出隐藏」（鼠标移向对话框必然离开主窗口） */
let dialogOpen = false;
/** 原生拖拽/缩放进行中：窗口事件循环被占用，期间不做鼠标移出隐藏 */
let nativeDrag = false;

const btnOpen = document.getElementById("btn-open") as HTMLButtonElement;
const btnFont = document.getElementById("btn-font") as HTMLButtonElement;
const btnToc = document.getElementById("btn-toc") as HTMLButtonElement;
const btnHide = document.getElementById("btn-hide") as HTMLButtonElement;

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

const tocPanel = initTocPanel();

const autoScroll = initAutoScroll({
  toast: showToast,
  getSpeed: () => config.autoScrollSpeed,
  setSpeed: (v) => {
    config.autoScrollSpeed = v;
    scheduleSave();
  },
});

function scheduleSave(): void {
  window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    void invoke("save_config_cmd", { config }).catch((err) => {
      console.error("save config failed", err);
    });
  }, 400);
}

function persistProgress(ratio: number): void {
  if (!currentPath) return;
  config.progress[currentPath] = ratio;
  scheduleSave();
}

let toastTimer: number | undefined;

function showToast(message: string): void {
  const el = document.getElementById("toast");
  if (!el) return;
  el.textContent = message;
  el.hidden = false;
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => {
    el.hidden = true;
  }, 3000);
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
    dialogOpen = true;
    await invoke("set_suspend_blur_hide", { suspend: true }).catch(() => {});
    try {
      const picked = await openDialog({
        multiple: false,
        filters: [{ name: "书籍", extensions: ["txt", "epub", "pdf"] }],
      });
      if (typeof picked === "string") target = picked;
    } finally {
      dialogOpen = false;
      await invoke("set_suspend_blur_hide", { suspend: false }).catch(() => {});
    }
  }
  if (!target) return;
  // 换书时停掉自动滚动，避免渲染间隙被当成“已到末尾”
  autoScroll.stop();

  try {
    const loaded = await invoke<LoadedText>("load_text", { path: target });
    currentPath = loaded.path;
    await renderText(loaded);
    tocPanel.setEntries(extractToc(loaded.text));
    config.recentFiles = pushRecent(config.recentFiles, loaded.path);
    settings.setConfig(config);
    scheduleSave();
    const ratio = config.progress[loaded.path] ?? 0;
    restoreScroll(ratio);
  } catch (err) {
    console.error("open file failed", err);
    showToast(`打开失败：${err instanceof Error ? err.message : String(err)}`);
    if (!currentPath) showPlaceholder(true);
  }
}

function hideWindow(): void {
  autoScroll.setVisible(false);
  void getCurrentWindow().hide();
}

// 正文区兜底：子节点没有 drag-region 时也能拖动窗口
function mountReaderDrag(): void {
  const reader = document.getElementById("reader")!;
  reader.addEventListener("mousedown", (e) => {
    if (e.button === 1) {
      // 中键开/关自动滚动（同浏览器习惯），并拦住默认的滚动光标
      e.preventDefault();
      e.stopPropagation();
      autoScroll.toggle();
      return;
    }
    if (e.button !== 0) return;
    const t = e.target as HTMLElement | null;
    if (t?.closest("#chrome, #panel, #toc-panel, .resize-handle, button, select, input")) return;
    nativeDrag = true;
    void getCurrentWindow().startDragging();
  });
}

function mountResizeHandles(): void {
  for (const el of document.querySelectorAll<HTMLElement>(".resize-handle")) {
    const dir = el.dataset.dir as ResizeDir | undefined;
    if (!dir) continue;
    el.addEventListener("mousedown", (e) => {
      e.preventDefault();
      e.stopPropagation();
      nativeDrag = true;
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

// 鼠标移出窗口即隐藏，移回窗口范围由 Rust 轮询光标自动重显（不抢焦点）。
// 200ms 宽限：回来即取消，防拖拽/缩放时指针擦边误触发；对话框打开期间不隐藏。
function mountMouseLeaveHide(): void {
  const DELAY = 200;
  let timer: number | undefined;
  const cancel = (): void => {
    window.clearTimeout(timer);
  };
  const schedule = (): void => {
    if (dialogOpen || nativeDrag) return;
    window.clearTimeout(timer);
    timer = window.setTimeout(() => {
      if (dialogOpen) return;
      void invoke("hide_on_mouse_leave").catch(() => hideWindow());
    }, DELAY);
  };
  document.addEventListener("mouseleave", schedule);
  document.addEventListener("mouseenter", cancel);
  // 原生拖拽/缩放结束后恢复（松开左键后的第一次移动/抬起；拖拽期间 webview 收不到事件）
  document.addEventListener("mousemove", (e) => {
    if ((e.buttons & 1) === 0) nativeDrag = false;
  });
  document.addEventListener("mouseup", () => {
    nativeDrag = false;
  });
  // 兜底：部分环境只派发 mouseout（relatedTarget 为空 = 离开窗口）
  document.addEventListener("mouseout", (e) => {
    if (!e.relatedTarget) schedule();
  });
  document.addEventListener("mouseover", (e) => {
    if (!e.relatedTarget) cancel();
  });
}

async function bootstrap(): Promise<void> {
  mountReaderDrag();
  mountResizeHandles();
  mountSlowWheel();
  mountMouseLeaveHide();
  await loadConfig();
  showPlaceholder(true);

  btnOpen.addEventListener("click", () => void openFile());
  btnFont.addEventListener("click", () => {
    tocPanel.close();
    settings.togglePanel();
  });
  btnToc.addEventListener("click", () => {
    if (settings.isOpen()) settings.togglePanel();
    tocPanel.toggle();
  });
  btnHide.addEventListener("click", hideWindow);

  onScroll((ratio) => {
    persistProgress(ratio);
    tocPanel.notifyScroll();
  });

  // 窗口显隐（Rust 侧 hide/show 时发出）→ 暂停/恢复自动滚动
  void listen<boolean>("fr://visibility", (e) => autoScroll.setVisible(e.payload));

  // 左手键区翻页：W/S 滚动（自动滚动时改为调速），A/D 按页，E 开关自动滚动，T 目录
  // 焦点在表单控件时不抢键
  document.addEventListener("keydown", (e) => {
    const t = e.target as HTMLElement | null;
    if (t && (t.tagName === "INPUT" || t.tagName === "SELECT" || t.tagName === "TEXTAREA")) {
      return;
    }
    if (e.metaKey || e.ctrlKey || e.altKey) return;

    const reader = document.getElementById("reader")!;
    const line = 48;
    const page = Math.max(80, reader.clientHeight * 0.9);
    const auto = autoScroll.isEnabled();

    switch (e.key) {
      case "w":
      case "W":
      case "ArrowUp":
        e.preventDefault();
        if (auto) autoScroll.adjustSpeed(-10);
        else reader.scrollTop = Math.max(0, reader.scrollTop - line);
        break;
      case "s":
      case "S":
      case "ArrowDown":
        e.preventDefault();
        if (auto) autoScroll.adjustSpeed(10);
        else reader.scrollTop = reader.scrollTop + line;
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
      case "e":
      case "E":
        e.preventDefault();
        autoScroll.toggle();
        break;
      case "t":
      case "T":
        e.preventDefault();
        if (settings.isOpen()) settings.togglePanel();
        tocPanel.toggle();
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
        if (tocPanel.isOpen()) tocPanel.close();
        else if (settings.isOpen()) settings.togglePanel();
        else hideWindow();
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
