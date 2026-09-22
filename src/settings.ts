import { clampFontSize, FONT_OPTIONS, normalizeConfig, pushRecent, type AppConfig } from "./types";

const familySel = document.getElementById("font-family") as HTMLSelectElement;
const sizeInput = document.getElementById("font-size") as HTMLInputElement;
const sizeVal = document.getElementById("font-size-val") as HTMLSpanElement;
const panel = document.getElementById("panel") as HTMLElement;
const recentList = document.getElementById("recent-list") as HTMLUListElement;

export type FontChangeHandler = (family: string, size: number) => void;
export type OpenRecentHandler = (path: string) => void;

export function initSettingsUi(opts: {
  onFontChange: FontChangeHandler;
  onOpenRecent: OpenRecentHandler;
}): { setConfig: (cfg: AppConfig) => void; togglePanel: () => void; isOpen: () => boolean } {
  for (const f of FONT_OPTIONS) {
    const opt = document.createElement("option");
    opt.value = f;
    opt.textContent = f;
    familySel.appendChild(opt);
  }

  familySel.addEventListener("change", () => {
    opts.onFontChange(familySel.value, Number(sizeInput.value));
  });
  sizeInput.addEventListener("input", () => {
    const size = clampFontSize(Number(sizeInput.value));
    sizeVal.textContent = String(size);
    opts.onFontChange(familySel.value, size);
  });

  function renderRecent(recent: string[]) {
    recentList.innerHTML = "";
    if (recent.length === 0) {
      const li = document.createElement("li");
      li.textContent = "暂无";
      li.style.opacity = "0.5";
      recentList.appendChild(li);
      return;
    }
    for (const path of recent) {
      const li = document.createElement("li");
      const btn = document.createElement("button");
      btn.type = "button";
      btn.textContent = path.split("/").pop() || path;
      btn.title = path;
      btn.addEventListener("click", () => opts.onOpenRecent(path));
      li.appendChild(btn);
      recentList.appendChild(li);
    }
  }

  function setConfig(cfg: AppConfig) {
    const normalized = normalizeConfig(cfg);
    familySel.value = normalized.fontFamily;
    // 若系统无此字体，仍保留值，下拉会显示第一项；以 value 优先匹配
    if (familySel.value !== normalized.fontFamily) {
      const opt = document.createElement("option");
      opt.value = normalized.fontFamily;
      opt.textContent = normalized.fontFamily;
      familySel.insertBefore(opt, familySel.firstChild);
      familySel.value = normalized.fontFamily;
    }
    sizeInput.value = String(normalized.fontSize);
    sizeVal.textContent = String(normalized.fontSize);
    renderRecent(normalized.recentFiles);
  }

  function togglePanel() {
    panel.hidden = !panel.hidden;
  }

  return {
    setConfig,
    togglePanel,
    isOpen: () => !panel.hidden,
  };
}

export function nextRecent(recent: string[], path: string): string[] {
  return pushRecent(recent, path);
}
