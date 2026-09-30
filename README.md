# 浮阅 FloatRead

[![Release](https://img.shields.io/github/v/release/clnxxx/float-read)](../../releases/latest)
[![Build](https://github.com/clnxxx/float-read/actions/workflows/release.yml/badge.svg)](../../actions/workflows/release.yml)
[![License](https://img.shields.io/badge/license-MIT-green)](./LICENSE)

一块几乎只剩文字的透明悬浮小窗，钉在屏幕角落里读小说。为「摸鱼」而生：

- **消失要快**——全局热键、点击别处、鼠标移开，三种方式瞬间藏起来
- **回来要快**——`Ctrl / ⌘ + Shift + H` 一键唤出，或鼠标移回原位自动浮现
- **足够小**——默认 260×320,可缩到一角；不上账号、不联网、无书库，数据全在本地

支持 **TXT / EPUB / PDF**(统一抽取为纯文本重排)，百万字级大文件流畅打开。

## 安装

从 [GitHub Releases](../../releases/latest) 下载：

| 平台 | 安装包 |
|------|--------|
| macOS(Apple Silicon / Intel 通用) | `FloatRead_*_universal.dmg` |
| Windows 10/11 x64 | `FloatRead_*_x64-setup.exe` |

安装包未签名：

- macOS 首次打开提示「无法验证」→ 右键 App → 打开；或执行 `xattr -cr /Applications/FloatRead.app`
- Windows 弹出 SmartScreen → 「更多信息」→「仍要运行」

## 隐蔽体系

| 方式 | 行为 |
|------|------|
| 全局热键 `Ctrl / ⌘ + Shift + H` | 任何应用里一键显示 / 隐藏(系统级注册，无需聚焦) |
| 失焦隐藏 | 点到其它应用窗口，立即消失 |
| 鼠标移出隐藏 | 鼠标离开悬浮窗约 0.2s 后自动消失——不用点击任何地方 |
| 移回唤出 | 因鼠标移出而隐藏后，鼠标移回窗口原位自动浮现 |

安全边界：**只有「鼠标移出」触发的隐藏会被移回唤出**。手动隐藏或失焦隐藏后，鼠标路过窗口原位不会把它弹出来——该藏的时候不会穿帮。托盘图标常驻，提供显示/隐藏与退出。

## 自动滚动

按 **E**(或中键点击正文)开启，匀速下滚，解放一只手：

- **W / S** 调速(10–400 px/秒，档位记住)
- 滚轮临时接管，停手 1.2s 后自动续读；惯性滚动不会误停
- 窗口隐藏时暂停，重新唤出后从原处继续
- 读到文末自动停止

## 目录与定位

- **T** 打开目录面板：自动识别「第X章 / Chapter N / 楔子 / 番外」等章节标题，点击跳转，当前章节高亮
- 每本书记住阅读位置，下次打开自动恢复
- 窗口位置和尺寸也会记住——拖到哪、缩成什么样，下次启动原样出现

## 快捷键

| 按键 | 功能 |
|------|------|
| `Ctrl / ⌘ + Shift + H` | 全局显示 / 隐藏 |
| `W` / `S` | 滚动(自动滚动时改为调速 − / +) |
| `A` / `D`、`PageUp` / `PageDown` | 按页翻动 |
| `空格` | 下翻一页 |
| `Home` / `End` | 文首 / 文末 |
| `E` | 自动滚动 开 / 关 |
| `T` | 目录面板 |
| `Esc` | 关面板；无面板时隐藏窗口 |
| 鼠标中键点正文 | 自动滚动 开 / 关 |

鼠标：按住正文拖动 = 移动窗口；边缘四角 = 调整大小；滚轮 = 滚动。

## 格式与个性化

- **TXT**:UTF-8 / GB18030(GBK)/ UTF-16(带或不带 BOM)自动识别
- **EPUB**:按 spine 顺序抽取正文；**PDF**:内置文本抽取
- 大文件：分块虚拟渲染 + 占位高度实测校准，数百万字 TXT 打开、滚动、跳目录不卡
- 个性化：7 种预设字体、12–48px 字号、黑 / 白字色——鼠标移到窗口右上角浮现工具按钮

配置保存在系统配置目录(`…/com.floatread.app/float-read.json`),原子写入，损坏时自动备份为 `.bak` 再回默认，进度不会静默丢失。

## 开发

要求：Node 20+、Rust stable。

```bash
npm install
npm test              # 前端测试(vitest)
cargo test            # Rust 测试(在 src-tauri/ 下)
npm run tauri dev     # 开发调试
npm run tauri build   # 本机安装包
```

技术栈:[Tauri 2](https://tauri.app)(Rust 后端 + 系统 WebView)+ TypeScript + Vite,无前端框架。文本解码基于 [encoding_rs],EPUB 解包基于 [zip],PDF 抽取基于 [pdf-extract]。

## License

[MIT](./LICENSE) · 第三方组件见 [THIRD_PARTY.md](./THIRD_PARTY.md)
