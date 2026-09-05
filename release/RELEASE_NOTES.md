# AI Switcher — Release Notes

## v0.2.1 — Release Notes

> [!NOTE]
> **Desktop-First Authentication Stabilization & Codex Fast Sequential Switching**
> This release stabilizes desktop authentication routing, enforces Desktop-First as the primary product experience across all platforms, hardens protocol interception, and introduces verified Fast Sequential Desktop Account Switching for OpenAI Codex.

### 🌟 Highlights
- **Desktop-First Experience Restored as Primary:**
  - **OpenAI Codex:** Primary card action is `[Open Codex Desktop]`. Displays a prominent `⚠️ Shared Windows Desktop session` transparency indicator. Fast sequential switching transitions between accounts via dedicated browser profiles in 2 clicks (~5.2s). CLI launch available in card overflow menu (`[⋮]`).
  - **Anthropic Claude:** Primary card action is `[Open Claude Desktop]` with deep-link navigation into Claude Code. Supports multi-instance concurrent execution.
  - **Google Antigravity:** Primary card action is `[Open Antigravity Desktop]` with isolated runtime environment. Supports multi-instance concurrent execution.
  - Windows System Tray, Favorites, Recent Launches, and Workspace Presets default to Desktop execution surfaces.
- **Codex Fast Sequential Desktop Account Switching:**
  - Fast, assisted sequential account switching for OpenAI Codex Desktop on Windows without multi-instance collision.
  - Graceful process-tree termination for background/minimized-to-tray `ChatGPT.exe` instances.
  - Automated OAuth URL relay (<0.10s capture) to dedicated Chromium browser profiles.
  - 1-click browser authorization redirects to official localhost loopback (`127.0.0.1:1455`), writing credentials natively via official `codex.exe`.
  - Zero-secret compliance: strictly adheres to zero copying of `auth.json`, tokens, or cookies.
- **Surface-Aware Concurrency Policy:**
  - `instance_policy_for(surface)` distinguishes execution surfaces: Codex Desktop is `SingleInstance` while Codex CLI supports `MultiInstance` across distinct `CODEX_HOME` profiles.
- **Protocol Broker Transactional Hardening:**
  - `ProtocolRegistryBackend` trait abstraction enables complete test isolation via `MockRegistryBackend`, ensuring zero real HKCU mutations during `cargo test`.
  - **Ownership Safety:** Only restores protocol handlers if current registry values match AI Switcher's broker marker. External handler updates are preserved.
  - **Auth Callback Filtering:** Deep link interception is strictly restricted to auth callback URLs (`antigravity://auth/...`). General application links bypass directly.
  - **Single Active Flow Guard:** Disallows concurrent login flows for the same platform, preventing callback token mismatch.
  - **Resilient State Persistence:** Atomic temp-file-and-rename writes, mutex synchronization, and 15-minute expiration timeouts.
- **Surface-Aware Authentication Model & Migration v6:**
  - Adds `account_auth_states` table with composite key `(account_id, surface)` for fine-grained per-surface session tracking.
  - Adds `last_launched_surface` to `accounts` table.
  - Resets false-positive legacy ready states for Claude and Antigravity to `Unknown`.
  - Decouples `AuthStatus` as the persistent truth, keeping `RuntimeStatus` in-memory.
- **Zero Host Side-Effect Testing:**
  - Full automated suite passes 100% with zero host registry or app data modifications.

---

## v0.2.0 — Release Notes

## 🌟 Highlights

- **Authentication & Runtime Decoupling:** Persistent authentication state (`AuthStatus`: Authenticated, LoginRequired, Pending, Unknown, Error) is fully decoupled from in-memory process execution (`RuntimeStatus`: Stopped, Running, Unknown). Launching an application never falsely turns authentication to ready.
- **Protocol Callback Broker:** Headless CLI broker handles OAuth deep-link redirects (`antigravity://`, `claude://`) within < 10ms, routing authentication tokens directly into the isolated profile rather than dropping them into the host's global profile.
- **Truthful Login Flow Semantics:** Launching an official login process displays "Official sign-in opened. Complete sign-in, then verify" instead of misrepresenting process launch as completed authentication.
- **Accurate Surface Verification:**
  - **Codex:** Primary isolated surface is `[Open Codex CLI]`. Status verified via `codex login status` under isolated `CODEX_HOME`. Desktop surface labeled `Open Codex Desktop (Shared Session)` with explicit confirmation.
  - **Claude Desktop:** Verified specifically by checking for `sessionKey` in `<profile>/desktop/Network/Cookies`. Anonymous Cloudflare cookies are rejected.
  - **Antigravity:** Verified by probing `<profile>/home/.gemini/oauth_creds.json` existence and size.
- **Multi-Account Claude Desktop Isolation:** Launches native Claude Desktop directly into Claude Code (`claude://code/new?folder=...`) using isolated `--user-data-dir` profiles. Supports multiple concurrent desktop instances.
- **Multi-Account Antigravity Desktop Isolation:** Launches native Antigravity Desktop with independent Chromium and `.gemini` sandboxing via custom `--user-data-dir`, `USERPROFILE`, and `APPDATA` overrides. Supports concurrent desktop instances.
- **Workspace Presets:** Save repositories with platform account bindings. Features native Windows directory browsing and missing-folder protection (`Locate Folder`, `Create Folder`, `Cancel`). Deleting presets preserves project source code.
- **Windows System Tray Integration:** Lives in the notification area with dynamic quick-launch submenus for Favorites, Recent Launches, Platform Profiles, and Workspaces.
- **Favorites & Recent Launches:** Star preferred accounts and workspace presets; access recently launched environments in descending chronological order.
- **Global Keyboard Shortcut:** Summon and hide the main window with `CommandOrControl+Alt+S` (configurable with conflict detection).
- **Diagnostics & Self-Healing:** Integrated Diagnostics & Repair Center providing transactional database migrations (`PRAGMA user_version = 5`), automated `VACUUM INTO` backups (5-snapshot retention), reparse-point safe directory deletions, and non-destructive profile scaffolding repair.

---

## ⚠️ Known Limitations & Verified Realities

- **Codex Desktop Shared Session:** Codex Desktop on Windows is a single-instance MSIX application that shares global Windows state and ignores ephemeral `CODEX_HOME`. Codex account isolation is supported and verified strictly on the **Codex CLI** surface.
- **Antigravity CWD-Only Workspace Launch:** Antigravity Desktop does not accept positional command-line workspace path arguments. Workspaces are launched by setting the working directory (`cwd`) to the target workspace.
- **External Service Authentication:** Session validity depends on upstream OpenAI, Anthropic, and Google authentication policies. Expired sessions prompt the user to re-authenticate via the platform's official login interface.
- **Architecture:** Compiled and verified for 64-bit Windows (`x86_64-pc-windows-msvc`). ARM64 Windows is currently untested and unsupported.

---

## 🛡️ Security & Privacy

- **Strict Security Prohibition:** AI Switcher never reads, dumps, prints, or copies credentials, tokens, or cookies between profiles. Zero password scraping, zero MFA automation.
- **No Credential Scraping:** AI Switcher never requests, scrapes, or stores passwords, tokens, or OTPs. Authentication secrets remain managed by the target applications, OS DPAPI, and browser profiles.
- **Reparse Point Protection:** Profile deletion unlinks Windows directory junctions without recursing into target folders, preventing accidental deletion of external user directories.
- **Sanitized Diagnostics:** Support bundles and logs automatically redact OAuth codes, bearer tokens, API keys, and session cookies.
- **Local Persistence Only:** All configuration and profile metadata are stored locally in `%APPDATA%\AI-Switcher`. AI Switcher never uploads telemetry or credentials.

