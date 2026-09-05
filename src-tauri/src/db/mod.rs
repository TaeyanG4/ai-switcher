use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::error::{AppError, Result};
use crate::models::{
    AccountProfile, AccountStatus, AppSettings, AuthStatus, BrowserKind, BrowserProfile,
    FavoriteTarget, LoginMethod, PlatformType, RecentItem, RuntimeStatus, UpdateAccountInput,
    UpdateBrowserProfileInput, UpdateWorkspaceInput, WorkspacePreset,
};

#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
    db_path: PathBuf,
}

pub fn backup_database_file(db_path: &Path, conn: &Connection) -> Result<PathBuf> {
    let backup_dir = db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("backups");
    std::fs::create_dir_all(&backup_dir)?;

    // Flush WAL log into database file
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f").to_string();
    let backup_filename = format!("ai-switcher-{}.sqlite", timestamp);
    let backup_path = backup_dir.join(&backup_filename);

    let backup_path_str = backup_path
        .to_str()
        .ok_or_else(|| AppError::DatabaseError("Backup path contains invalid UTF-8".to_string()))?;

    conn.execute("VACUUM INTO ?1", params![backup_path_str])?;

    // Retention policy: keep last 5 backups
    cleanup_old_backups(&backup_dir, 5)?;

    Ok(backup_path)
}

pub fn cleanup_old_backups(backup_dir: &Path, keep_count: usize) -> Result<()> {
    if !backup_dir.exists() {
        return Ok(());
    }
    let mut backups = Vec::new();
    for entry in std::fs::read_dir(backup_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.starts_with("ai-switcher-") && file_name.ends_with(".sqlite") {
                    backups.push(path);
                }
            }
        }
    }
    backups.sort();
    if backups.len() > keep_count {
        let delete_count = backups.len() - keep_count;
        for path in backups.iter().take(delete_count) {
            let _ = std::fs::remove_file(path);
        }
    }
    Ok(())
}
pub const ACCOUNT_SELECT_COLS: &str =
    "id, platform, display_name, account_identifier, login_method, status, 
                    profile_path, browser_profile_path, browser_profile_id, custom_executable_path, 
                    launch_arguments, environment_variables, default_workspace_path, is_enabled, 
                    last_launched_at, created_at, updated_at, auth_status";

pub fn map_account_row(row: &rusqlite::Row) -> rusqlite::Result<AccountProfile> {
    let platform_str: String = row.get(1)?;
    let login_method_str: String = row.get(4)?;
    let status_str: String = row.get(5)?;
    let launch_args_json: String = row.get(10)?;
    let env_vars_json: String = row.get(11)?;
    let is_enabled_int: i32 = row.get(13)?;
    let auth_status_str: Option<String> = row.get(17).ok();

    let platform = PlatformType::from_str(&platform_str).unwrap_or(PlatformType::Codex);
    let login_method = LoginMethod::from_str(&login_method_str);
    let status = AccountStatus::from_str(&status_str);
    let auth_status = auth_status_str
        .map(|s| AuthStatus::from_str(&s))
        .unwrap_or_else(|| match status {
            AccountStatus::Ready => AuthStatus::Authenticated,
            AccountStatus::LoginRequired => AuthStatus::LoginRequired,
            _ => AuthStatus::Unknown,
        });

    let launch_arguments: Vec<String> = serde_json::from_str(&launch_args_json).unwrap_or_default();
    let environment_variables: HashMap<String, String> =
        serde_json::from_str(&env_vars_json).unwrap_or_default();

    Ok(AccountProfile {
        id: row.get(0)?,
        platform,
        display_name: row.get(2)?,
        account_identifier: row.get(3)?,
        login_method,
        status,
        auth_status,
        runtime_status: RuntimeStatus::Stopped,
        profile_path: row.get(6)?,
        browser_profile_path: row.get(7)?,
        browser_profile_id: row.get(8)?,
        custom_executable_path: row.get(9)?,
        launch_arguments,
        environment_variables,
        default_workspace_path: row.get(12)?,
        is_enabled: is_enabled_int == 1,
        last_launched_at: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

impl Db {
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db_path = path.as_ref().to_path_buf();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            ",
        )?;

        // Read current schema version
        let mut user_version: i32 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap_or(0);

        // Detect if this is an unversioned database from Phases 1-9
        if user_version == 0 {
            let has_accounts: bool = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='accounts'",
                    [],
                    |r| r.get::<_, i32>(0),
                )
                .map(|c| c > 0)
                .unwrap_or(false);

            if has_accounts {
                let has_favorites: bool = conn
                    .query_row(
                        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='favorites'",
                        [],
                        |r| r.get::<_, i32>(0),
                    )
                    .map(|c| c > 0)
                    .unwrap_or(false);

                let has_browser_profiles: bool = conn
                    .query_row(
                        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='browser_profiles'",
                        [],
                        |r| r.get::<_, i32>(0),
                    )
                    .map(|c| c > 0)
                    .unwrap_or(false);

                if has_favorites {
                    user_version = 3;
                } else if has_browser_profiles {
                    user_version = 2;
                } else {
                    user_version = 1;
                }
                conn.execute(&format!("PRAGMA user_version = {}", user_version), [])?;
            }
        }

        // Backup existing database before running new migrations
        if user_version > 0 && user_version < 5 {
            let _ = backup_database_file(&db_path, &conn);
        }

        // Migration 1: Base tables
        if user_version < 1 {
            let tx = conn.transaction()?;
            tx.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS accounts (
                    id TEXT PRIMARY KEY NOT NULL,
                    platform TEXT NOT NULL,
                    display_name TEXT NOT NULL,
                    account_identifier TEXT,
                    login_method TEXT NOT NULL,
                    status TEXT NOT NULL,
                    profile_path TEXT NOT NULL,
                    browser_profile_path TEXT,
                    custom_executable_path TEXT,
                    launch_arguments TEXT NOT NULL,
                    environment_variables TEXT NOT NULL,
                    default_workspace_path TEXT,
                    is_enabled INTEGER NOT NULL DEFAULT 1,
                    last_launched_at TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS workspaces (
                    id TEXT PRIMARY KEY NOT NULL,
                    name TEXT NOT NULL,
                    directory_path TEXT NOT NULL UNIQUE,
                    preferred_codex_account_id TEXT REFERENCES accounts(id) ON DELETE SET NULL,
                    preferred_claude_account_id TEXT REFERENCES accounts(id) ON DELETE SET NULL,
                    preferred_antigravity_account_id TEXT REFERENCES accounts(id) ON DELETE SET NULL,
                    last_opened_at TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS app_settings (
                    key TEXT PRIMARY KEY NOT NULL,
                    value TEXT NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_accounts_platform ON accounts(platform);
                CREATE INDEX IF NOT EXISTS idx_accounts_status ON accounts(status);
                ",
            )?;
            tx.execute("PRAGMA user_version = 1", [])?;
            tx.commit()?;
            user_version = 1;
        }

        // Migration 2: Browser profiles & FK column
        if user_version < 2 {
            let tx = conn.transaction()?;
            tx.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS browser_profiles (
                    id TEXT PRIMARY KEY NOT NULL,
                    display_name TEXT NOT NULL,
                    browser_kind TEXT NOT NULL,
                    custom_executable_path TEXT,
                    user_data_directory TEXT NOT NULL UNIQUE,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    last_used_at TEXT
                );
                ",
            )?;

            let has_col = {
                let mut stmt = tx.prepare("PRAGMA table_info(accounts)")?;
                let columns = stmt
                    .query_map([], |row| row.get::<_, String>(1))?
                    .filter_map(|r| r.ok())
                    .collect::<Vec<_>>();
                columns.contains(&"browser_profile_id".to_string())
            };

            if !has_col {
                tx.execute(
                    "ALTER TABLE accounts ADD COLUMN browser_profile_id TEXT REFERENCES browser_profiles(id) ON DELETE SET NULL",
                    [],
                )?;
            }
            tx.execute_batch("CREATE INDEX IF NOT EXISTS idx_accounts_browser_profile_id ON accounts(browser_profile_id);")?;
            tx.execute("PRAGMA user_version = 2", [])?;
            tx.commit()?;
            user_version = 2;
        }

        // Migration 3: Favorites engine
        if user_version < 3 {
            let tx = conn.transaction()?;
            tx.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS favorites (
                    id TEXT PRIMARY KEY NOT NULL,
                    target_type TEXT NOT NULL,
                    account_id TEXT REFERENCES accounts(id) ON DELETE CASCADE,
                    workspace_id TEXT REFERENCES workspaces(id) ON DELETE CASCADE,
                    platform TEXT,
                    sort_order INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_favorites_target_type ON favorites(target_type);
                CREATE INDEX IF NOT EXISTS idx_favorites_account_id ON favorites(account_id);
                CREATE INDEX IF NOT EXISTS idx_favorites_workspace_id ON favorites(workspace_id);
                ",
            )?;
            tx.execute("PRAGMA user_version = 3", [])?;
            tx.commit()?;
            user_version = 3;
        }

        // Migration 4: System metadata table
        if user_version < 4 {
            let tx = conn.transaction()?;
            tx.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS system_metadata (
                    key TEXT PRIMARY KEY NOT NULL,
                    value TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                ",
            )?;
            tx.execute("PRAGMA user_version = 4", [])?;
            tx.commit()?;
            user_version = 4;
        }

        // Migration 5: Separate auth_status column
        if user_version < 5 {
            let tx = conn.transaction()?;
            let has_col = {
                let mut stmt = tx.prepare("PRAGMA table_info(accounts)")?;
                let columns = stmt
                    .query_map([], |row| row.get::<_, String>(1))?
                    .filter_map(|r| r.ok())
                    .collect::<Vec<_>>();
                columns.contains(&"auth_status".to_string())
            };

            if !has_col {
                tx.execute(
                    "ALTER TABLE accounts ADD COLUMN auth_status TEXT NOT NULL DEFAULT 'unknown'",
                    [],
                )?;
                tx.execute(
                    "UPDATE accounts SET auth_status = 'authenticated' WHERE status = 'ready'",
                    [],
                )?;
                tx.execute(
                    "UPDATE accounts SET auth_status = 'login_required' WHERE status = 'login_required'",
                    [],
                )?;
                tx.execute(
                    "UPDATE accounts SET auth_status = 'login_required', status = 'login_required' WHERE status = 'running'",
                    [],
                )?;
                tx.execute(
                    "UPDATE accounts SET auth_status = 'unknown' WHERE status NOT IN ('ready', 'login_required')",
                    [],
                )?;
            }
            tx.execute_batch(
                "CREATE INDEX IF NOT EXISTS idx_accounts_auth_status ON accounts(auth_status);",
            )?;
            tx.execute("PRAGMA user_version = 5", [])?;
            tx.commit()?;
        }

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path,
        })
    }

    pub fn get_path(&self) -> &Path {
        &self.db_path
    }

    pub fn backup_database(&self) -> Result<PathBuf> {
        let conn = self.conn.lock().unwrap();
        backup_database_file(&self.db_path, &conn)
    }

    pub fn get_backup_info(&self) -> Result<(usize, Option<String>)> {
        let backup_dir = self
            .db_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backups");
        if !backup_dir.exists() {
            return Ok((0, None));
        }
        let mut backups = Vec::new();
        for entry in std::fs::read_dir(backup_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("ai-switcher-") && name.ends_with(".sqlite") {
                        backups.push(name.to_string());
                    }
                }
            }
        }
        backups.sort();
        let count = backups.len();
        let latest = backups.last().cloned();
        Ok((count, latest))
    }

    pub fn check_integrity(&self) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let result: String = conn.query_row("PRAGMA integrity_check(1)", [], |r| r.get(0))?;
        Ok(result == "ok")
    }

    pub fn get_schema_version(&self) -> Result<i32> {
        let conn = self.conn.lock().unwrap();
        let version: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        Ok(version)
    }

    pub fn get_metadata(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM system_metadata WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_metadata(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO system_metadata (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, now],
        )?;
        Ok(())
    }

    // ==========================================
    // Account Profile CRUD
    // ==========================================

    pub fn list_accounts(&self, include_disabled: bool) -> Result<Vec<AccountProfile>> {
        let conn = self.conn.lock().unwrap();
        let query = if include_disabled {
            format!(
                "SELECT {} FROM accounts ORDER BY created_at ASC",
                ACCOUNT_SELECT_COLS
            )
        } else {
            format!(
                "SELECT {} FROM accounts WHERE is_enabled = 1 ORDER BY created_at ASC",
                ACCOUNT_SELECT_COLS
            )
        };

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map([], map_account_row)?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_account(&self, id: &str) -> Result<AccountProfile> {
        let conn = self.conn.lock().unwrap();
        let query = format!("SELECT {} FROM accounts WHERE id = ?1", ACCOUNT_SELECT_COLS);

        let mut stmt = conn.prepare(&query)?;
        let mut rows = stmt.query_map(params![id], map_account_row)?;

        if let Some(res) = rows.next() {
            Ok(res?)
        } else {
            Err(AppError::NotFound(format!(
                "Account not found with ID: {}",
                id
            )))
        }
    }

    pub fn insert_account(&self, account: &AccountProfile) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let launch_args_json = serde_json::to_string(&account.launch_arguments)
            .map_err(|e| AppError::ValidationError(e.to_string()))?;
        let env_vars_json = serde_json::to_string(&account.environment_variables)
            .map_err(|e| AppError::ValidationError(e.to_string()))?;

        conn.execute(
            "INSERT INTO accounts (
                id, platform, display_name, account_identifier, login_method, status,
                profile_path, browser_profile_path, browser_profile_id, custom_executable_path, 
                launch_arguments, environment_variables, default_workspace_path, is_enabled, 
                last_launched_at, created_at, updated_at, auth_status
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                account.id,
                account.platform.as_str(),
                account.display_name,
                account.account_identifier,
                account.login_method.as_str(),
                account.status.as_str(),
                account.profile_path,
                account.browser_profile_path,
                account.browser_profile_id,
                account.custom_executable_path,
                launch_args_json,
                env_vars_json,
                account.default_workspace_path,
                if account.is_enabled { 1 } else { 0 },
                account.last_launched_at,
                account.created_at,
                account.updated_at,
                account.auth_status.as_str(),
            ],
        )?;

        Ok(())
    }

    pub fn update_account(&self, input: &UpdateAccountInput) -> Result<AccountProfile> {
        let existing = self.get_account(&input.id)?;
        let conn = self.conn.lock().unwrap();

        let display_name = input
            .display_name
            .as_ref()
            .unwrap_or(&existing.display_name);
        let account_identifier = match &input.account_identifier {
            Some(v) => Some(v.clone()),
            None => existing.account_identifier.clone(),
        };
        let default_workspace = match &input.default_workspace_path {
            Some(v) => Some(v.clone()),
            None => existing.default_workspace_path.clone(),
        };
        let custom_exec = match &input.custom_executable_path {
            Some(v) => Some(v.clone()),
            None => existing.custom_executable_path.clone(),
        };
        let browser_profile_id = match &input.browser_profile_id {
            Some(v) => Some(v.clone()),
            None => existing.browser_profile_id.clone(),
        };
        let is_enabled = input.is_enabled.unwrap_or(existing.is_enabled);
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE accounts SET 
                display_name = ?1, 
                account_identifier = ?2, 
                default_workspace_path = ?3, 
                custom_executable_path = ?4, 
                browser_profile_id = ?5,
                is_enabled = ?6, 
                updated_at = ?7 
             WHERE id = ?8",
            params![
                display_name,
                account_identifier,
                default_workspace,
                custom_exec,
                browser_profile_id,
                if is_enabled { 1 } else { 0 },
                now,
                input.id,
            ],
        )?;

        drop(conn);
        self.get_account(&input.id)
    }

    pub fn update_account_auth_status(&self, id: &str, auth_status: AuthStatus) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let status = auth_status.to_account_status();
        conn.execute(
            "UPDATE accounts SET auth_status = ?1, status = ?2, updated_at = ?3 WHERE id = ?4",
            params![auth_status.as_str(), status.as_str(), now, id],
        )?;
        Ok(())
    }

    pub fn update_account_status(&self, id: &str, status: AccountStatus) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let auth_status = match status {
            AccountStatus::Ready => Some(AuthStatus::Authenticated),
            AccountStatus::LoginRequired => Some(AuthStatus::LoginRequired),
            AccountStatus::Error => Some(AuthStatus::Error),
            _ => None,
        };
        if let Some(auth) = auth_status {
            conn.execute(
                "UPDATE accounts SET status = ?1, auth_status = ?2, updated_at = ?3 WHERE id = ?4",
                params![status.as_str(), auth.as_str(), now, id],
            )?;
        } else {
            conn.execute(
                "UPDATE accounts SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params![status.as_str(), now, id],
            )?;
        }
        Ok(())
    }

    pub fn update_last_launched(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE accounts SET last_launched_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn toggle_account_enabled(&self, id: &str, is_enabled: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE accounts SET is_enabled = ?1, updated_at = ?2 WHERE id = ?3",
            params![if is_enabled { 1 } else { 0 }, now, id],
        )?;
        Ok(())
    }

    pub fn delete_account(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM accounts WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ==========================================
    // Browser Profile CRUD & Reference Safety
    // ==========================================

    pub fn insert_browser_profile(&self, profile: &BrowserProfile) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO browser_profiles (
                id, display_name, browser_kind, custom_executable_path,
                user_data_directory, created_at, updated_at, last_used_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                profile.id,
                profile.display_name,
                profile.browser_kind.as_str(),
                profile.custom_executable_path,
                profile.user_data_directory,
                profile.created_at,
                profile.updated_at,
                profile.last_used_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_browser_profile(&self, id: &str) -> Result<BrowserProfile> {
        let conn = self.conn.lock().unwrap();
        let query = "SELECT id, display_name, browser_kind, custom_executable_path, 
                            user_data_directory, created_at, updated_at, last_used_at 
                     FROM browser_profiles WHERE id = ?1";

        let mut stmt = conn.prepare(query)?;
        let mut rows = stmt.query_map(params![id], |row| {
            let kind_str: String = row.get(2)?;
            Ok(BrowserProfile {
                id: row.get(0)?,
                display_name: row.get(1)?,
                browser_kind: BrowserKind::from_str(&kind_str),
                custom_executable_path: row.get(3)?,
                user_data_directory: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                last_used_at: row.get(7)?,
            })
        })?;

        if let Some(res) = rows.next() {
            Ok(res?)
        } else {
            Err(AppError::NotFound(format!(
                "Browser profile not found with ID: {}",
                id
            )))
        }
    }

    pub fn list_browser_profiles(&self) -> Result<Vec<BrowserProfile>> {
        let conn = self.conn.lock().unwrap();
        let query = "SELECT id, display_name, browser_kind, custom_executable_path, 
                            user_data_directory, created_at, updated_at, last_used_at 
                     FROM browser_profiles ORDER BY created_at ASC";

        let mut stmt = conn.prepare(query)?;
        let rows = stmt.query_map([], |row| {
            let kind_str: String = row.get(2)?;
            Ok(BrowserProfile {
                id: row.get(0)?,
                display_name: row.get(1)?,
                browser_kind: BrowserKind::from_str(&kind_str),
                custom_executable_path: row.get(3)?,
                user_data_directory: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                last_used_at: row.get(7)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn update_browser_profile(
        &self,
        input: &UpdateBrowserProfileInput,
    ) -> Result<BrowserProfile> {
        let existing = self.get_browser_profile(&input.id)?;
        let conn = self.conn.lock().unwrap();

        let display_name = input
            .display_name
            .as_ref()
            .unwrap_or(&existing.display_name);
        let browser_kind = input.browser_kind.unwrap_or(existing.browser_kind);
        let custom_exec = match &input.custom_executable_path {
            Some(v) => Some(v.clone()),
            None => existing.custom_executable_path.clone(),
        };
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE browser_profiles SET 
                display_name = ?1, 
                browser_kind = ?2, 
                custom_executable_path = ?3, 
                updated_at = ?4 
             WHERE id = ?5",
            params![
                display_name,
                browser_kind.as_str(),
                custom_exec,
                now,
                input.id,
            ],
        )?;

        drop(conn);
        self.get_browser_profile(&input.id)
    }

    pub fn update_browser_last_used(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE browser_profiles SET last_used_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn delete_browser_profile(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM browser_profiles WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn count_accounts_referencing_browser_profile(
        &self,
        browser_profile_id: &str,
    ) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT COUNT(*) FROM accounts WHERE browser_profile_id = ?1")?;
        let count: i64 = stmt.query_row(params![browser_profile_id], |r| r.get(0))?;
        Ok(count as usize)
    }

    pub fn list_accounts_referencing_browser_profile(
        &self,
        browser_profile_id: &str,
    ) -> Result<Vec<AccountProfile>> {
        let conn = self.conn.lock().unwrap();
        let query = format!(
            "SELECT {} FROM accounts WHERE browser_profile_id = ?1 ORDER BY created_at ASC",
            ACCOUNT_SELECT_COLS
        );

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(params![browser_profile_id], map_account_row)?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    // ==========================================
    // Workspace Presets CRUD
    // ==========================================

    pub fn insert_workspace(&self, ws: &WorkspacePreset) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO workspaces (
                id, name, directory_path,
                preferred_codex_account_id, preferred_claude_account_id, preferred_antigravity_account_id,
                last_opened_at, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                ws.id,
                ws.name,
                ws.directory_path,
                ws.preferred_codex_account_id,
                ws.preferred_claude_account_id,
                ws.preferred_antigravity_account_id,
                ws.last_opened_at,
                ws.created_at,
                ws.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_workspace(&self, id: &str) -> Result<WorkspacePreset> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, directory_path,
                    preferred_codex_account_id, preferred_claude_account_id, preferred_antigravity_account_id,
                    last_opened_at, created_at, updated_at
             FROM workspaces WHERE id = ?1",
        )?;

        let ws = stmt
            .query_row(params![id], |row| {
                Ok(WorkspacePreset {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    directory_path: row.get(2)?,
                    preferred_codex_account_id: row.get(3)?,
                    preferred_claude_account_id: row.get(4)?,
                    preferred_antigravity_account_id: row.get(5)?,
                    last_opened_at: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    AppError::NotFound(format!("Workspace preset '{}' not found", id))
                }
                other => AppError::from(other),
            })?;

        Ok(ws)
    }

    pub fn list_workspaces(&self) -> Result<Vec<WorkspacePreset>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, directory_path,
                    preferred_codex_account_id, preferred_claude_account_id, preferred_antigravity_account_id,
                    last_opened_at, created_at, updated_at
             FROM workspaces ORDER BY updated_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(WorkspacePreset {
                id: row.get(0)?,
                name: row.get(1)?,
                directory_path: row.get(2)?,
                preferred_codex_account_id: row.get(3)?,
                preferred_claude_account_id: row.get(4)?,
                preferred_antigravity_account_id: row.get(5)?,
                last_opened_at: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn update_workspace(&self, input: &UpdateWorkspaceInput) -> Result<WorkspacePreset> {
        let existing = self.get_workspace(&input.id)?;
        let now = chrono::Utc::now().to_rfc3339();

        let name = input.name.as_ref().unwrap_or(&existing.name);
        let directory_path = input
            .directory_path
            .as_ref()
            .unwrap_or(&existing.directory_path);
        let preferred_codex = input
            .preferred_codex_account_id
            .as_ref()
            .or(existing.preferred_codex_account_id.as_ref());
        let preferred_claude = input
            .preferred_claude_account_id
            .as_ref()
            .or(existing.preferred_claude_account_id.as_ref());
        let preferred_ag = input
            .preferred_antigravity_account_id
            .as_ref()
            .or(existing.preferred_antigravity_account_id.as_ref());

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE workspaces SET
                name = ?1,
                directory_path = ?2,
                preferred_codex_account_id = ?3,
                preferred_claude_account_id = ?4,
                preferred_antigravity_account_id = ?5,
                updated_at = ?6
             WHERE id = ?7",
            params![
                name,
                directory_path,
                preferred_codex,
                preferred_claude,
                preferred_ag,
                now,
                input.id,
            ],
        )?;

        drop(conn);
        self.get_workspace(&input.id)
    }

    pub fn delete_workspace(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM workspaces WHERE id = ?1", params![id])?;
        if rows == 0 {
            return Err(AppError::NotFound(format!(
                "Workspace preset '{}' not found",
                id
            )));
        }
        Ok(())
    }

    pub fn update_workspace_last_opened(&self, id: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE workspaces SET last_opened_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    // ==========================================
    // App Settings
    // ==========================================

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM app_settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_app_settings(&self) -> Result<AppSettings> {
        let default = AppSettings::default();
        let theme = self.get_setting("theme")?.unwrap_or(default.theme);
        let close_to_tray = self
            .get_setting("close_to_tray")?
            .map(|v| v == "true")
            .unwrap_or(default.close_to_tray);
        let start_minimized = self
            .get_setting("start_minimized")?
            .map(|v| v == "true")
            .unwrap_or(default.start_minimized);
        let first_close_shown = self
            .get_setting("first_close_shown")?
            .map(|v| v == "true")
            .unwrap_or(default.first_close_shown);
        let global_shortcut = self
            .get_setting("global_shortcut")?
            .unwrap_or(default.global_shortcut);
        let language = self.get_setting("language")?.unwrap_or(default.language);

        Ok(AppSettings {
            theme,
            close_to_tray,
            start_minimized,
            first_close_shown,
            global_shortcut,
            language,
        })
    }

    pub fn save_app_settings(&self, settings: &AppSettings) -> Result<()> {
        self.set_setting("theme", &settings.theme)?;
        self.set_setting(
            "close_to_tray",
            if settings.close_to_tray {
                "true"
            } else {
                "false"
            },
        )?;
        self.set_setting(
            "start_minimized",
            if settings.start_minimized {
                "true"
            } else {
                "false"
            },
        )?;
        self.set_setting(
            "first_close_shown",
            if settings.first_close_shown {
                "true"
            } else {
                "false"
            },
        )?;
        self.set_setting("global_shortcut", &settings.global_shortcut)?;
        self.set_setting("language", &settings.language)?;
        Ok(())
    }

    // ==========================================
    // Favorites
    // ==========================================

    pub fn list_favorites(&self) -> Result<Vec<FavoriteTarget>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, target_type, account_id, workspace_id, platform, sort_order, created_at
             FROM favorites ORDER BY sort_order ASC, created_at ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let plat_str: Option<String> = row.get(4)?;
            let platform = plat_str.and_then(|s| PlatformType::from_str(&s));
            Ok(FavoriteTarget {
                id: row.get(0)?,
                target_type: row.get(1)?,
                account_id: row.get(2)?,
                workspace_id: row.get(3)?,
                platform,
                sort_order: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn add_favorite(&self, fav: &FavoriteTarget) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let plat_str = fav.platform.map(|p| p.as_str().to_string());
        conn.execute(
            "INSERT INTO favorites (id, target_type, account_id, workspace_id, platform, sort_order, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                fav.id,
                fav.target_type,
                fav.account_id,
                fav.workspace_id,
                plat_str,
                fav.sort_order,
                fav.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn remove_favorite(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM favorites WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn is_favorite_account(&self, account_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT COUNT(*) FROM favorites WHERE target_type = 'account' AND account_id = ?1",
        )?;
        let count: i64 = stmt.query_row(params![account_id], |r| r.get(0))?;
        Ok(count > 0)
    }

    pub fn is_favorite_workspace(
        &self,
        workspace_id: &str,
        platform: Option<&str>,
    ) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        if let Some(plat) = platform {
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM favorites WHERE target_type = 'workspace_platform' AND workspace_id = ?1 AND platform = ?2")?;
            let count: i64 = stmt.query_row(params![workspace_id, plat], |r| r.get(0))?;
            Ok(count > 0)
        } else {
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM favorites WHERE target_type = 'workspace_platform' AND workspace_id = ?1")?;
            let count: i64 = stmt.query_row(params![workspace_id], |r| r.get(0))?;
            Ok(count > 0)
        }
    }

    pub fn toggle_favorite_account(&self, account_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id FROM favorites WHERE target_type = 'account' AND account_id = ?1",
        )?;
        let existing_id: Option<String> = stmt.query_row(params![account_id], |r| r.get(0)).ok();
        if let Some(fid) = existing_id {
            conn.execute("DELETE FROM favorites WHERE id = ?1", params![fid])?;
            Ok(false)
        } else {
            let fid = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO favorites (id, target_type, account_id, sort_order, created_at) VALUES (?1, 'account', ?2, 0, ?3)",
                params![fid, account_id, now],
            )?;
            Ok(true)
        }
    }

    pub fn toggle_favorite_workspace(
        &self,
        workspace_id: &str,
        platform: Option<&str>,
    ) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let plat_val = platform.unwrap_or("");
        let mut stmt = conn.prepare(
            "SELECT id FROM favorites WHERE target_type = 'workspace_platform' AND workspace_id = ?1 AND (platform = ?2 OR (platform IS NULL AND ?2 = ''))",
        )?;
        let existing_id: Option<String> = stmt
            .query_row(params![workspace_id, plat_val], |r| r.get(0))
            .ok();
        if let Some(fid) = existing_id {
            conn.execute("DELETE FROM favorites WHERE id = ?1", params![fid])?;
            Ok(false)
        } else {
            let fid = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            let plat_param = if plat_val.is_empty() {
                None
            } else {
                Some(plat_val)
            };
            conn.execute(
                "INSERT INTO favorites (id, target_type, workspace_id, platform, sort_order, created_at) VALUES (?1, 'workspace_platform', ?2, ?3, 0, ?4)",
                params![fid, workspace_id, plat_param, now],
            )?;
            Ok(true)
        }
    }

    // ==========================================
    // Recent Launches
    // ==========================================

    pub fn get_recent_launches(&self, limit: usize) -> Result<Vec<RecentItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt_acc = conn.prepare(
            "SELECT id, platform, display_name, account_identifier, last_launched_at
             FROM accounts
             WHERE last_launched_at IS NOT NULL AND is_enabled = 1
             ORDER BY last_launched_at DESC LIMIT ?1",
        )?;
        let acc_rows = stmt_acc.query_map(params![limit as i64], |row| {
            let plat_str: String = row.get(1)?;
            let platform = PlatformType::from_str(&plat_str);
            let ident: Option<String> = row.get(3)?;
            Ok(RecentItem {
                id: row.get(0)?,
                kind: "account".to_string(),
                title: row.get(2)?,
                subtitle: ident.unwrap_or(plat_str),
                platform,
                last_used_at: row.get(4)?,
            })
        })?;

        let mut items = Vec::new();
        for r in acc_rows {
            items.push(r?);
        }

        let mut stmt_ws = conn.prepare(
            "SELECT id, name, directory_path, last_opened_at
             FROM workspaces
             WHERE last_opened_at IS NOT NULL
             ORDER BY last_opened_at DESC LIMIT ?1",
        )?;
        let ws_rows = stmt_ws.query_map(params![limit as i64], |row| {
            Ok(RecentItem {
                id: row.get(0)?,
                kind: "workspace".to_string(),
                title: row.get(1)?,
                subtitle: row.get(2)?,
                platform: None,
                last_used_at: row.get(3)?,
            })
        })?;

        for r in ws_rows {
            items.push(r?);
        }

        items.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
        items.truncate(limit);
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AccountStatus, AuthStatus, BrowserKind, LoginMethod, PlatformType, RuntimeStatus,
    };

    #[test]
    fn test_db_crud() {
        let temp_dir =
            std::env::temp_dir().join(format!("ai_switcher_test_{}", uuid::Uuid::new_v4()));
        let db_path = temp_dir.join("test.db");
        let db = Db::init(&db_path).expect("Failed to init test db");

        let account = AccountProfile {
            id: "test-id-1".to_string(),
            platform: PlatformType::Codex,
            display_name: "Personal 1".to_string(),
            account_identifier: Some("test@example.com".to_string()),
            login_method: LoginMethod::Google,
            status: AccountStatus::Ready,
            auth_status: AuthStatus::Authenticated,
            runtime_status: RuntimeStatus::Stopped,
            profile_path: "C:\\profiles\\test-1".to_string(),
            browser_profile_path: None,
            browser_profile_id: None,
            custom_executable_path: None,
            launch_arguments: vec!["--test".to_string()],
            environment_variables: HashMap::new(),
            default_workspace_path: None,
            is_enabled: true,
            last_launched_at: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        db.insert_account(&account).expect("Insert failed");
        let fetched = db.get_account("test-id-1").expect("Get failed");
        assert_eq!(fetched.display_name, "Personal 1");
        assert_eq!(fetched.platform, PlatformType::Codex);

        // List
        let list = db.list_accounts(false).expect("List failed");
        assert_eq!(list.len(), 1);

        // Toggle disabled
        db.toggle_account_enabled("test-id-1", false)
            .expect("Toggle failed");
        let active_list = db.list_accounts(false).expect("List active failed");
        assert_eq!(active_list.len(), 0);
        let all_list = db.list_accounts(true).expect("List all failed");
        assert_eq!(all_list.len(), 1);

        // Delete
        db.delete_account("test-id-1").expect("Delete failed");
        let empty_list = db.list_accounts(true).expect("List after delete failed");
        assert_eq!(empty_list.len(), 0);

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_browser_profile_db_crud() {
        let temp_dir =
            std::env::temp_dir().join(format!("ai_switcher_test_{}", uuid::Uuid::new_v4()));
        let db_path = temp_dir.join("test.db");
        let db = Db::init(&db_path).expect("Failed to init test db");

        let b_profile = BrowserProfile {
            id: "browser-1".to_string(),
            display_name: "Google Chrome A".to_string(),
            browser_kind: BrowserKind::Chrome,
            custom_executable_path: None,
            user_data_directory: "C:\\profiles\\browser-1".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            last_used_at: None,
        };

        db.insert_browser_profile(&b_profile)
            .expect("Insert browser profile failed");
        let fetched = db.get_browser_profile("browser-1").expect("Get failed");
        assert_eq!(fetched.display_name, "Google Chrome A");
        assert_eq!(fetched.browser_kind, BrowserKind::Chrome);

        let list = db.list_browser_profiles().expect("List failed");
        assert_eq!(list.len(), 1);

        // Link account
        let account = AccountProfile {
            id: "acc-1".to_string(),
            platform: PlatformType::Claude,
            display_name: "Claude User".to_string(),
            account_identifier: None,
            login_method: LoginMethod::Google,
            status: AccountStatus::Ready,
            auth_status: AuthStatus::Authenticated,
            runtime_status: RuntimeStatus::Stopped,
            profile_path: "C:\\profiles\\claude-1".to_string(),
            browser_profile_path: None,
            browser_profile_id: Some("browser-1".to_string()),
            custom_executable_path: None,
            launch_arguments: vec![],
            environment_variables: HashMap::new(),
            default_workspace_path: None,
            is_enabled: true,
            last_launched_at: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        db.insert_account(&account).expect("Insert account failed");

        let count = db
            .count_accounts_referencing_browser_profile("browser-1")
            .expect("Count failed");
        assert_eq!(count, 1);

        let refs = db
            .list_accounts_referencing_browser_profile("browser-1")
            .expect("List refs failed");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].id, "acc-1");

        // Delete account leaves browser profile intact
        db.delete_account("acc-1").expect("Delete account failed");
        let count_after = db
            .count_accounts_referencing_browser_profile("browser-1")
            .expect("Count failed");
        assert_eq!(count_after, 0);

        let b_still_exists = db.get_browser_profile("browser-1");
        assert!(b_still_exists.is_ok());

        // Delete browser profile
        db.delete_browser_profile("browser-1")
            .expect("Delete failed");
        assert!(db.get_browser_profile("browser-1").is_err());

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_workspace_preset_db_crud() {
        let temp_dir =
            std::env::temp_dir().join(format!("ai_switcher_ws_test_{}", uuid::Uuid::new_v4()));
        let db_path = temp_dir.join("test.db");
        let db = Db::init(&db_path).expect("Failed to init test db");

        let ws = WorkspacePreset {
            id: "ws-1".to_string(),
            name: "Forge Engine".to_string(),
            directory_path: "C:\\Projects\\Forge".to_string(),
            preferred_codex_account_id: None,
            preferred_claude_account_id: None,
            preferred_antigravity_account_id: None,
            last_opened_at: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        db.insert_workspace(&ws).expect("Insert workspace failed");
        let fetched = db.get_workspace("ws-1").expect("Get workspace failed");
        assert_eq!(fetched.name, "Forge Engine");
        assert_eq!(fetched.directory_path, "C:\\Projects\\Forge");

        let list = db.list_workspaces().expect("List workspaces failed");
        assert_eq!(list.len(), 1);

        // Insert an account for FK constraint
        let codex_acc = AccountProfile {
            id: "acc-codex-1".to_string(),
            platform: PlatformType::Codex,
            display_name: "Codex Pro".to_string(),
            account_identifier: None,
            login_method: LoginMethod::Google,
            status: AccountStatus::Ready,
            auth_status: AuthStatus::Authenticated,
            runtime_status: RuntimeStatus::Stopped,
            profile_path: "C:\\profiles\\codex-1".to_string(),
            browser_profile_path: None,
            browser_profile_id: None,
            custom_executable_path: None,
            launch_arguments: vec![],
            environment_variables: HashMap::new(),
            default_workspace_path: None,
            is_enabled: true,
            last_launched_at: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        db.insert_account(&codex_acc)
            .expect("Insert account failed");

        // Update
        let update_input = UpdateWorkspaceInput {
            id: "ws-1".to_string(),
            name: Some("Forge Engine v2".to_string()),
            directory_path: None,
            preferred_codex_account_id: Some("acc-codex-1".to_string()),
            preferred_claude_account_id: None,
            preferred_antigravity_account_id: None,
        };
        let updated = db.update_workspace(&update_input).expect("Update failed");
        assert_eq!(updated.name, "Forge Engine v2");
        assert_eq!(
            updated.preferred_codex_account_id,
            Some("acc-codex-1".to_string())
        );

        // Verify ON DELETE SET NULL
        db.delete_account("acc-codex-1")
            .expect("Delete account failed");
        let after_acc_deleted = db.get_workspace("ws-1").expect("Get failed");
        assert_eq!(after_acc_deleted.preferred_codex_account_id, None);

        // Update last opened
        db.update_workspace_last_opened("ws-1")
            .expect("Update last opened failed");
        let after_opened = db.get_workspace("ws-1").expect("Get failed");
        assert!(after_opened.last_opened_at.is_some());

        // Delete
        db.delete_workspace("ws-1")
            .expect("Delete workspace failed");
        assert!(db.get_workspace("ws-1").is_err());

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_app_settings_crud() {
        let temp_dir = std::env::temp_dir().join(format!(
            "ai_switcher_test_settings_{}",
            uuid::Uuid::new_v4()
        ));
        let db_path = temp_dir.join("test.db");
        let db = Db::init(&db_path).expect("Failed to init test db");

        // Defaults
        let default_settings = db.get_app_settings().expect("Get default settings failed");
        assert_eq!(default_settings.theme, "system");
        assert!(default_settings.close_to_tray);
        assert!(!default_settings.start_minimized);
        assert!(!default_settings.first_close_shown);
        assert_eq!(default_settings.global_shortcut, "CommandOrControl+Alt+S");
        assert_eq!(default_settings.language, "en");

        // Custom save
        let custom = AppSettings {
            theme: "dark".to_string(),
            close_to_tray: false,
            start_minimized: true,
            first_close_shown: true,
            global_shortcut: "CommandOrControl+Shift+Space".to_string(),
            language: "ko".to_string(),
        };
        db.save_app_settings(&custom)
            .expect("Save app settings failed");

        let fetched = db.get_app_settings().expect("Get custom settings failed");
        assert_eq!(fetched.theme, "dark");
        assert!(!fetched.close_to_tray);
        assert!(fetched.start_minimized);
        assert!(fetched.first_close_shown);
        assert_eq!(fetched.global_shortcut, "CommandOrControl+Shift+Space");
        assert_eq!(fetched.language, "ko");

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_favorites_crud_and_cascade() {
        let temp_dir =
            std::env::temp_dir().join(format!("ai_switcher_test_fav_{}", uuid::Uuid::new_v4()));
        let db_path = temp_dir.join("test.db");
        let db = Db::init(&db_path).expect("Failed to init test db");

        // Insert account
        let acc = AccountProfile {
            id: "fav-acc-1".to_string(),
            platform: PlatformType::Claude,
            display_name: "Claude Fav".to_string(),
            account_identifier: None,
            login_method: LoginMethod::Google,
            status: AccountStatus::Ready,
            auth_status: AuthStatus::Authenticated,
            runtime_status: RuntimeStatus::Stopped,
            profile_path: "C:\\profiles\\claude-fav".to_string(),
            browser_profile_path: None,
            browser_profile_id: None,
            custom_executable_path: None,
            launch_arguments: vec![],
            environment_variables: HashMap::new(),
            default_workspace_path: None,
            is_enabled: true,
            last_launched_at: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        db.insert_account(&acc).expect("Insert account failed");

        // Insert workspace
        let ws = WorkspacePreset {
            id: "fav-ws-1".to_string(),
            name: "Fav WS".to_string(),
            directory_path: "H:\\fav-ws".to_string(),
            preferred_codex_account_id: None,
            preferred_claude_account_id: None,
            preferred_antigravity_account_id: None,
            last_opened_at: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        db.insert_workspace(&ws).expect("Insert workspace failed");

        // Toggle favorite account ON
        assert!(!db.is_favorite_account("fav-acc-1").unwrap());
        let toggled_on = db.toggle_favorite_account("fav-acc-1").unwrap();
        assert!(toggled_on);
        assert!(db.is_favorite_account("fav-acc-1").unwrap());

        // Toggle favorite workspace ON
        assert!(!db
            .is_favorite_workspace("fav-ws-1", Some("claude"))
            .unwrap());
        let ws_toggled_on = db
            .toggle_favorite_workspace("fav-ws-1", Some("claude"))
            .unwrap();
        assert!(ws_toggled_on);
        assert!(db
            .is_favorite_workspace("fav-ws-1", Some("claude"))
            .unwrap());

        let favs = db.list_favorites().expect("List favorites failed");
        assert_eq!(favs.len(), 2);

        // Delete account -> foreign key cascade removes favorite
        db.delete_account("fav-acc-1")
            .expect("Delete account failed");
        let favs_after = db
            .list_favorites()
            .expect("List favorites after delete failed");
        assert_eq!(favs_after.len(), 1);
        assert!(!db.is_favorite_account("fav-acc-1").unwrap());

        // Toggle favorite workspace OFF
        let ws_toggled_off = db
            .toggle_favorite_workspace("fav-ws-1", Some("claude"))
            .unwrap();
        assert!(!ws_toggled_off);
        assert!(!db
            .is_favorite_workspace("fav-ws-1", Some("claude"))
            .unwrap());

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_recent_launches_ordering() {
        let temp_dir =
            std::env::temp_dir().join(format!("ai_switcher_test_recent_{}", uuid::Uuid::new_v4()));
        let db_path = temp_dir.join("test.db");
        let db = Db::init(&db_path).expect("Failed to init test db");

        let acc1 = AccountProfile {
            id: "rec-acc-1".to_string(),
            platform: PlatformType::Antigravity,
            display_name: "Antigravity Old".to_string(),
            account_identifier: Some("old@example.com".to_string()),
            login_method: LoginMethod::Google,
            status: AccountStatus::Ready,
            auth_status: AuthStatus::Authenticated,
            runtime_status: RuntimeStatus::Stopped,
            profile_path: "C:\\profiles\\ag-old".to_string(),
            browser_profile_path: None,
            browser_profile_id: None,
            custom_executable_path: None,
            launch_arguments: vec![],
            environment_variables: HashMap::new(),
            default_workspace_path: None,
            is_enabled: true,
            last_launched_at: Some("2026-09-01T10:00:00Z".to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        db.insert_account(&acc1).expect("Insert account failed");

        let ws1 = WorkspacePreset {
            id: "rec-ws-1".to_string(),
            name: "Recent Workspace".to_string(),
            directory_path: "H:\\recent-ws".to_string(),
            preferred_codex_account_id: None,
            preferred_claude_account_id: None,
            preferred_antigravity_account_id: None,
            last_opened_at: Some("2026-09-05T09:00:00Z".to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        db.insert_workspace(&ws1).expect("Insert workspace failed");

        let recents = db.get_recent_launches(5).expect("Get recents failed");
        assert_eq!(recents.len(), 2);
        // Most recent first: workspace at 2026-09-05
        assert_eq!(recents[0].kind, "workspace");
        assert_eq!(recents[0].title, "Recent Workspace");
        assert_eq!(recents[1].kind, "account");
        assert_eq!(recents[1].title, "Antigravity Old");

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
