# 浮阅 · 透明摸鱼阅读

几乎只显示文字的透明悬浮阅读窗：改字体/字号/黑白字色，全局热键或失焦快速隐蔽，记住进度。

支持 **TXT / EPUB / PDF**（EPUB、PDF 抽正文后可变字号重排）；窗口边缘/四角可拖拽改大小。

## 开发

```bash
npm install
npm run test
npm run typecheck
npm run tauri dev
```

## 默认热键

- `Cmd/Ctrl + Shift + H`：隐藏 / 恢复
- 点击其它窗口：自动隐藏
- `Esc`：隐藏（面板打开时先关面板）

托盘图标可再次唤出或退出。
