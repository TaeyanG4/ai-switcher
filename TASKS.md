# AI Switcher — Implementation Tasks & Acceptance Criteria (TASKS.md)

**Target OS:** Windows 10 / Windows 11  
**Execution Mode:** Sequential Phase Execution  

---

## Phase 0 — Investigation and Specification

- [x] **0.1 Environment & Tooling Audit**
  - Verify installed runtimes: Node.js (v24.19.0), npm (11.17.0), pnpm (9.15.9), VS Build Tools 2022.
  - Verify missing components: Rust toolchain (`rustup`/`cargo`).
- [x] **0.2 Platform Isolation Verification**
  - Verify Codex isolation: `CODEX_HOME` redirects configuration, SQLite databases, and login sessions.
  - Verify Claude Code isolation: `CLAUDE_CONFIG_DIR` redirects `.credentials.json` and project histories.
  - Verify Antigravity isolation: Electron `--user-data-dir` and `%USERPROFILE%` sandbox isolation.
- [x] **0.3 Interactive Alignment via /grill-me**
  - Align on execution surface: GUI preferred for Codex & Antigravity; External Terminal for Claude Code.
  - Align on process lifecycle: Cross-platform independence; prompt confirmation for single-instance conflicts.
  - Align on browser isolation: External Chromium browser (`--user-data-dir`).
  - Align on account lifecycle: Add, Authenticate, Open, Rename, Disable/Re-enable, Re-login, Logout, Delete (2-tier).
  - Align on workspace launch UX: Quick launch with optional folder picker in MVP.
- [x] **0.4 Core Specification & Architectural Documents**
  - Deliver `SPEC.md`
  - Deliver `ARCHITECTURE.md`
  - Deliver `TASKS.md`
  - Deliver Phase 0 Feasibility Report

**Phase 0 Acceptance Criteria:**
- Complete discovery report approved before initiating Phase 1.

---

## Phase 1 — Application Shell

- [x] **1.1 Toolchain Setup**
  - Install Rust via `winget install Rustlang.Rustup` or official `rustup-init.exe`.
  - Verify `cargo --version` and `rustc --version` with MSVC toolchain.
- [x] **1.2 Project Initialization**
  - Initialize Tauri 2 project with React 19 + TypeScript + Vite.
  - Configure `tauri.conf.json` with appropriate Windows permissions and capabilities.
- [x] **1.3 Backend Foundation**
  - Implement SQLite persistence layer (`rusqlite`) with schema migrations (`accounts`, `workspaces`, `settings`).
  - Implement domain models (`AccountProfile`, `AccountStatus`, `PlatformType`).
  - Implement typed error handling (`AppError`) with serde serialization.
  - Define `PlatformAdapter` trait with placeholder adapters (`CodexAdapter`, `ClaudeAdapter`, `AntigravityAdapter`).
- [x] **1.4 Frontend Shell**
  - Implement compact, developer-oriented UI layout (Platform sections, account cards, action buttons).
  - Implement Tauri IPC client wrappers with full TypeScript typing.
  - Implement dummy profile CRUD (Add dummy account, list accounts, toggle disabled, delete).

**Phase 1 Acceptance Criteria:**
- `cargo tauri dev` builds and launches cleanly without warnings. (Verified: `pnpm build` and `cargo build` pass with 0 errors)
- SQLite database initializes automatically on startup. (Verified: Unit & Integration tests pass)
- Account list correctly displays grouped platforms, statuses, and handles CRUD operations on mock profiles. (Verified)


---

## Phase 2 — Generic Launcher Engine

- [x] **2.1 Command Builder & Process Execution**
  - Implement `LauncherEngine` supporting structured process spawning via `std::process::Command`.
  - Implement environment variable injection per spawned process.
  - Support working directory configuration and path canonicalization.
- [x] **2.2 Terminal Spawner**
  - Implement external terminal launcher supporting Windows Terminal (`wt.exe`) with fallback to PowerShell.
- [x] **2.3 Process Manager & Concurrency Checks**
  - Implement process lifecycle tracking via PID polling and OS process table verification.
  - Implement conflict detector: detect if another profile of the same platform is running.
  - Cross-platform independence guarantee: different platforms never conflict.
  - Same-platform conflict prompt: never force-terminate without explicit confirmation.
  - Implement IPC commands: `launch_profile`, `check_launch_conflict`, `terminate_process`, `list_active_processes`.
- [x] **2.4 Automated Tests**
  - Unit tests for desktop app command construction, CLI command construction (`wt.exe` and `powershell.exe`), web command construction, environment variables, Unicode/Korean paths, cross-platform non-conflict, same-platform conflict per `InstancePolicy`, and disabled profile blocking.

**Phase 2 Acceptance Criteria:**
- LauncherEngine supports DesktopApp, Cli, and Web execution surfaces at the architectural level. (Verified)
- Native desktop application launcher and external terminal launcher implemented without raw shell concatenation. (Verified)
- ProcessManager tracks processes, detects same-platform conflicts without affecting other platforms, and requires explicit user confirmation before closing any profile. (Verified)
- All 11 unit and integration tests pass cleanly (`cargo test`). (Verified)
- Frontend TypeScript check and production build (`pnpm build`) pass cleanly. (Verified)


---

## Phase 3 — Codex Adapter (First Real Integration)

- [x] **3.1 Codex Discovery & Initialization**
  - Detect `codex.exe` and Codex Desktop application paths.
  - Implement `initialize_profile`: create isolated directory tree and default config under `%APPDATA%\AI-Switcher\profiles\codex\<id>`.
- [x] **3.2 Codex Status Probing**
  - Implement `check_status`: execute `codex login status` with `CODEX_HOME` and return `Ready` or `LoginRequired`.
- [x] **3.3 Codex Desktop & CLI Launching**
  - Implement `build_launch_spec` for Codex Desktop (`codex app [PATH]`) and CLI (`codex [PROMPT]`).
  - Connect folder picker to launch Codex Desktop targeting selected workspace root.
- [x] **3.4 Codex Auth & Logout**
  - Implement login flow trigger and clean logout via `codex logout`.
- [x] **3.5 Integration Verification**
  - Test two independent Codex profiles; verify authentication sessions remain isolated in respective `CODEX_HOME` directories.

**Phase 3 Acceptance Criteria:**
- Two distinct Codex profiles can be launched independently. (Verified)
- Authenticating Profile A does not overwrite Profile B's tokens or settings. (Verified)
- Process conflict dialog correctly warns when switching Codex Desktop instances. (Verified)

---

## Phase 4 — Browser Profile Engine

- [x] **4.1 Browser Discovery**
  - Detect installed Chromium browsers (Chrome, Edge, Brave) via standard Windows installation directories and PATH (`where.exe`).
  - Automatically detect Chromium version via adjacent versioned directory (`extract_chromium_version`).
- [x] **4.2 Isolated Profile Management & Safety**
  - Implement browser profile persistence under `%APPDATA%\AI-Switcher\browser-profiles\<id>`.
  - Enforce directory safety: reject path traversal (`..`), forbid targeting default personal browser data directories (`Google\Chrome\User Data`).
  - Implement reference-safe deletion: reject deletion if referenced by active account profiles.
  - Construct pure structured browser launch commands with `--user-data-dir`, `--no-first-run`, and `--no-default-browser-check`.
- [x] **4.3 Launch & Diagnostics Security**
  - Prevent CLI flag injection from target URLs (reject URLs starting with `-` or `/`).
  - Implement `redact_url_for_diagnostics` to mask OAuth authorization codes, session tokens, and CSRF states in logs (`code=[REDACTED]`, `state=[REDACTED]`).
- [x] **4.4 UI & Integration Tests**
  - Add `BrowserProfilesModal` for creating, launching, and managing browser profiles.
  - Integrate browser profile selector in `AddAccountModal` and diagnostics in `DiagnosticsModal`.
  - Connect `ExecutionSurface::Web` in `launch_profile` to use the associated isolated browser profile.
  - 8 automated integration tests in `browser_profile_tests.rs` verifying discovery, arguments, path traversal rejection, reference-safe deletion, directory isolation, and URL redaction.

**Phase 4 Acceptance Criteria:**
- Launching an isolated browser opens a fresh profile completely independent of the user's primary desktop browser. (Verified via Chrome, Edge, and Brave with `--user-data-dir`)
- Multiple browser profiles can run concurrently with independent cookies and LocalStorage. (Verified)
- Directory safety prevents traversal and reference safety prevents dangling pointers. (Verified)

---

## Phase 5 — Claude Desktop Integration (Desktop-First)

- [x] **5A.1 Claude Desktop & CLI Live Host Discovery**
  - Inspected Windows host installation: found Claude Desktop `v1.46388.2` (`AnthropicClaude\claude.exe`) and Claude Code CLI `v2.1.252` (`.local\bin\claude.exe`).
  - Inspected Windows registry for `claude://` deep link protocol registration (`HKCR\claude\shell\open\command`).
- [x] **5A.2 Claude Desktop Reverse-Engineering & Deep Link Verification**
  - Reverse-engineered `app.asar` URL handler: discovered routing for `claude://code/new?folder=<path>`.
  - Proved live deep link launch opens Claude Desktop directly into Claude Code view with target workspace root.
  - Verified percent-encoding for paths containing spaces and Korean/Unicode characters.
- [x] **5A.3 Profile & Session Isolation Verification**
  - Discovered Electron anti-tamper logic in packaged builds rejects `process.env.CLAUDE_USER_DATA_DIR`, but Electron runtime honors `--user-data-dir="<path>"`.
  - Verified `--user-data-dir` live: creates dedicated profile (`Local Storage`, `IndexedDB`, cookies, `config.json`, SafeStorage key) completely separate from `%APPDATA%\Claude`.
  - Discovered Claude Desktop single-instance lock is scoped per `userData` directory: multiple Desktop instances across different profiles run concurrently (`InstancePolicy::MultiInstance`).
- [x] **5B.1 Desktop-First `ClaudeAdapter` Implementation**
  - Implemented `detect_desktop_executable` and `detect_cli_executable`.
  - Configured `ExecutionSurface::DesktopApp` as the primary surface when Claude Desktop is installed, launching with `--user-data-dir="<profile>\desktop"` and `claude://code/new?folder=<encoded_path>`.
  - Implemented `ExecutionSurface::Cli` fallback using `CLAUDE_CONFIG_DIR="<profile>\cli"` in external terminal.
  - Implemented `ExecutionSurface::Web` fallback routing to `https://claude.ai` via isolated browser profile.
- [x] **5B.2 Status Probing & Logout**
  - Probed auth state via `claude auth status --json` with `CLAUDE_CONFIG_DIR`, parsing `{"loggedIn": bool}`.
  - Implemented graceful `logout` via CLI auth logout and profile session cleanup.
- [x] **5B.3 Frontend Integration**
  - Updated `AccountCard.tsx` for Claude: primary action is "Open Code" (Desktop Code).
  - Added multi-surface dropdown menu: "Launch Desktop Code", "Launch Claude Code CLI", "Open Claude Web", "Check Status", "Log Out".
- [x] **5B.4 Automated Integration Tests**
  - Added 9 unit/integration tests in `src-tauri/tests/claude_adapter_tests.rs`: metadata, executable discovery, deep link encoding (with spaces and Korean characters), desktop/cli/web launch specs, directory scaffolding, and status probing.
  - All 45 tests across the entire backend suite passing. Frontend builds cleanly.

**Phase 5 Acceptance Criteria:**
- Native Claude Desktop application launches directly into Claude Code with the specified workspace. (Verified live on host)
- Account profile isolation is enforced via dedicated `--user-data-dir` without touching the user's default `%APPDATA%\Claude`. (Verified live on host)
- Multiple Claude Desktop profiles can run concurrently as separate process trees. (Verified live on host)
- CLI and Web surfaces remain available as secondary/fallback targets. (Verified)
- Status probing and logout work reliably without credential tampering. (Verified)

---

## Phase 6 — Google Antigravity Integration (Desktop-First)

- [x] **6A.1 Live Antigravity Host Inspection & Reverse-Engineering**
  - Inspected Windows host installation: found `Antigravity.exe` v2.12.0 at `C:\Users\Taeyang\AppData\Local\Programs\antigravity\Antigravity.exe`.
  - Inspected process tree: Electron main process (PID 26444) spawns Go `language_server.exe`, GPU process, Network service, and renderer webview.
  - Decompiled and inspected Electron `app.asar`:
    - `paths.js`: `getAppDataDir()` resolves to `os.homedir()/.gemini/antigravity`, `getAppStoragePath()` uses `app.getPath('userData')`.
    - `main.js`: parses Chromium flags and `antigravity://` deep links; ignores positional folder args.
    - `languageServer.js`: receives `--app_data_dir antigravity` and `-gemini_dir .gemini` which resolve relative to `USERPROFILE`.
- [x] **6A.2 Live Profile & Concurrency Isolation Verification**
  - Live tested `Antigravity.exe` with `--user-data-dir="<profile>\data"` and `USERPROFILE="<profile>\home"`.
  - Confirmed 100% of Chromium storage (`Local Storage`, `IndexedDB`, cookies, `app_storage.json`) is isolated in `<profile>\data`.
  - Confirmed 100% of Gemini state (`.gemini/antigravity`, `.gemini/config`) is isolated in `<profile>\home\.gemini`, leaving system `%USERPROFILE%\.gemini` untouched.
  - Live tested concurrent execution: Profile A (PID 23220) and Profile B (PID 28164) ran simultaneously with independent process trees alongside the host instance (`InstancePolicy::MultiInstance`).
- [x] **6A.3 Workspace Launch Investigation**
  - Determined that Antigravity's Electron `main.js` does NOT support direct command-line folder/workspace arguments (bare positional arguments are ignored).
  - Workspace selection inside Antigravity is handled via GUI project switcher and `dialog:open-workspace`.
  - Verified safe behavior: launcher sets working directory (`cwd`) to workspace path and launches isolated profile.
- [x] **6B.1 Production `AntigravityAdapter` Implementation**
  - Primary execution surface: `ExecutionSurface::DesktopApp`.
  - Executable discovery across `AppData\Local\Programs\antigravity`, `ProgramFiles`, and PATH.
  - Profile scaffolding: `<profile>\home\.gemini`, `<profile>\data`, `<profile>\appdata`.
  - Desktop launch specification injecting `--user-data-dir="<profile>\data"` and `USERPROFILE`/`HOME`/`APPDATA` environment variables.
  - Safe status probing checking non-empty `<profile>\home\.gemini\oauth_creds.json` existence without reading sensitive token contents.
  - Safe logout deleting profile credential cache and session cookies.
- [x] **6B.2 Managed Directory Safety & Traversal Prevention**
  - Implemented `validate_profile_id`, `validate_antigravity_profile_dir`, and `safe_delete_antigravity_profile_dir`.
  - Explicitly rejected path traversal (`..`), slashes, user home directory, system `.gemini`, and system `AppData\Roaming\Antigravity`.
  - Added canonical directory containment check in `commands.rs::delete_account`.
- [x] **6B.3 Frontend Integration**
  - Updated `AccountCard.tsx` with primary button **"Open Antigravity"** and dropdown item **"Launch Antigravity Desktop"**.
- [x] **6B.4 Automated Integration Tests**
  - Added 9 unit/integration tests in `src-tauri/tests/antigravity_adapter_tests.rs`: metadata, executable discovery, profile scaffolding, desktop launch spec, spaces and Korean paths, profile separation, status probing, logout, directory safety & traversal prevention.
  - All 54 tests passing across the entire backend suite. Frontend builds cleanly.

**Phase 6 Acceptance Criteria:**
- Antigravity Desktop launches with fully isolated profile directories. (Verified live on host)
- Sessions, cookies, and local Gemini state persist inside the profile without modifying `%USERPROFILE%\.gemini` or `%APPDATA%\Antigravity`. (Verified live on host)
- Multiple isolated Antigravity profiles run concurrently as separate process trees (`InstancePolicy::MultiInstance`). (Verified live on host)
- Directory safety prevents traversal and accidental deletion of user directories. (Verified)
- Full test suite (54 tests) and frontend build pass without regression. (Verified)

---

## Phase 7 — Account Registration Wizard & Lifecycle UX

- [x] **7.1 Wizard UI & Multi-Step Architecture**
  - Designed and built 7-step wizard (`AccountWizardModal.tsx`):
    - Step 1: Platform Selection with dynamic, backend-driven capabilities, surface badges, and truthful Codex isolation disclaimers.
    - Step 2: Account Details (Display Name required, Account Identifier optional hint, Login Method selector, Workspace path).
    - Step 3: Browser Environment (Create dedicated isolated browser profile, link existing with sharing warnings, or system default).
    - Step 4: Automated Scaffolding with progress spinner and retry.
    - Step 5: Official Platform Authentication launch (`startLoginFlow`).
    - Step 6: Non-Intrusive Verification probe (`checkAccountStatus`).
    - Step 7: Ready / Completion card with quick launch [Open Now] and [Done].
  - Implemented safe cancellation handling:
    - Pre-scaffolding (Steps 1–3): Discard with 0 changes to DB or filesystem.
    - Post-scaffolding (Steps 4–6): Confirmation dialog allowing "Keep Profile for Later" (status: `login_required`) or "Delete Local Setup & Discard" (calls `cleanupDraftAccount`).
- [x] **7.2 Backend Capability Reporting & Command Extensions**
  - Added `PlatformCapabilities` model and `list_platform_capabilities` command in `src-tauri`.
  - Updated `create_account`: newly scaffolded accounts default to `AccountStatus::LoginRequired`.
  - Added `start_login_flow`: dynamically launches platform login (CLI terminal for Codex, Desktop app for Claude and Antigravity).
  - Added `cleanup_draft_account`: cleans up draft DB row and deletes profile directory within canonical containment bounds.
  - Extended `delete_account` with two tiers and shared browser profile reference protection: prevents deleting browser profiles linked to multiple accounts.
- [x] **7.3 Account Card & Lifecycle Polish**
  - Added visible "Log In" quick action button on `AccountCard` when status is `login_required`.
  - Added "Re-authenticate / Complete Login" to card dropdown menu.
  - Enhanced `DeleteAccountModal` with shared browser profile protection and warning badges.
- [x] **7.4 Automated Integration Tests**
  - Added 5 integration tests in `src-tauri/tests/registration_wizard_tests.rs`:
    - Platform capability truthfulness (Codex desktop isolation `false`, Claude/Antigravity `true`).
    - Account creation starting in `LoginRequired`.
    - Draft account cancellation and rollback.
    - Official login spec construction for CLI (Codex) and Desktop (Antigravity).
    - Shared browser profile deletion protection and single-reference safe cleanup.
  - All 59 tests passing in `cargo test`. `pnpm build` clean with 0 errors.

**Phase 7 Acceptance Criteria:**
- End-to-end account onboarding works cleanly for all supported platforms without manual path typing. (Verified)
- Strict non-intrusive authentication: zero credential interception or secret token storage. (Verified)
- Truthful Codex UI: explicitly informs user that Codex Desktop is single-instance and only CLI profiles are isolated. (Verified)
- Full lifecycle UX implemented: Add (Wizard), Authenticate, Open, Rename, Disable/Enable, Re-login, Logout, Two-Tier Delete with reference protection. (Verified)

---

## Phase 8 — Workspace Presets

- [x] **8.1 Workspace Management UI & DB**
  - Add `workspaces` table in SQLite schema with foreign keys referencing accounts with `ON DELETE SET NULL`.
  - Implement full backend CRUD operations (`insert_workspace`, `get_workspace`, `list_workspaces`, `update_workspace`, `delete_workspace`, `update_workspace_last_opened`, `create_workspace_directory`).
  - Implement native Windows folder browsing via `tauri-plugin-dialog` alongside manual path text entry for both Registration Wizard and Workspace Presets.
  - Implement input validation preventing empty names or invalid directory paths.
- [x] **8.2 Account & Agent Association**
  - Model and configure `preferred_codex_account_id`, `preferred_claude_account_id`, `preferred_antigravity_account_id` per preset.
  - Foreign key cascading: deleting an account profile safely resets preferred mappings to `NULL` without deleting the preset.
  - Smart account resolution: prefers configured account profile; gracefully falls back to any active account of that platform if unassigned or previously deleted.
- [x] **8.3 One-Click Workspace Launch**
  - Implement `launch_workspace_preset` Tauri command and backend impl:
    - Resolves target profile for requested platform (Codex, Claude, Antigravity).
    - Explicit missing workspace behavior: strictly avoids silent auto-creation; returns `WorkspaceDirectoryNotFound` and prompts user with `[Locate Folder]`, `[Create Folder]`, `[Cancel]`.
    - Injects project directory path into platform launch spec (Codex: `codex app [DIR]`, Claude: `--cwd`/deep-link, Antigravity: `working_directory = workspace_path` with zero positional CLI arguments).
    - Updates preset `last_opened_at` timestamp and account `last_launched_at`.
  - Frontend `WorkspacePresetsModal` with preset cards, mapping badges, quick launch buttons (`[Open Codex]`, `[Open Claude]`, `[Open Antigravity]`), and live status feedback.
- [x] **8.4 Automated Tests**
  - Added unit test in `src-tauri/src/db/mod.rs` (`test_workspace_preset_db_crud`).
  - Added 4 comprehensive integration tests in `src-tauri/tests/workspace_preset_tests.rs`:
    - `test_workspace_preset_crud_impl`: Name/path validation, creation, retrieval, updates, and deletion.
    - `test_workspace_preset_foreign_key_cascade`: Multi-agent account links and `ON DELETE SET NULL` cascade behavior upon account deletion.
    - `test_launch_workspace_preset_account_resolution_and_dir_creation`: Fallback resolution, missing directory blocking, explicit folder creation, and Locate Folder update flow.
    - `test_antigravity_workspace_launch_cwd_only`: Confirms Antigravity workspace launch sets `working_directory` and never passes positional workspace arguments.
  - All 64 tests passing in `cargo test`. `pnpm build` clean with 0 errors.

**Phase 8 Acceptance Criteria:**
- User can define a project (e.g. "Forge") and launch it directly into a specific account profile with one click. (Verified)
- Preferred accounts can be independently bound to OpenAI Codex, Anthropic Claude, and Google Antigravity. (Verified)
- Deleting an account profile never breaks or deletes existing workspace presets. (Verified)
- Missing workspace folders require explicit user action (Locate Folder / Create Folder) and are never silently recreated. (Verified)

---

## Phase 9 — Windows Integration & UX Polish

- [x] **9.1 System Tray & Close-to-Tray Subsystem**
  - Integrated native Tauri 2 System Tray (`TrayIconBuilder`, `tray-icon` feature, `tauri::menu::{Menu, MenuItem, Submenu, PredefinedMenuItem}`).
  - Built pure, headless tray menu generator (`src-tauri/src/tray/mod.rs`):
    - 🌟 **Favorites Submenu**: fast launch for starred accounts and workspace presets.
    - 🕒 **Recent Launches Submenu**: aggregated last 5 launched accounts and opened workspaces sorted by activity.
    - 🟢 **OpenAI Codex Submenu**: all enabled Codex accounts.
    - 🟣 **Anthropic Claude Submenu**: all enabled Claude accounts.
    - 🔵 **Google Antigravity Submenu**: all enabled Antigravity accounts.
    - 📁 **Workspaces Submenu**: all configured presets with "Open in Codex", "Open in Claude", "Open in Antigravity".
    - 🪟 **Open AI Switcher**, ⚙️ **Settings...**, ❌ **Quit AI Switcher**.
  - Left-click tray icon restores, unminimizes, and focuses main window; right-click displays context menu.
  - Close-to-tray behavior intercepts `WindowEvent::CloseRequested` to hide window into tray, with clean complete exit on tray "Quit" (`app.exit(0)`).
  - First-close notification dialog (`FirstCloseDialog.tsx`) informs user of background tray operation with "Don't show again" toggle.
- [x] **9.2 Single-Instance Enforcement & Windows Autostart**
  - Integrated `tauri-plugin-single-instance`: secondary launches automatically bring the existing window to the foreground and set focus.
  - Integrated `tauri-plugin-autostart`: user-configurable Windows startup launch with `--minimized` argument support.
- [x] **9.3 Global Keyboard Shortcuts**
  - Integrated `tauri-plugin-global-shortcut` with frontend registration and conflict validation.
  - Configurable hotkey (default: `CommandOrControl+Alt+S`) to toggle window visibility from anywhere in Windows.
- [x] **9.4 Favorites & Recents Engine**
  - Added `favorites` table in SQLite with foreign keys and `ON DELETE CASCADE`.
  - Added star toggle buttons to both Account Cards and Workspace Preset cards.
  - Added unified recents aggregation (`get_recent_launches`) merging account launches and workspace openings in descending timestamp order.
- [x] **9.5 Settings Modal & Theme System**
  - Created tabbed `SettingsModal.tsx` covering:
    - **General**: Windows autostart toggle, start minimized, close-to-tray, reset tray notification.
    - **Appearance**: System default, Light, and Dark themes with live preview.
    - **Shortcuts**: Configurable global hotkey input and validation.
    - **Diagnostics**: Data directory path, active process count, platform adapter capability summaries.
  - Complete CSS theme engine with variables for `.theme-light`, `[data-theme="light"]`, `.theme-dark`, `[data-theme="dark"]`.
- [x] **9.6 Automated Integration Tests**
  - Added `src-tauri/tests/tray_and_settings_tests.rs`:
    - `test_app_settings_persistence_and_defaults`: verifies default settings, serialization, and database roundtrip.
    - `test_favorites_management_and_cascades`: verifies account and workspace favorite toggles, ordering, and foreign key cascade deletion.
    - `test_recents_aggregation_and_ordering`: verifies merging of account and workspace timestamps in correct descending order.
  - All 67 backend tests pass (`cargo test`).
  - Frontend production build compiles cleanly with 0 TypeScript/Vite errors (`pnpm build`).

**Phase 9 Acceptance Criteria:**
- System tray menu displays account hierarchy, favorites, recents, and platforms for one-click launches directly from the Windows taskbar. (Verified)
- Left-click tray icon focuses the window; close button hides to tray with first-close notice. (Verified)
- Single-instance enforcement prevents duplicate windows. (Verified)
- Theme switching (System, Light, Dark) is persistent and immediate. (Verified)

---

## Phase 10 — Security, Recovery & Reliability Hardening

- [x] **10.1 Phase 9 Live Verification Gate**
  - Verified system tray icon visibility, left-click show/focus, right-click menu, close-to-tray, and quit termination.
  - Verified single-instance mutex enforcement preventing duplicate application windows.
  - Verified global shortcut registration, conflict tolerance, and autostart toggle behavior.
- [x] **10.2 Database Reliability, Backup Strategy & Schema Migrations**
  - Implemented transactional schema migrations tracked via `PRAGMA user_version` (v1 -> v4).
  - Implemented automated database backups via `VACUUM INTO` before migrations and on-demand.
  - Implemented retention policy maintaining the 5 most recent backups in `%APPDATA%\AI-Switcher\backups\`.
  - Implemented non-destructive database corruption recovery and integrity checking (`PRAGMA integrity_check(1)`).
- [x] **10.3 Filesystem Safety & Windows Reparse Point / Junction Protection**
  - Created `src-tauri/src/fs_safety.rs` with `safe_remove_dir_all`.
  - Implemented canonical containment checks against `%APPDATA%\AI-Switcher` managed base directories.
  - Hardened recursive deletion on Windows: inspects `FILE_ATTRIBUTE_REPARSE_POINT` (0x400) and `FILE_ATTRIBUTE_DIRECTORY` (0x10) to unlink junctions without descending into external target folders.
  - Verified that workspace preset deletion only deletes database metadata and never touches actual project files.
- [x] **10.4 Profile Health Model & Non-Destructive Structure Repair**
  - Introduced `ProfileHealth` enum (Healthy, AuthenticationRequired, ExecutableMissing, ProfileDirectoryMissing, ProfileCorrupted, BrowserProfileUnavailable, Locked, Unknown, Error).
  - Implemented `check_account_health` to probe directory existence, scaffolding, executable paths, and browser locks without inspecting credential tokens.
  - Implemented `repair_account_profile`: non-destructively recreates missing empty scaffolding directories and default configs; sets status to `LoginRequired` without fabricating mock credentials.
- [x] **10.5 Executable Rediscovery, Browser Locks & Process Lifecycle**
  - Automated executable rediscovery before declaring an account broken.
  - Browser profile lock detection (`SingletonLock`) prevents corrupted browser sessions without forcibly deleting active locks.
  - ProcessManager PID reuse protection: verifies OS process executable name against tracked binary to prevent stale PID association.
  - Confirmed that AI Switcher process crashes or exits do not terminate spawned AI coding environments (`CREATE_NEW_PROCESS_GROUP`).
- [x] **10.6 Diagnostics Repair Center & Sanitized Support Bundle**
  - Upgraded `DiagnosticsModal.tsx` to a comprehensive Repair Center displaying database health, schema version, backup counts, profile health badges, and one-click `[Repair Structure]` actions.
  - Added `[Export Support Bundle]` generating 100% sanitized JSON with zero tokens, cookies, passwords, or secret environment variables.
- [x] **10.7 Automated Reliability & Safety Test Suite**
  - Added `src-tauri/tests/reliability_and_safety_tests.rs` (11 tests).
  - Full automated suite passes 100% (87 tests passing, 0 failing).
  - Frontend production build compiles cleanly (`pnpm build`).

**Phase 10 Acceptance Criteria:**
- Database migrations are versioned, deterministic, and transactional with automated backup retention. (Verified)
- Junction/symlink safe deletion guarantees external user directories can never be traversed or destroyed. (Verified)
- Profile repair recreates structural scaffolding safely without fabricating credentials. (Verified)
- All 87 backend tests pass cleanly (`cargo test`). (Verified)
- Frontend production bundle compiles cleanly with 0 TypeScript/Vite errors (`pnpm build`). (Verified)

---

## Phase 11 — QA, Packaging & Release

- [x] **11.1 Test Suite Execution**
  - Rust unit & integration tests (`cargo test`): 87/87 passing with 0 failures.
  - Rust code quality gates: `cargo clippy --all-targets --all-features -- -D warnings` passing with 0 warnings; `cargo fmt -- --check` passing with 0 diffs.
  - Frontend TypeScript tests & build (`pnpm build`): 0 TypeScript errors, 0 Vite errors.
  - Windows edge cases: paths with spaces, Korean Hangul characters, long paths verified across adapters.
- [x] **11.2 Installer Production**
  - Built reproducible Windows NSIS installer (`release\AI Switcher_0.1.0_x64-setup.exe`) via Tauri 2 bundler.
  - Verified per-user installation (`currentUser`) avoiding UAC elevation requirements.
  - Verified clean silent install (`/S`), upgrade, uninstall, and reinstall workflows.
  - Verified data preservation: user configuration, SQLite database, and project directories are 100% preserved across uninstall/reinstall cycles.
- [x] **11.3 Final Documentation & Validation**
  - Generated `RELEASE_NOTES.md` and `release\SHA256SUMS.txt`.
  - Updated `README.md` with complete installation, persistence, privacy statement, and known limitations.
  - Updated `ARCHITECTURE.md` with Section 11 hardening and packaging details.
  - Completed repository secret scan (0 secrets found).

**Phase 11 Acceptance Criteria:**
- Standalone installer builds cleanly and successfully installs/uninstalls on a clean Windows machine. (Verified)
- All unit, integration, and platform tests pass 100%. (Verified: 87/87 tests green)
