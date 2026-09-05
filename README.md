# AI Switcher

<p align="center">
  <strong>Windows Multi-Account & Workspace Manager for OpenAI Codex, Anthropic Claude, and Google Antigravity</strong>
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

## 📖 Overview

**AI Switcher** is an everyday Windows desktop utility designed for developers who work across multiple accounts, clients, organizations, and workspaces on AI coding platforms: **OpenAI Codex**, **Anthropic Claude**, and **Google Antigravity**.

Built **Desktop-First**, AI Switcher eliminates the friction of session collisions, credential overrides, and manual directory juggling by providing completely isolated environments and 1-click workspace switching directly from the desktop or system tray.

![AI Switcher Main Dashboard](docs/images/dashboard.png)

---

## ✨ Key Capabilities

### 🖥️ Desktop-First Isolated Execution
- **Google Antigravity Desktop:** Launches isolated Antigravity profiles with independent Chromium and `.gemini` user directories via custom `--user-data-dir`, `USERPROFILE`, and `APPDATA` overrides. OAuth deep links are seamlessly routed into the isolated profile via the Protocol Broker.
- **Anthropic Claude Desktop:** Launches native Claude Desktop directly into **Claude Code** (`claude://code/new?folder=...`) using isolated `--user-data-dir` profiles with active session validation (`sessionKey`). Supports multiple concurrent desktop instances.
- **OpenAI Codex Desktop & CLI:** Launches native Codex Desktop directly (`[Open Codex Desktop]`) with explicit transparency indicating that Codex Desktop operates in a single shared Windows session (`⚠️ Shared Windows Desktop session`). Fully isolated multi-account environments are supported on the CLI surface via distinct `CODEX_HOME` profile directories (`Open Codex CLI`), accessible via the card dropdown.

### 📁 Workspace Presets
- Save repositories with preferred account mappings for 1-click launching (`Open in Codex`, `Open in Claude`, `Open in Antigravity`).
- Includes safety checks against missing folders, allowing you to locate or safely create directories.

### 🌐 Multilingual Interface
- Built-in localized interfaces for **English**, **한국어 (Korean)**, **日本語 (Japanese)**, and **简体中文 (Simplified Chinese)**.
- Changes take effect instantly without restarting the application and are persisted in SQLite.

---

## 🖼️ Application Screenshots & Features

### 1. Main Dashboard
Manage all your account profiles grouped by platform, view active process statuses, pin favorites, and trigger 1-click launches.

![Main Dashboard](docs/images/dashboard.png)

---

### 2. General Settings & Windows Tray Integration
Configure AI Switcher to start automatically with Windows, start minimized, minimize to the system tray on close (`X`), and customize tray notices.

![General Settings](docs/images/settings-general.png)

---

### 3. Theme & Appearance
High-contrast daylight Light Theme, sleek Dark Theme, or System Default automatically adapting to Windows color mode. All alerts meet WCAG AAA contrast standards.

![Appearance Settings](docs/images/settings-appearance.png)

---

### 4. Language Selection
Effortlessly switch between **English**, **한국어**, **日本語**, and **简体中文**.

![Language Settings](docs/images/settings-language.png)

---

### 5. Global Hotkey & Shortcuts
Summon or hide AI Switcher from anywhere in Windows with a configurable global shortcut (default: `CommandOrControl+Alt+S`).

![Shortcuts Settings](docs/images/settings-shortcuts.png)

---

## 🛡️ Reliability & Security Guarantees

- **100% Local Storage:** Credentials, session data, and workspace configurations reside strictly in isolated directories on your local Windows PC (`%APPDATA%\AI-Switcher`).
- **Zero Token Transmission:** AI Switcher never intercepts, logs, or transmits your API keys, OAuth tokens, or login passwords.
- **Windows Junction Protection:** Safe deletion logic inspects Win32 reparse point metadata to unlink directory junctions without touching or deleting external target files.
- **Automated Database Backups:** Snapshot SQLite backups via `VACUUM INTO` with automated 5-version retention pruning and startup integrity checks (`PRAGMA integrity_check`).
- **Sanitized Support Bundle:** Generates exportable diagnostics with all sensitive tokens and query strings redacted for safe troubleshooting.

---

## 📦 Installation & Download

Pre-compiled Windows installers are available on the [Releases Page](https://github.com/TaeyanG4/ai-switcher/releases/tag/v0.2.0).

1. Download **`AI Switcher_0.2.0_x64-setup.exe`**.
2. Run the installer (installs to `%LOCALAPPDATA%\AI Switcher` without requiring Administrator privileges).
3. Launch **AI Switcher** from your Start Menu or Desktop.

---

## 🛠️ Development & Building from Source

### Prerequisites
- Windows 10/11 64-bit
- [Node.js](https://nodejs.org/) (v18+) & [pnpm](https://pnpm.io/)
- [Rust](https://www.rust-lang.org/) (MSVC toolchain)

### Setup & Run
```powershell
# Clone repository
git clone https://github.com/TaeyanG4/ai-switcher.git
cd ai-switcher

# Install dependencies
pnpm install

# Run backend test suite (88 tests)
cargo test --manifest-path src-tauri/Cargo.toml

# Start development application
pnpm tauri dev
```

### Production Build
```powershell
# Build frontend assets and native Windows installer
pnpm tauri build
```
Installer outputs to `src-tauri/target/release/bundle/nsis/`.

---

## 📄 License

MIT License. See [LICENSE](LICENSE) for details.
