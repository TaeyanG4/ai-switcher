use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex, RwLock};

use crate::models::{ExecutionSurface, PendingAuthFlow, PlatformType};

// =========================================================================
// Protocol Registry Backend Abstraction (Enables Zero OS Side Effects in Tests)
// =========================================================================

pub trait ProtocolRegistryBackend: Send + Sync {
    fn read_command(&self, protocol: &str) -> Option<String>;
    fn write_command(&self, protocol: &str, command: &str) -> Result<(), String>;
}

pub struct WindowsRegistryBackend;

impl ProtocolRegistryBackend for WindowsRegistryBackend {
    fn read_command(&self, protocol: &str) -> Option<String> {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let key_path = format!(r"HKCU\Software\Classes\{}\shell\open\command", protocol);
            let output = Command::new("reg.exe")
                .args(["query", &key_path, "/ve"])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .ok()?;

            if !output.status.success() {
                return None;
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("REG_SZ") {
                    if let Some((_, val)) = line.split_once("REG_SZ") {
                        return Some(val.trim().to_string());
                    }
                }
            }
            None
        }
        #[cfg(not(windows))]
        {
            let _ = protocol;
            None
        }
    }

    fn write_command(&self, protocol: &str, command: &str) -> Result<(), String> {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let key_path = format!(r"HKCU\Software\Classes\{}\shell\open\command", protocol);
            let output = Command::new("reg.exe")
                .args(["add", &key_path, "/ve", "/d", command, "/f"])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| format!("Failed to run reg.exe: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Registry modification failed: {}", stderr));
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = (protocol, command);
            Ok(())
        }
    }
}

#[derive(Clone, Default)]
pub struct MockRegistryBackend {
    pub store: Arc<Mutex<HashMap<String, String>>>,
}

impl MockRegistryBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, protocol: &str, command: &str) {
        self.store
            .lock()
            .unwrap()
            .insert(protocol.to_string(), command.to_string());
    }

    pub fn get(&self, protocol: &str) -> Option<String> {
        self.store.lock().unwrap().get(protocol).cloned()
    }
}

impl ProtocolRegistryBackend for MockRegistryBackend {
    fn read_command(&self, protocol: &str) -> Option<String> {
        self.get(protocol)
    }

    fn write_command(&self, protocol: &str, command: &str) -> Result<(), String> {
        self.set(protocol, command);
        Ok(())
    }
}

static TEST_BACKEND: RwLock<Option<Arc<dyn ProtocolRegistryBackend>>> = RwLock::new(None);

pub fn set_test_registry_backend(backend: Arc<dyn ProtocolRegistryBackend>) {
    *TEST_BACKEND.write().unwrap() = Some(backend);
}

pub fn clear_test_registry_backend() {
    *TEST_BACKEND.write().unwrap() = None;
}

pub fn get_registry_backend() -> Arc<dyn ProtocolRegistryBackend> {
    if let Some(ref b) = *TEST_BACKEND.read().unwrap() {
        return Arc::clone(b);
    }
    Arc::new(WindowsRegistryBackend)
}

// =========================================================================
// Paths & URL Sanitization
// =========================================================================

static TEST_BASE_DIR: RwLock<Option<PathBuf>> = RwLock::new(None);

pub fn set_test_base_dir(path: PathBuf) {
    *TEST_BASE_DIR.write().unwrap() = Some(path);
}

pub fn clear_test_base_dir() {
    *TEST_BASE_DIR.write().unwrap() = None;
}

fn get_base_dir() -> PathBuf {
    if let Some(ref p) = *TEST_BASE_DIR.read().unwrap() {
        return p.clone();
    }
    if let Some(app_data) = dirs::data_dir() {
        app_data.join("AI-Switcher")
    } else {
        PathBuf::from(".")
    }
}

fn get_pending_auth_path() -> PathBuf {
    get_base_dir().join("pending_auth.json")
}

fn get_backup_path(protocol: &str) -> PathBuf {
    get_base_dir().join(format!("orig_protocol_{}.txt", protocol))
}

pub fn redact_url(url: &str) -> String {
    if let Some((base, _query)) = url.split_once('?') {
        format!("{}?[QUERY REDACTED]", base)
    } else {
        url.to_string()
    }
}

/// Checks if an incoming URL has the shape of an authentication callback.
/// Only authentication callbacks are routed to isolated profiles;
/// normal deep links (e.g. claude://code/new or workspace links) are bypassed.
pub fn is_auth_callback_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    if lower.starts_with("antigravity://") {
        lower.contains("auth") || lower.contains("callback") || lower.contains("code=")
    } else {
        false
    }
}

// =========================================================================
// Pending Authentication State (Atomic Storage & Single-Flow Enforcement)
// =========================================================================

pub fn list_pending_auth_flows() -> Vec<PendingAuthFlow> {
    let path = get_pending_auth_path();
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save_pending_auth_flows(flows: &[PendingAuthFlow]) {
    let path = get_pending_auth_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(flows) {
        let temp_path = path.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
        if std::fs::write(&temp_path, &json).is_ok() {
            let _ = std::fs::remove_file(&path);
            let _ = std::fs::rename(&temp_path, &path);
        }
    }
}

/// Registers an official pending authentication flow.
/// Enforces Section 13: ONLY ONE pending native authentication flow per platform at a time.
pub fn register_pending_auth(
    account_id: &str,
    platform: PlatformType,
    surface: ExecutionSurface,
    profile_path: &str,
    custom_executable: Option<&str>,
    helper_pid: Option<u32>,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp() as u64;
    let mut flows = list_pending_auth_flows();

    // Check if there is already an active pending flow for this platform with a different account_id
    for f in &flows {
        if f.platform == platform
            && f.account_id != account_id
            && now.saturating_sub(f.started_at) < 900
        {
            return Err(format!(
                "Another {} sign-in is already in progress.",
                platform.as_str()
            ));
        }
    }

    flows.retain(|f| now.saturating_sub(f.started_at) < 900 && f.account_id != account_id);

    flows.push(PendingAuthFlow {
        account_id: account_id.to_string(),
        platform,
        surface,
        profile_path: profile_path.to_string(),
        custom_executable: custom_executable.map(|s| s.to_string()),
        started_at: now,
        helper_pid,
    });

    save_pending_auth_flows(&flows);

    // Only Antigravity uses protocol broker for OAuth callbacks on Windows.
    // Claude Desktop authentication is purely webview-internal and does not use claude://.
    if platform == PlatformType::Antigravity {
        setup_protocol_broker("antigravity")?;
    }

    Ok(())
}

pub fn cleanup_pending_auth(account_id: Option<&str>, platform: Option<PlatformType>) {
    let mut flows = list_pending_auth_flows();
    let before_count = flows.len();

    if let Some(aid) = account_id {
        flows.retain(|f| f.account_id != aid);
    } else if let Some(p) = platform {
        flows.retain(|f| f.platform != p);
    } else {
        flows.clear();
    }

    if flows.len() != before_count {
        save_pending_auth_flows(&flows);
    }

    let has_antigravity = flows
        .iter()
        .any(|f| f.platform == PlatformType::Antigravity);

    if !has_antigravity {
        restore_protocol("antigravity");
    }
}

// =========================================================================
// Transactional Protocol Broker Registration & Ownership Safety
// =========================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerRegistration {
    pub protocol: String,
    pub original_command: Option<String>,
    pub broker_command: String,
    pub registered_at: u64,
}

/// Transactionally points the protocol handler to AI Switcher broker.
/// Returns Result<BrokerRegistration, String> — registry write failure MUST propagate.
pub fn setup_protocol_broker(protocol: &str) -> Result<BrokerRegistration, String> {
    let backend = get_registry_backend();
    let current_exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => return Err(format!("Failed to determine current executable: {}", e)),
    };

    let broker_cmd = format!(
        "\"{}\" --broker-protocol {} \"%1\"",
        current_exe.to_string_lossy(),
        protocol
    );

    let current_cmd = backend.read_command(protocol);
    let orig_cmd = if let Some(orig) = current_cmd {
        if !orig.contains("--broker-protocol") {
            let backup_path = get_backup_path(protocol);
            if let Some(parent) = backup_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&backup_path, &orig);
            Some(orig)
        } else {
            let backup_path = get_backup_path(protocol);
            std::fs::read_to_string(&backup_path)
                .ok()
                .map(|s| s.trim().to_string())
        }
    } else {
        None
    };

    backend.write_command(protocol, &broker_cmd)?;

    Ok(BrokerRegistration {
        protocol: protocol.to_string(),
        original_command: orig_cmd,
        broker_command: broker_cmd,
        registered_at: chrono::Utc::now().timestamp() as u64,
    })
}

/// Restores the original protocol handler with strict ownership verification.
/// If an external application or update changed the handler, AI Switcher aborts
/// restoration and preserves the external handler.
pub fn restore_protocol(protocol: &str) {
    let backend = get_registry_backend();
    let current_cmd = backend.read_command(protocol);

    // Section 11: Ownership Safety Check
    if let Some(current) = current_cmd {
        if !current.contains("--broker-protocol") {
            eprintln!(
                "[ProtocolBroker] Ownership check: handler for '{}' modified externally (value: '{}'). Preserving external handler.",
                protocol, current
            );
            let backup_path = get_backup_path(protocol);
            let _ = std::fs::remove_file(&backup_path);
            return;
        }
    }

    let backup_path = get_backup_path(protocol);
    if backup_path.exists() {
        if let Ok(orig_cmd) = std::fs::read_to_string(&backup_path) {
            let trimmed = orig_cmd.trim();
            if !trimmed.is_empty() && !trimmed.contains("--broker-protocol") {
                let _ = backend.write_command(protocol, trimmed);
            }
        }
        let _ = std::fs::remove_file(&backup_path);
    }
}

pub fn restore_all_protocols() {
    restore_protocol("antigravity");
    // Clean up any stale claude backup if present from earlier versions
    let claude_backup = get_backup_path("claude");
    if claude_backup.exists() {
        restore_protocol("claude");
    }
}

// =========================================================================
// Dispatcher CLI Logic
// =========================================================================

pub fn handle_broker_cli(protocol: &str, url: &str) -> i32 {
    let safe_url = redact_url(url);
    eprintln!(
        "[ProtocolBroker] Received callback dispatch for '{}': {}",
        protocol, safe_url
    );

    // Section 12: Only route auth callback URLs through pending flows
    if !is_auth_callback_url(url) {
        eprintln!(
            "[ProtocolBroker] URL is not an authentication callback; bypassing to original handler: {}",
            safe_url
        );
        return fallback_spawn(protocol, url);
    }

    let now = chrono::Utc::now().timestamp() as u64;
    let flows = list_pending_auth_flows();

    let target_platform = match protocol.to_lowercase().as_str() {
        "antigravity" => Some(PlatformType::Antigravity),
        _ => None,
    };

    let matching_flow = target_platform.and_then(|p| {
        flows
            .iter()
            .find(|f| f.platform == p && now.saturating_sub(f.started_at) < 900)
    });

    if let Some(flow) = matching_flow {
        eprintln!(
            "[ProtocolBroker] Routing auth callback to active isolated profile: {}",
            flow.profile_path
        );

        let exec = if let Some(custom) = &flow.custom_executable {
            PathBuf::from(custom)
        } else {
            detect_antigravity_executable()
        };

        let profile_buf = PathBuf::from(&flow.profile_path);
        let data_dir = profile_buf.join("data");
        let home_dir = profile_buf.join("home");
        let appdata_dir = profile_buf.join("appdata");

        let mut cmd = Command::new(exec);
        cmd.arg(format!("--user-data-dir={}", data_dir.to_string_lossy()));
        cmd.arg(url);
        cmd.env("USERPROFILE", &home_dir);
        cmd.env("HOME", &home_dir);
        cmd.env("APPDATA", &appdata_dir);

        match cmd.spawn() {
            Ok(_) => {
                // Section 14: Callback successfully routed, cleanup flow & restore protocol immediately
                cleanup_pending_auth(Some(&flow.account_id), None);
                return 0;
            }
            Err(e) => {
                eprintln!(
                    "[ProtocolBroker] Failed to spawn isolated Antigravity: {}",
                    e
                );
            }
        }
    } else {
        eprintln!(
            "[ProtocolBroker] No active pending auth flow for '{}', invoking fallback.",
            protocol
        );
    }

    fallback_spawn(protocol, url)
}

fn fallback_spawn(protocol: &str, url: &str) -> i32 {
    eprintln!(
        "[ProtocolBroker] Invoking fallback handler for '{}': {}",
        protocol,
        redact_url(url)
    );

    // If backup exists, restore handler first so Windows resolves correctly
    restore_protocol(protocol);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut cmd = Command::new("rundll32.exe");
        cmd.args(["url.dll,FileProtocolHandler", url]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        if cmd.spawn().is_ok() {
            return 0;
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (protocol, url);
    }

    1
}

fn detect_antigravity_executable() -> PathBuf {
    if let Some(local_app_data) = dirs::data_local_dir() {
        let path = local_app_data.join(r"Programs\antigravity\Antigravity.exe");
        if path.exists() {
            return path;
        }
    }
    PathBuf::from("Antigravity.exe")
}
