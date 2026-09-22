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

export function renderText(loaded: LoadedText): void {
  content.textContent = loaded.text;
  showPlaceholder(loaded.text.length === 0);
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
