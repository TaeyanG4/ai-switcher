# AI Switcher v0.1.0 — Release Notes

**AI Switcher** is a native Windows desktop profile launcher and environment switcher for multi-account AI coding workflows across OpenAI Codex, Anthropic Claude, and Google Antigravity.

---

## 🌟 Highlights

- **Multi-Account Claude Desktop Isolation:** Launches native Claude Desktop directly into Claude Code (`claude://code/new?folder=...`) using isolated `--user-data-dir` profiles. Supports multiple concurrent desktop instances.
- **Multi-Account Antigravity Desktop Isolation:** Launches native Antigravity Desktop with independent Chromium and `.gemini` sandboxing via custom `--user-data-dir`, `USERPROFILE`, and `APPDATA` overrides. Supports concurrent desktop instances.
- **Codex CLI Isolated Profiles:** Full account profile isolation via dedicated `CODEX_HOME` directories. Native Codex Desktop workspace launching is supported.
- **Workspace Presets:** Save repositories with platform account bindings. Features native Windows directory browsing and missing-folder protection (`Locate Folder`, `Create Folder`, `Cancel`). Deleting presets preserves project source code.
- **Windows System Tray Integration:** Lives in the notification area with dynamic quick-launch submenus for Favorites, Recent Launches, Platform Profiles, and Workspaces.
- **Favorites & Recent Launches:** Star preferred accounts and workspace presets; access recently launched environments in descending chronological order.
- **Global Keyboard Shortcut:** Summon and hide the main window with `CommandOrControl+Alt+S` (configurable with conflict detection).
- **Diagnostics & Self-Healing:** Integrated Diagnostics & Repair Center providing transactional database migrations (`PRAGMA user_version = 4`), automated `VACUUM INTO` backups (5-snapshot retention), reparse-point safe directory deletions, and non-destructive profile scaffolding repair.

---

## ⚠️ Known Limitations

- **Codex Desktop Account Isolation:** Codex Desktop is currently single-instance on Windows; MSIX protocol activation does not reliably propagate custom caller environment variables (`CODEX_HOME`). Desktop workspace launch is verified, but multi-account isolation is supported only via Codex CLI.
- **Antigravity CWD-Only Workspace Launch:** Antigravity Desktop does not accept positional command-line workspace path arguments. Workspaces are launched by setting the working directory (`cwd`) to the target workspace.
- **External Service Authentication:** Session validity depends on upstream OpenAI, Anthropic, and Google authentication policies. Expired sessions prompt the user to re-authenticate via the platform's official login interface.
- **Architecture:** Compiled and verified for 64-bit Windows (`x86_64-pc-windows-msvc`). ARM64 Windows is currently untested and unsupported.

---

## 🛡️ Security & Privacy

- **No Credential Scraping:** AI Switcher never requests, scrapes, or stores passwords, tokens, or OTPs. Authentication secrets remain managed by the target applications, OS DPAPI, and browser profiles.
- **Reparse Point Protection:** Profile deletion unlinks Windows directory junctions without recursing into target folders, preventing accidental deletion of external user directories.
- **Sanitized Diagnostics:** Support bundles and logs automatically redact OAuth codes, bearer tokens, API keys, and session cookies.
- **Local Persistence Only:** All configuration and profile metadata are stored locally in `%APPDATA%\AI-Switcher`. AI Switcher never uploads telemetry or credentials.
