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

/** 每块在全文中的起始偏移，供目录按偏移定位 */
let blockStarts: number[] = [];

function splitBlocks(text: string): { blocks: string[]; starts: number[] } {
  if (text.length <= BLOCK_CHARS) {
    return text ? { blocks: [text], starts: [0] } : { blocks: [], starts: [] };
  }
  const blocks: string[] = [];
  const starts: number[] = [];
  let pos = 0;
  while (pos < text.length) {
    starts.push(pos);
    let end = Math.min(text.length, pos + BLOCK_CHARS);
    if (end < text.length) {
      const nl = text.lastIndexOf("\n", end);
      if (nl > pos + BLOCK_CHARS * 0.55) end = nl + 1;
    }
    blocks.push(text.slice(pos, end));
    pos = end;
  }
  return { blocks, starts };
}

/** 渲染代数：换书时新渲染会使旧渲染泵失效，否则两本书的内容会交替追加进 DOM */
let renderGeneration = 0;

/**
 * 分块渲染：每块 content-visibility:auto，离屏块跳过布局。
 * 大文件分帧提交，避免一次性建 DOM 把界面冻住。
 * 被更新的渲染取代时旧 promise 永不 resolve，调用方的后续恢复逻辑随之中止。
 */
export function renderText(loaded: LoadedText): Promise<void> {
  const gen = ++renderGeneration;
  const text = loaded.text;
  content.replaceChildren();
  if (!text) {
    blockStarts = [];
    showPlaceholder(true);
    return Promise.resolve();
  }
  showPlaceholder(false);

  const { blocks, starts } = splitBlocks(text);
  blockStarts = starts;
  let i = 0;
  const CHUNK = 40;
  let calibrated = false;

  return new Promise((resolve) => {
    const pump = () => {
      if (gen !== renderGeneration) return;
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
      if (!calibrated) {
        calibrated = true;
        calibrateIntrinsicSize();
      }
      if (i < blocks.length) {
        requestAnimationFrame(pump);
      } else {
        resolve();
      }
    };
    pump();
  });
}

/**
 * 实测首块真实高度（em 倍数）供占位使用：contain-intrinsic-size 的估算值
 * 偏小时 scrollHeight 严重失真，阅读比例、进度恢复、End 键全都会漂移。
 */
function calibrateIntrinsicSize(): void {
  const first = content.querySelector<HTMLElement>(".block");
  if (!first) return;
  const height = first.offsetHeight;
  const fontSize = Number.parseFloat(getComputedStyle(first).fontSize);
  if (height > 0 && Number.isFinite(fontSize) && fontSize > 0) {
    content.style.setProperty("--block-est-em", (height / fontSize).toFixed(1));
  }
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

/** 全文中字符偏移 pos 所在的块下标（blockStarts 升序，二分） */
export function blockIndexForOffset(pos: number): number {
  const starts = blockStarts;
  if (starts.length === 0) return -1;
  let lo = 0;
  let hi = starts.length - 1;
  let ans = 0;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (starts[mid] <= pos) {
      ans = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return ans;
}

/** 视口顶部当前所在的块下标（离屏块 offsetTop 是估算值，足够用于目录高亮） */
export function blockIndexAtScrollTop(): number {
  const n = content.childElementCount;
  if (n === 0) return -1;
  const target = reader.scrollTop + 1;
  let lo = 0;
  let hi = n - 1;
  let ans = 0;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if ((content.children[mid] as HTMLElement).offsetTop <= target) {
      ans = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return ans;
}

export function jumpToOffset(pos: number): void {
  const idx = blockIndexForOffset(pos);
  const el = idx >= 0 ? (content.children[idx] as HTMLElement | undefined) : undefined;
  if (!el) return;
  const apply = () => {
    reader.scrollTop = Math.max(0, el.offsetTop - 8);
  };
  apply();
  // 离屏块高度是估算值，落点会随邻近块真实渲染而移动，连帧校正直到稳定
  let prev = -1;
  let frames = 0;
  const cancel = () => {
    frames = 1 << 20;
  };
  const settle = () => {
    const top = Math.max(0, el.offsetTop - 8);
    if (top === prev || frames >= 30) return;
    frames += 1;
    prev = top;
    apply();
    requestAnimationFrame(settle);
  };
  reader.addEventListener("wheel", cancel, { once: true, passive: true });
  reader.addEventListener("mousedown", cancel, { once: true });
  document.addEventListener("keydown", cancel, { once: true });
  requestAnimationFrame(settle);
}