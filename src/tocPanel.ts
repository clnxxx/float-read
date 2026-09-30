import { blockIndexAtScrollTop, blockIndexForOffset, jumpToOffset } from "./reader";
import type { TocEntry } from "./toc";

const panel = document.getElementById("toc-panel") as HTMLElement;
const list = document.getElementById("toc-list") as HTMLUListElement;
const btnToc = document.getElementById("btn-toc") as HTMLButtonElement;
const btnClose = document.getElementById("toc-close") as HTMLButtonElement;

let entries: TocEntry[] = [];
/** 每条目录对应的块下标（与 entries 同序，pos 升序保证单调可顺序扫描） */
let blockIdx: number[] = [];
let active = -1;

function render(): void {
  list.replaceChildren();
  if (entries.length === 0) {
    const li = document.createElement("li");
    li.textContent = "未找到章节";
    li.style.opacity = "0.5";
    list.appendChild(li);
    return;
  }
  const frag = document.createDocumentFragment();
  for (const entry of entries) {
    const li = document.createElement("li");
    const btn = document.createElement("button");
    btn.type = "button";
    btn.textContent = entry.title;
    btn.title = entry.title;
    btn.addEventListener("click", () => {
      jumpToOffset(entry.pos);
      close();
    });
    li.appendChild(btn);
    frag.appendChild(li);
  }
  list.appendChild(frag);
}

function refreshActive(): void {
  if (entries.length === 0) return;
  const cur = blockIndexAtScrollTop();
  let idx = 0;
  for (let i = 0; i < blockIdx.length; i++) {
    if (blockIdx[i] <= cur) idx = i;
    else break;
  }
  if (idx === active) return;
  if (active >= 0) list.children[active]?.classList.remove("active");
  active = idx;
  (list.children[active] as HTMLElement | undefined)?.classList.add("active");
}

function close(): void {
  panel.hidden = true;
  // 交还焦点，防止列表按钮持有焦点吃掉快捷键
  (document.activeElement as HTMLElement | null)?.blur();
}

function toggle(): void {
  if (panel.hidden) {
    panel.hidden = false;
    refreshActive();
    list.querySelector(".active")?.scrollIntoView({ block: "center" });
  } else {
    close();
  }
}

export function initTocPanel(): {
  setEntries: (next: TocEntry[]) => void;
  toggle: () => void;
  close: () => void;
  isOpen: () => boolean;
  notifyScroll: () => void;
} {
  btnToc.disabled = true;
  btnClose.addEventListener("click", close);
  return {
    setEntries(next) {
      entries = next;
      blockIdx = entries.map((e) => blockIndexForOffset(e.pos));
      active = -1;
      btnToc.disabled = entries.length === 0;
      render();
      if (!panel.hidden) refreshActive();
    },
    toggle,
    close,
    isOpen: () => !panel.hidden,
    notifyScroll: () => {
      if (!panel.hidden) refreshActive();
    },
  };
}
