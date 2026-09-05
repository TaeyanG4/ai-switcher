# AI Switcher

**AI Switcher** is a native Windows desktop profile launcher and environment switcher for multi-account AI coding workflows across **OpenAI Codex**, **Anthropic Claude**, and **Google Antigravity**.

Designed **Desktop-First**, AI Switcher allows developers to manage multiple isolated corporate, personal, and project accounts across desktop AI IDEs and coding agents without session conflicts or manual directory swapping.

---

## 🌟 Key Features

### 🖥️ Desktop-First Architecture
- **OpenAI Codex:** Launches native Codex Desktop workspace sessions with single-instance enforcement and isolated CLI profiles (`CODEX_HOME`).
- **Anthropic Claude:** Launches native Claude Desktop directly into **Claude Code** (`claude://code/new?folder=...`) using isolated `--user-data-dir` profiles. Supports multiple concurrent desktop instances.
- **Google Antigravity:** Launches native Antigravity Desktop with full Chromium and `.gemini` isolation via custom `--user-data-dir`, `USERPROFILE`, and `APPDATA` overrides. Supports concurrent instances with verified CWD workspace activation.

### 📁 Workspace Presets
- Save active repositories with preferred platform account bindings.
- One-click launch (`[Open in Codex]`, `[Open in Claude]`, `[Open in Antigravity]`).
- Explicit missing directory protection (`[Locate Folder]`, `[Create Folder]`, `[Cancel]`).

### 🪟 Windows Integration & Everyday Utility
- **Native System Tray:** Lives quietly in the Windows notification area with dynamic quick-launch submenus for:
  - 🌟 **Favorites** (starred accounts and workspace presets)
  - 🕒 **Recent Launches** (last 5 active items)
  - 🟢 **Codex Profiles**, 🟣 **Claude Profiles**, 🔵 **Antigravity Profiles**
  - 📁 **Workspaces**
- **Single-Instance Enforcement:** Secondary launches automatically bring the existing window to the foreground and focus it.
- **Close-to-Tray:** Closing the window hides it in the tray by default, accompanied by a one-time explanation notice.
- **Windows Autostart:** Optionally start AI Switcher with Windows, minimizing directly to the system tray (`--minimized`).
- **Global Keyboard Shortcut:** Global hotkey (`CommandOrControl+Alt+S`) to summon or hide the main window from anywhere.
- **Theme Support:** System Default (matches Windows dark/light mode), Light, and Dark modes.

### 🛡️ Security, Recovery & Reliability Hardening
- **Transactional Schema Migrations:** Atomic, forward-only SQLite migrations using `PRAGMA user_version` (v1 -> v4) with pre-migration backups.
- **Automated SQLite Backups:** Snapshot database backups via `VACUUM INTO` with automated 5-snapshot retention policy.
- **Reparse Point & Junction-Safe Deletion:** Protects linked external user directories (e.g. `Documents`) by unlinking Windows directory junctions without recursing into target files.
- **Profile Health & Non-Destructive Repair:** Diagnoses `MissingDirectory`, `CorruptedScaffolding`, `SessionExpired`, and `Locked` profiles; restores directory scaffolding safely without touching existing configs or fabricating credentials.
- **Process PID Reuse Protection:** Verifies process image path before assuming an AI IDE is running, avoiding recycled Windows PID collisions.
- **Diagnostics & Repair Center:** Integrated UI modal to inspect DB integrity, manage database backups, repair profile structures, and export 100% sanitized support bundles (zero secrets/tokens).

---

## 🚀 Getting Started

### Prerequisites
- Windows 10/11 64-bit
- Node.js (v18+) & `pnpm`
- Rust toolchain (MSVC)

### Development Setup
```powershell
# Install frontend dependencies
pnpm install

# Run backend tests
cargo test --manifest-path src-tauri/Cargo.toml

# Run development app
pnpm tauri dev
```

### Production Build
```powershell
# Compile frontend assets
pnpm build

# Build native Windows binaries
pnpm tauri build
```

---

## 🧪 Automated Testing

AI Switcher features a comprehensive automated test suite (87 tests) covering:
- Database schema CRUD, cascading deletions, settings, and favorites.
- Process conflict detection and execution surface construction.
- Antigravity CWD-only workspace launch guarantees and isolation.
- Claude Desktop deep linking, space/Korean path percent-encoding, and session isolation.
- Codex CLI profile isolation and single-instance conflict checks.
- System tray headless menu generation and recents ordering.
- Windows reparse-point / junction-safe deletion without touching target files.
- Automated database backups (`VACUUM INTO`) and 5-file retention pruning.
- Non-destructive profile health checking and directory structure repair.
- Zero-secret support bundle sanitization and command injection defense.

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

---

## 📦 Installation & Uninstallation

### Windows Installation
1. Run `AI Switcher_0.1.0_x64-setup.exe` from the `release/` directory.
2. The installer runs in **per-user mode** (`currentUser`), installing application binaries to `%LOCALAPPDATA%\AI Switcher` without requiring administrative elevation (UAC).
3. A Start Menu shortcut is automatically created under `Start Menu > Programs > AI Switcher`.
4. *Note:* Because release builds are unsigned by default, Windows SmartScreen may display an informational prompt on first run ("Windows protected your PC"). Click **More info** -> **Run anyway**.

### Data Persistence & Uninstall Policy
- **Logical Directory Separation:** Application executable binaries are stored in `%LOCALAPPDATA%\AI Switcher`, while user configuration, SQLite databases, backups, and profile data are stored in `%APPDATA%\AI-Switcher`.
- **Safe Uninstall:** Uninstalling AI Switcher (via Windows Settings > Installed Apps or `%LOCALAPPDATA%\AI Switcher\uninstall.exe`) removes only the application binaries and shortcuts. It **never** deletes `%APPDATA%\AI-Switcher` or user project/workspace repositories. Reinstalling AI Switcher immediately rediscovers all existing profiles, favorites, and settings.

---

## 🔒 Privacy & Local Data Statement

AI Switcher stores configuration and profile metadata strictly on the local machine (`%APPDATA%\AI-Switcher`).
- **No Secret Scraping:** AI Switcher never prompts for, stores, or logs passwords, API tokens, or OTP codes.
- **Native Authentication:** Authentication credentials remain managed exclusively by the respective target applications (Anthropic Claude Desktop, Google Antigravity, and OpenAI Codex CLI) and Windows DPAPI / SafeStorage.
- **Zero Telemetry / Cloud Uploads:** AI Switcher contains no remote analytics, external tracking, or telemetry reporting.

---

## ⚠️ Known Limitations

1. **Codex Desktop Single-Instance & Profile Isolation:** Codex Desktop is currently single-instance on Windows; MSIX protocol activation does not reliably propagate custom caller environment variables (`CODEX_HOME`). Native desktop workspace launching is supported, but concurrent account profile isolation is only supported via Codex CLI.
2. **Antigravity CWD-Only Workspace Launch:** Antigravity Desktop does not accept positional command-line workspace path arguments. Workspaces are launched by setting the working directory (`cwd`) to the target workspace.
3. **Session Revocation:** External authentication sessions are subject to upstream provider expiration policies. If a session expires, AI Switcher prompts the user to re-authenticate using the platform's official login interface.
4. **Architecture:** Native 64-bit Windows (`x86_64-pc-windows-msvc`). ARM64 is untested and unsupported.

---

## 📄 License

License not yet selected.

