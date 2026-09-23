---
feature: transparent-reader
status: delivered
updated: 2026-09-23
branch: feature/transparent-reader
commits: 0559e2f9d2b229720c5cc26932761275b6679c13..118c3c17bca3848176e249c529352ba9be73fd68
---

# 浮阅 · 透明悬浮看书

## Report

**What was built** — 桌面「浮阅」：Tauri 2 + Vite/TS 的透明悬浮 TXT 摸鱼阅读窗。无边框、透明、置顶、不进任务栏；正文纯色裸浮于桌面（无阴影、无描边，降低被注意概率）。支持打开 TXT（UTF-8 / GB18030），字体族与 12–48px 字号即时生效并持久化。快速隐蔽：全局热键 `Cmd/Ctrl+Shift+H` 切换显隐，失焦自动隐藏；系统文件对话框期间挂起误隐藏。连续滚动阅读，按文件记录滚动比例并恢复；最近 10 条可一键打开，启动自动续读上次文件。托盘菜单作为热键失灵时的保险丝。

**Verification** — `npm run test` PASS 4/4；`npm run typecheck` PASS；`npm run build` PASS；`cargo test` PASS 6/6（含 camelCase 序列化与配置 roundtrip、UTF-8/GB18030 解码）；`cargo check` PASS；debug 二进制 smoke 启动 2s 无 panic；`samples/demo-utf8.txt`、`samples/demo-gbk.txt` 编码解码正确。GUI 级热键/失焦/透明像素未做自动化，以代码审查 + 启动冒烟覆盖。

**Journey log** —
1. 选型敲定 Tauri 2 后环境缺 Rust，rustup 中断装坏 toolchain，重装 stable 才可编译。
2. 审查抓到 serde 字段大小写不一致导致配置静默回默认值——两侧单测都测不到 IPC 边界，补了 camelCase 往返测试。
3. encoding_rs 的 `GBK` ≠ 完整 GB18030，规格写了 GB18030 就该用 `GB18030`。
4. 失焦隐藏与系统文件对话框会打架：对话框一弹主窗就隐；用 `set_suspend_blur_hide` 挂起后解决。
5. 配置目录应跟 Tauri `app_config_dir`，避免写死 `~/Library` 破坏可移植性。

## [S1] Problem

在桌面上阅读 TXT 时，普通阅读器会遮挡工作区，切换窗口也容易被看到。需要一个几乎只显示文字的透明悬浮阅读窗：可改字体和字号，能一键/失焦快速隐蔽，并记住阅读进度。

## [S2] Design

### 产品形态

- 应用名：**浮阅**（内部工程名 `float-read`）
- 技术栈：**Tauri 2 + Vite + TypeScript**（无框架，原生 DOM）
- 窗口：无边框、透明背景、始终置顶、不进任务栏/Dock 主界面干扰；默认 260×320（约原 520×640 的 50%）
- 文字直接浮在桌面上，纯色无阴影/描边（隐蔽优先，不靠 text-shadow 保读）
- 极简交互：无常驻标题栏；按住正文/页边距即可拖拽移动悬浮窗；**不做划词选择**（user-select: none）；角落悬停露出极简工具（打开、字体设置、隐蔽）

### 功能范围

| 能力 | 行为 |
|------|------|
| 打开书籍 | 对话框支持 `.txt` / `.epub` / `.pdf`；TXT 用 UTF-8/GB18030；EPUB 按 spine 抽正文；PDF 抽纯文本重排 |
| 字体 | 字体族下拉（系统中文字体 + 常用衬线/无衬线/等宽），立即预览 |
| 字号 | 12–48px，滑杆 + 数字，立即预览 |
| 字色 | 黑 / 白 切换，立即生效并写入配置 |
| 窗口大小 | 边缘与四角手柄拖拽调整（最小 80×36，可收到约一行字） |
| 阅读 | 连续滚动；鼠标滚轮 / 触控板 |
| 进度 | 按文件绝对路径记录滚动比例（0–1），打开同一文件时恢复 |
| 最近文件 | 最多 10 条，设置面板可一键打开 |
| 快速隐蔽 | ① 全局热键 `CmdOrCtrl+Shift+H` 切换显示/隐藏 ② 窗口失焦自动隐藏 |
| 隐蔽语义 | 隐藏 = `window.hide()`（不销毁、不清进度）；恢复 = `window.show()` + 聚焦 |

### 架构

```
┌─────────────────────────────────────────────┐
│  WebView (Vite + TS)                        │
│  ├─ reader.ts   文本渲染 / 滚动进度          │
│  ├─ settings.ts 字体字号 / 最近文件 UI       │
│  └─ main.ts     装配、热键状态、防误隐藏      │
│                    ▲ invoke                 │
├────────────────────┼────────────────────────┤
│  Rust (src-tauri)  │                        │
│  ├─ load_text      读文件 + 编码探测         │
│  ├─ persist        配置与进度 JSON 读写      │
│  ├─ window         透明/置顶/拖拽/显隐       │
│  └─ global_shortcut CmdOrCtrl+Shift+H       │
│  数据目录: app_config_dir/float-read.json    │
└─────────────────────────────────────────────┘
```

### 合同

**配置文件** `float-read.json`（app config dir）：

```json
{
  "fontFamily": "PingFang SC",
  "fontSize": 18,
  "fontColor": "white",
  "recentFiles": ["/abs/path/a.txt"],
  "progress": { "/abs/path/a.txt": 0.42 }
}
```

Rust 侧 `AppConfig` 使用 `#[serde(rename_all = "camelCase")]` 与上述 JSON / TS 对齐。

**Rust commands**：

| Command | 入参 | 出参 |
|---------|------|------|
| `load_text` | `path: string` | `{ text, encoding, path }` |
| `load_config_cmd` | — | `Config` |
| `save_config_cmd` | `config: Config` | `()` |
| `set_suspend_blur_hide` | `suspend: boolean` | `()` |
| `toggle_window` | — | `visible: bool`（由全局热键在 Rust 侧直接处理，不强制走 invoke） |

**前端接口**：

- `applyFont(family: string, size: number): void` — 同步写 CSS 变量并 `save_config_cmd`
- `openFile(path?: string): Promise<void>` — 无参时弹系统对话框
- `restoreScroll(ratio: number): void` — 文本渲染后按比例滚到位置
- 防误隐藏：系统对话框打开期间挂起 `onBlur` 隐藏；全局热键恢复后 350ms 内不因 blur 再隐藏

### 窗口与隐蔽

- `tauri.conf.json`: `decorations: false`, `transparent: true`, `alwaysOnTop: true`, `shadow: false`, `skipTaskbar: true`
- macOS：`titleBarStyle` 保持隐藏；拖拽区域 = 阅读区四边 padding（`data-tauri-drag-region`）
- 全局热键：`global-shortcut` 插件注册 `CommandOrControl+Shift+H`，toggle hide/show
- `WebviewWindow::on_blur` → 隐藏；热键显示后的短暂豁免窗口防抖
- 隐藏时 `hide()` 不卸载 WebView，滚动位置与已读配置保持

### 字体与可读性

- 默认字体栈：`"PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", sans-serif`
- 可选：苹方 / 宋体 / 黑体 / 楷体 / Menlo / Georgia
- 几乎无底板，正文颜色近白 `#f5f5f4`，**不用 text-shadow / 描边**（阴影易被发现）
- 字号、字体写入 CSS 自定义属性 `--reader-font-family` / `--reader-font-size`

### 测试边界

- 单测（前端 vitest 或纯 node）：进度比例夹紧、配置默认值合并、编码选择逻辑（若放在前端）；Rust 侧 `load_text` 的 UTF-8/GB18030 探测单测
- 不把系统对话框、全局热键、真实透明像素列入自动化；验收以手动走查清单为准（见 Tasks）

## [S3] Out of Scope

- 目录、书签、搜索、分页模式
- 行距、颜色主题、背景透明度、字重等完整排版设置
- PDF 版式还原 / 图文混排（仅抽文本重排）
- EPUB 图片、复杂 CSS、脚本
- 老板键伪装、多书架管理、云同步
- Windows 专项打包（代码保持 Tauri 可移植，但本阶段只保证 macOS 可构建运行）

## Tasks

- [x] T1: 脚手架 — Vite + TS + Tauri 2 工程可 `npm run build` 与 `cargo check` 通过（covers: S2）
- [x] T2: TXT 加载与编码 — `load_text` 读文件，UTF-8/GB18030 可读，错误路径返回明确信息（covers: S2）
- [x] T3: 阅读渲染与字体字号 — 连续滚动正文；字体族/字号可改并立即生效、写入配置（covers: S2）
- [x] T4: 透明窗与快速隐蔽 — 透明置顶无边框；`CmdOrCtrl+Shift+H` 切换；失焦隐藏；对话框期间不误隐藏（covers: S2）
- [x] T5: 进度与最近文件 — 按文件记滚动比例并恢复；最近 10 条可打开（covers: S2）
- [ ] T6: 手动走查 — 打开 GBK/UTF-8 样书、改字号字体、热键隐藏恢复、失焦隐藏、进度恢复均符合预期（covers: S2; depends: T1, T2, T3, T4, T5）
  - 已覆盖：样书编码解码、配置 roundtrip、构建与启动冒烟、独立代码审查（含隐蔽与字体路径）。完整 GUI 交互（真人热键/失焦）待本机点验。
