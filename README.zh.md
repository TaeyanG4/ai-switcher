# AI Switcher (简体中文)

<p align="center">
  <strong>专为 OpenAI Codex、Anthropic Claude 和 Google Antigravity 设计的 Windows 多账号与工作区管理器</strong>
</p>

<p align="center">
  <a href="README.md"><strong>English</strong></a> |
  <a href="README.ko.md"><strong>한국어</strong></a> |
  <a href="README.ja.md"><strong>日本語</strong></a> |
  <a href="README.zh.md"><strong>简体中文</strong></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Release-v0.2.0-blue.svg" alt="Release v0.2.0" />
  <img src="https://img.shields.io/badge/Platform-Windows%2010%2F11-0078D6.svg" alt="Windows 10/11" />
  <img src="https://img.shields.io/badge/Framework-Tauri%202-FFC131.svg" alt="Tauri 2" />
  <img src="https://img.shields.io/badge/Rust-Backend-dea584.svg" alt="Rust" />
  <img src="https://img.shields.io/badge/React-Frontend-61DAFB.svg" alt="React" />
</p>

---

## 📖 简介

**AI Switcher** 是一款专为需要在不同组织、团队或个人账号间无缝切换 AI 编程环境的开发者打造的 Windows 桌面日常工具。全面支持 **OpenAI Codex**、**Anthropic Claude** 和 **Google Antigravity**。

采用 **桌面端优先 (Desktop-First)** 架构设计，彻底告别账号切换时的会话覆盖、登录凭证失效以及繁琐的手动更改目录操作。用户可通过系统托盘菜单或全局快捷键一键启动完全隔离的专属开发工作区。

![AI Switcher 主仪表盘](docs/images/dashboard.png)

---

## ✨ 核心特性

### 🖥️ 桌面端优先的完全环境隔离
- **Google Antigravity Desktop:** 构建独立的 Chromium 和 `.gemini` 用户目录，通过自定义覆盖 `--user-data-dir`、`USERPROFILE` 和 `APPDATA` 确保配置完全隔离。支持基于工作目录验证 (CWD) 的安全多实例并发运行。
- **Anthropic Claude Desktop:** 基于独立的 `--user-data-dir` 配置文件，直接唤起本地 Claude Desktop 进入 **Claude Code** (`claude://code/new?folder=...`) 编程会话。支持同时多开不同账号的桌面实例。
- **OpenAI Codex Desktop & CLI:** 妥善处理 Windows 平台下的单实例桌面应用会话，并通过专属的 `CODEX_HOME` 隔离目录提供完全独立的多实例命令行 (CLI) 终端环境。

### 📁 工作区 (Workspace) 预设
- 保存常用项目目录与推荐的各平台账号关联，实现一键启动（`以 Codex 打开`、`以 Claude 打开`、`以 Antigravity 打开`）。
- 具备目录保护检测机制，若关联的文件夹移动或遗失，系统将弹出引导对话框协助重新定位或创建，绝不会损坏您的原始代码工程。

### 🌐 多语言界面
- 原生内置 **简体中文**、**English**、**한국어**、**日本語** 4种语言。
- 点击切换即可立即生效，无需重启软件，设置将自动持久化至 SQLite 数据库。

---

## 🖼️ 界面截图与功能展示

### 1. 主控制台
集中展示按平台分类的全部账号配置，实时显示运行状态徽标，支持固定收藏及一键唤起开发环境。

![主控制台](docs/images/dashboard.png)

---

### 2. 常规设置与 Windows 系统托盘集成
支持开机自启、启动时自动最小化至系统托盘、点击关闭按钮 (`X`) 最小化至托盘等贴心的 Windows 实用功能。

![常规设置](docs/images/settings-general.png)

---

### 3. 主题与界面风格
内置明亮清晰的日间模式、护眼舒适的现代深色模式，以及跟随 Windows 系统自动切换的主题模式。所有提示卡片文本均符合 WCAG AAA 高对比度（7:1以上）阅读标准。

![外观设置](docs/images/settings-appearance.png)

---

### 4. 语言设置
提供 **简体中文**、**English**、**한국어**、**日本語**，随时自由切换。

![语言设置](docs/images/settings-language.png)

---

### 5. 全局快捷键
在 Windows 任意工作界面下按下预设快捷键（默认：`CommandOrControl+Alt+S`），即可即时唤起或隐藏 AI Switcher 主窗口。

![快捷键设置](docs/images/settings-shortcuts.png)

---

## 🛡️ 可靠性与安全保障

- **100% 本地隔离存储:** 所有的账号配置、登录会话及环境数据仅保存在您的本地电脑专属目录（`%APPDATA%\AI-Switcher`）中。
- **零凭证上传保障:** AI Switcher 绝对不会拦截、记录或向任何外部服务器发送您的登录密码、API 密钥或 OAuth 凭据。
- **目录联接点 (Junction) 安全保护:** 在删除账号环境时，自动分析 Win32 重解析点元数据，仅解除目录关联，严防误删您的本地文档或原始工程源码。
- **自动 SQLite 数据快照备份:** 内置 `VACUUM INTO` 备份引擎，自动保留最近5个历史快照，并于启动时执行 `PRAGMA integrity_check` 完整性校验。
- **诊断支持包数据脱敏:** 导出问题排查日志包时，系统将自动抹除所有隐私访问令牌与关键敏感字符串。

---

## 📦 安装与下载

已编译的 Windows 安装程序可直接在 [Releases 页面](https://github.com/TaeyanG4/ai-switcher/releases/tag/v0.2.0) 下载：

1. 下载 **`AI Switcher_0.2.0_x64-setup.exe`**。
2. 双击运行安装（默认安装至 `%LOCALAPPDATA%\AI Switcher`，无需管理员权限）。
3. 从“开始菜单”或桌面快捷方式启动 **AI Switcher**。

---

## 🛠️ 源码编译与开发

### 环境要求
- Windows 10/11 64位操作系统
- [Node.js](https://nodejs.org/) (v18+) 与 [pnpm](https://pnpm.io/)
- [Rust](https://www.rust-lang.org/) (MSVC 工具链)

### 本地开发运行
```powershell
# 克隆仓库
git clone https://github.com/TaeyanG4/ai-switcher.git
cd ai-switcher

# 安装前端依赖
pnpm install

# 运行后端单元测试 (88个测试全部通过)
cargo test --manifest-path src-tauri/Cargo.toml

# 启动开发调试应用
pnpm tauri dev
```

### 构建正式安装包
```powershell
# 编译前端并打包 Windows 原生安装程序
pnpm tauri build
```
生成的安装程序位于 `src-tauri/target/release/bundle/nsis/`。

---

## 📄 开源许可证

本项目遵循 MIT License，详情请参阅 [LICENSE](LICENSE)。
