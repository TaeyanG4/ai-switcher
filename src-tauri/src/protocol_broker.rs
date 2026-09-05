use std::path::PathBuf;
use std::process::Command;

use crate::models::{ExecutionSurface, PendingAuthFlow, PlatformType};

fn get_base_dir() -> PathBuf {
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
        let _ = std::fs::write(&path, json);
    }
}

pub fn register_pending_auth(
    account_id: &str,
    platform: PlatformType,
    surface: ExecutionSurface,
    profile_path: &str,
    custom_executable: Option<&str>,
    helper_pid: Option<u32>,
) {
    let now = chrono::Utc::now().timestamp() as u64;
    let mut flows = list_pending_auth_flows();

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

    let protocol = match platform {
        PlatformType::Antigravity => Some("antigravity"),
        PlatformType::Claude => Some("claude"),
        _ => None,
    };

    if let Some(proto) = protocol {
        setup_protocol_broker(proto);
    }
}

pub fn cleanup_pending_auth(account_id: Option<&str>, platform: Option<PlatformType>) {
    let mut flows = list_pending_auth_flows();
    let before_count = flows.len();

    if let Some(aid) = account_id {
        flows.retain(|f| f.account_id != aid);
    } else if let Some(p) = platform {
        flows.retain(|f| f.platform != p);
    }

    if flows.len() != before_count {
        save_pending_auth_flows(&flows);
    }

    let has_antigravity = flows
        .iter()
        .any(|f| f.platform == PlatformType::Antigravity);
    let has_claude = flows.iter().any(|f| f.platform == PlatformType::Claude);

    if !has_antigravity {
        restore_protocol("antigravity");
    }
    if !has_claude {
        restore_protocol("claude");
    }
}

pub fn setup_protocol_broker(protocol: &str) {
    #[cfg(windows)]
    {
        let current_exe = match std::env::current_exe() {
            Ok(p) => p,
            Err(_) => return,
        };

        let key_path = format!(r"HKCU\Software\Classes\{}\shell\open\command", protocol);

        let current_cmd = read_registry_default(&key_path);
        let broker_cmd = format!(
            "\"{}\" --broker-protocol {} \"%1\"",
            current_exe.to_string_lossy(),
            protocol
        );

        if let Some(orig) = current_cmd {
            if !orig.contains("--broker-protocol") {
                let backup_path = get_backup_path(protocol);
                if let Some(parent) = backup_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&backup_path, &orig);
            }
        }

        write_registry_default(&key_path, &broker_cmd);
    }
    #[cfg(not(windows))]
    let _ = protocol;
}

pub fn restore_protocol(protocol: &str) {
    #[cfg(windows)]
    {
        let backup_path = get_backup_path(protocol);
        if backup_path.exists() {
            if let Ok(orig_cmd) = std::fs::read_to_string(&backup_path) {
                let trimmed = orig_cmd.trim();
                if !trimmed.is_empty() && !trimmed.contains("--broker-protocol") {
                    let key_path =
                        format!(r"HKCU\Software\Classes\{}\shell\open\command", protocol);
                    write_registry_default(&key_path, trimmed);
                }
            }
            let _ = std::fs::remove_file(&backup_path);
        }
    }
    #[cfg(not(windows))]
    let _ = protocol;
}

pub fn restore_all_protocols() {
    restore_protocol("antigravity");
    restore_protocol("claude");
}

#[cfg(windows)]
fn read_registry_default(key_path: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let output = Command::new("reg.exe")
        .args(["query", key_path, "/ve"])
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

#[cfg(windows)]
fn write_registry_default(key_path: &str, value: &str) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let _ = Command::new("reg.exe")
        .args(["add", key_path, "/ve", "/d", value, "/f"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
}

pub fn handle_broker_cli(protocol: &str, url: &str) -> i32 {
    let safe_url = redact_url(url);
    eprintln!(
        "[ProtocolBroker] Dispatched callback for '{}': {}",
        protocol, safe_url
    );

    let now = chrono::Utc::now().timestamp() as u64;
    let flows = list_pending_auth_flows();

    let target_platform = match protocol.to_lowercase().as_str() {
        "antigravity" => Some(PlatformType::Antigravity),
        "claude" => Some(PlatformType::Claude),
        _ => None,
    };

    let matching_flow = target_platform.and_then(|p| {
        flows
            .iter()
            .find(|f| f.platform == p && now.saturating_sub(f.started_at) < 900)
    });

    if let Some(flow) = matching_flow {
        eprintln!(
            "[ProtocolBroker] Routing callback to active isolated profile: {}",
            flow.profile_path
        );

        match flow.platform {
            PlatformType::Antigravity => {
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
                    Ok(_) => return 0,
                    Err(e) => {
                        eprintln!(
                            "[ProtocolBroker] Failed to spawn isolated Antigravity: {}",
                            e
                        );
                    }
                }
            }
            PlatformType::Claude => {
                let exec = if let Some(custom) = &flow.custom_executable {
                    PathBuf::from(custom)
                } else {
                    detect_claude_executable()
                };

                let profile_buf = PathBuf::from(&flow.profile_path);
                let desktop_dir = profile_buf.join("desktop");

                let mut cmd = Command::new(exec);
                cmd.arg(format!("--user-data-dir={}", desktop_dir.to_string_lossy()));
                cmd.arg(url);

                match cmd.spawn() {
                    Ok(_) => return 0,
                    Err(e) => {
                        eprintln!("[ProtocolBroker] Failed to spawn isolated Claude: {}", e);
                    }
                }
            }
            _ => {}
        }
    } else {
        eprintln!(
            "[ProtocolBroker] No active pending auth flow found for '{}', invoking fallback.",
            protocol
        );
    }

    fallback_spawn(protocol, url)
}

fn fallback_spawn(protocol: &str, url: &str) -> i32 {
    let backup_path = get_backup_path(protocol);
    if backup_path.exists() {
        if let Ok(orig_cmd) = std::fs::read_to_string(&backup_path) {
            let trimmed = orig_cmd.trim();
            if !trimmed.is_empty() && !trimmed.contains("--broker-protocol") {
                if let Some((exe, arg_template)) = parse_command_line(trimmed) {
                    let mut cmd = Command::new(exe);
                    for a in arg_template {
                        if a == "%1" || a == "\"%1\"" {
                            cmd.arg(url);
                        } else {
                            cmd.arg(a);
                        }
                    }
                    if cmd.spawn().is_ok() {
                        return 0;
                    }
                }
            }
        }
    }

    let fallback_exe = match protocol.to_lowercase().as_str() {
        "antigravity" => detect_antigravity_executable(),
        "claude" => detect_claude_executable(),
        _ => PathBuf::from(protocol),
    };

    let mut cmd = Command::new(fallback_exe);
    cmd.arg(url);
    if cmd.spawn().is_ok() {
        0
    } else {
        1
    }
}

fn parse_command_line(cmd: &str) -> Option<(PathBuf, Vec<String>)> {
    let parts = shlex_split(cmd);
    if parts.is_empty() {
        None
    } else {
        let exe = PathBuf::from(&parts[0]);
        let args = parts[1..].to_vec();
        Some((exe, args))
    }
}

fn shlex_split(s: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;

    for c in s.chars() {
        match c {
            '"' => in_quotes = !in_quotes,
            ' ' | '\t' if !in_quotes => {
                if !cur.is_empty() {
                    words.push(cur.clone());
                    cur.clear();
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    words
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

fn detect_claude_executable() -> PathBuf {
    if let Some(local_app_data) = dirs::data_local_dir() {
        let base = local_app_data.join("AnthropicClaude");
        if base.exists() {
            if let Ok(entries) = std::fs::read_dir(&base) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if name.starts_with("app-") {
                                let exe = path.join("claude.exe");
                                if exe.exists() {
                                    return exe;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    PathBuf::from("claude.exe")
}
