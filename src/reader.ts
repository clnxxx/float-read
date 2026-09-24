import { clampRatio, type FontColor, type LoadedText } from "./types";

const reader = document.getElementById("reader") as HTMLElement;
const content = document.getElementById("content") as HTMLElement;
const placeholder = document.getElementById("placeholder") as HTMLElement;

export function applyFont(family: string, size: number, color: FontColor): void {
  document.documentElement.style.setProperty("--reader-font-family", `"${family}", sans-serif`);
  document.documentElement.style.setProperty("--reader-font-size", `${size}px`);
  document.documentElement.style.setProperty(
    "--reader-ink",
    color === "black" ? "#141414" : "#f4f4f2",
  );
}

export function showPlaceholder(show: boolean): void {
  placeholder.hidden = !show;
  content.hidden = show;
}

/** 约每块字符数：过大会卡布局，过小会碎成上万节点 */
const BLOCK_CHARS = 3500;

function splitBlocks(text: string): string[] {
  if (text.length <= BLOCK_CHARS) return text ? [text] : [];
  const blocks: string[] = [];
  let pos = 0;
  while (pos < text.length) {
    let end = Math.min(text.length, pos + BLOCK_CHARS);
    if (end < text.length) {
      const nl = text.lastIndexOf("\n", end);
      if (nl > pos + BLOCK_CHARS * 0.55) end = nl + 1;
    }
    blocks.push(text.slice(pos, end));
    pos = end;
  }
  return blocks;
}

/**
 * 分块渲染：每块 content-visibility:auto，离屏块跳过布局。
 * 大文件分帧提交，避免一次性建 DOM 把界面冻住。
 */
export function renderText(loaded: LoadedText): Promise<void> {
  const text = loaded.text;
  content.replaceChildren();
  if (!text) {
    showPlaceholder(true);
    return Promise.resolve();
  }
  showPlaceholder(false);

  const blocks = splitBlocks(text);
  let i = 0;
  const CHUNK = 40;

  return new Promise((resolve) => {
    const pump = () => {
      const frag = document.createDocumentFragment();
      const end = Math.min(blocks.length, i + CHUNK);
      for (; i < end; i++) {
        const div = document.createElement("div");
        div.className = "block";
        div.textContent = blocks[i];
        div.setAttribute("data-tauri-drag-region", "");
        frag.appendChild(div);
      }
      content.appendChild(frag);
      if (i < blocks.length) {
        requestAnimationFrame(pump);
      } else {
        resolve();
      }
    };
    pump();
  });
}

export function scrollRatio(): number {
  const max = reader.scrollHeight - reader.clientHeight;
  if (max <= 0) return 0;
  return clampRatio(reader.scrollTop / max);
}

export function restoreScroll(ratio: number): void {
  const max = reader.scrollHeight - reader.clientHeight;
  reader.scrollTop = max * clampRatio(ratio);
}

export function onScroll(cb: (ratio: number) => void): () => void {
  const handler = () => cb(scrollRatio());
  reader.addEventListener("scroll", handler, { passive: true });
  return () => reader.removeEventListener("scroll", handler);
}