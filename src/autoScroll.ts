/**
 * 自动滚动：rAF 匀速下滚。
 * - 滚轮 = 人工接管：挂起 1.2s，滚轮静默后自动恢复（惯性滚动不会再把刚开启的
 *   自动滚动瞬间停掉）；要彻底关闭按 E / 中键。
 * - 窗口隐藏时暂停（由 Rust 侧显隐时发 fr://visibility 事件驱动）。
 * - 滚动按整数步进 + 余数累积：WebKit 可能丢弃小数 scrollTop，
 *   每帧 0.6px 的小步进会被取整归零，表现为「开了但不动」。
 * - 读到末尾自动停止。速度档位持久化在 config.autoScrollSpeed。
 */
const readerEl = document.getElementById("reader") as HTMLElement;

export type AutoScrollDeps = {
  toast: (msg: string) => void;
  getSpeed: () => number;
  setSpeed: (v: number) => void;
};

export function initAutoScroll(deps: AutoScrollDeps): {
  toggle: () => void;
  adjustSpeed: (delta: number) => void;
  setVisible: (visible: boolean) => void;
  isEnabled: () => boolean;
  stop: () => void;
} {
  let enabled = false;
  let paused = false;
  let raf = 0;
  let last = 0;
  let wheelQuietUntil = 0;
  let carry = 0;

  const stop = (): void => {
    enabled = false;
    cancelAnimationFrame(raf);
    last = 0;
    carry = 0;
  };

  const tick = (now: number): void => {
    if (!enabled) return;
    if (!paused && last !== 0 && now >= wheelQuietUntil) {
      const dt = Math.min(0.1, (now - last) / 1000);
      const max = readerEl.scrollHeight - readerEl.clientHeight;
      if (max <= 0 || readerEl.scrollTop >= max) {
        stop();
        deps.toast("已到末尾，自动滚动停止");
        return;
      }
      carry += deps.getSpeed() * dt;
      const step = Math.floor(carry);
      if (step > 0) {
        carry -= step;
        readerEl.scrollTop = Math.min(max, readerEl.scrollTop + step);
      }
    }
    last = now;
    raf = requestAnimationFrame(tick);
  };

  readerEl.addEventListener(
    "wheel",
    () => {
      if (enabled) wheelQuietUntil = performance.now() + 1200;
    },
    { passive: true },
  );

  return {
    toggle: () => {
      if (enabled) {
        stop();
        deps.toast("自动滚动 关");
      } else {
        enabled = true;
        paused = false;
        wheelQuietUntil = 0;
        carry = 0;
        last = 0;
        raf = requestAnimationFrame(tick);
        deps.toast(`自动滚动 开 · ${deps.getSpeed()} px/秒（W/S 调速）`);
      }
    },
    adjustSpeed: (delta) => {
      if (!enabled) return;
      const next = Math.min(400, Math.max(10, deps.getSpeed() + delta));
      deps.setSpeed(next);
      deps.toast(`自动滚动 ${next} px/秒`);
    },
    setVisible: (visible) => {
      paused = !visible;
    },
    isEnabled: () => enabled,
    stop,
  };
}
