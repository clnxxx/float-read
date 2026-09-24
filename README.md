# 浮阅 · 透明摸鱼阅读

几乎只显示文字的透明悬浮阅读窗：改字体/字号/黑白字色，全局热键或失焦快速隐蔽，记住进度。

支持 **TXT / EPUB / PDF**（EPUB、PDF 抽正文后可变字号重排）；窗口边缘/四角可拖拽改大小。

## 安装

从 [GitHub Releases](../../releases) 下载：

| 平台 | 安装包 |
|------|--------|
| macOS (Intel / Apple Silicon) | `浮阅_*_universal.dmg` 或 `.app` |
| Windows 10/11 x64 | `浮阅_*-setup.exe` |

macOS 若提示无法验证：右键 App → 打开，或 `xattr -cr /Applications/浮阅.app`。

## 使用

- 打开 TXT / EPUB / PDF
- 拖住正文移动窗口；边缘/四角拖拽改大小
- 字体、字号、黑白字色在右上角「字体」里（鼠标移上去才显示）
- **⌘⇧H** / **Ctrl+Shift+H** 一键隐蔽；点到其它窗口自动隐藏
- 聚焦时：`W/S` 微调，`A/D` 按页，`Home/End` 首尾

## 开发

```bash
npm install
npm run test
npm run tauri dev
npm run tauri build   # 本机安装包
```

## 发版（macOS + Windows）

1. 在 GitHub 新建仓库并推送本项目
2. 打标签并推送：`git tag v0.1.0 && git push origin v0.1.0`
3. Actions 的 `release` 工作流会在 macOS / Windows 上构建，把 dmg / exe 挂到 **Draft Release**
4. 到 Releases 页检查后点 **Publish**

## License

[MIT](./LICENSE)
