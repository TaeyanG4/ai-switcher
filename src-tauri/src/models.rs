use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlatformType {
    Codex,
    Claude,
    Antigravity,
}

impl PlatformType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlatformType::Codex => "codex",
            PlatformType::Claude => "claude",
            PlatformType::Antigravity => "antigravity",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "codex" => Some(PlatformType::Codex),
            "claude" => Some(PlatformType::Claude),
            "antigravity" => Some(PlatformType::Antigravity),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionSurface {
    DesktopApp,
    Cli,
    Web,
}

impl ExecutionSurface {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionSurface::DesktopApp => "desktop_app",
            ExecutionSurface::Cli => "cli",
            ExecutionSurface::Web => "web",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstancePolicy {
    MultiInstance,
    SingleInstance,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoginMethod {
    Google,
    Email,
    EmailOtp,
    Phone,
    Passkey,
    Oauth,
    Other,
    Unknown,
}

impl LoginMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoginMethod::Google => "google",
            LoginMethod::Email => "email",
            LoginMethod::EmailOtp => "email_otp",
            LoginMethod::Phone => "phone",
            LoginMethod::Passkey => "passkey",
            LoginMethod::Oauth => "oauth",
            LoginMethod::Other => "other",
            LoginMethod::Unknown => "unknown",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "google" => LoginMethod::Google,
            "email" => LoginMethod::Email,
            "email_otp" => LoginMethod::EmailOtp,
            "phone" => LoginMethod::Phone,
            "passkey" => LoginMethod::Passkey,
            "oauth" => LoginMethod::Oauth,
            "other" => LoginMethod::Other,
            _ => LoginMethod::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    Ready,
    Running,
    LoginRequired,
    Unknown,
    Error,
}

impl AccountStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccountStatus::Ready => "ready",
            AccountStatus::Running => "running",
            AccountStatus::LoginRequired => "login_required",
            AccountStatus::Unknown => "unknown",
            AccountStatus::Error => "error",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "ready" => AccountStatus::Ready,
            "running" => AccountStatus::Running,
            "login_required" => AccountStatus::LoginRequired,
            "error" => AccountStatus::Error,
            _ => AccountStatus::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthStatus {
    Authenticated,
    LoginRequired,
    Pending,
    Unknown,
    Error,
}

impl Default for AuthStatus {
    fn default() -> Self {
        AuthStatus::Unknown
    }
}

impl AuthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthStatus::Authenticated => "authenticated",
            AuthStatus::LoginRequired => "login_required",
            AuthStatus::Pending => "pending",
            AuthStatus::Unknown => "unknown",
            AuthStatus::Error => "error",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "authenticated" | "ready" => AuthStatus::Authenticated,
            "login_required" => AuthStatus::LoginRequired,
            "pending" => AuthStatus::Pending,
            "error" => AuthStatus::Error,
            _ => AuthStatus::Unknown,
        }
    }

    pub fn to_account_status(&self) -> AccountStatus {
        match self {
            AuthStatus::Authenticated => AccountStatus::Ready,
            AuthStatus::LoginRequired => AccountStatus::LoginRequired,
            AuthStatus::Pending => AccountStatus::Unknown,
            AuthStatus::Unknown => AccountStatus::Unknown,
            AuthStatus::Error => AccountStatus::Error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStatus {
    Stopped,
    Running,
    Unknown,
}

impl Default for RuntimeStatus {
    fn default() -> Self {
        RuntimeStatus::Stopped
    }
}

impl RuntimeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeStatus::Stopped => "stopped",
            RuntimeStatus::Running => "running",
            RuntimeStatus::Unknown => "unknown",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "running" => RuntimeStatus::Running,
            "stopped" => RuntimeStatus::Stopped,
            _ => RuntimeStatus::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthFlowStartResult {
    pub platform: PlatformType,
    pub flow_type: String,
    pub process_started: bool,
    pub helper_pid: Option<u32>,
    pub verification_mode: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingAuthFlow {
    pub account_id: String,
    pub platform: PlatformType,
    pub surface: ExecutionSurface,
    pub profile_path: String,
    pub custom_executable: Option<String>,
    pub started_at: u64,
    pub helper_pid: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchTarget {
    Default,
    Desktop,
    Cli,
    Web,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserKind {
    Chrome,
    Edge,
    Brave,
    Custom,
}

impl BrowserKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            BrowserKind::Chrome => "chrome",
            BrowserKind::Edge => "edge",
            BrowserKind::Brave => "brave",
            BrowserKind::Custom => "custom",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "chrome" => BrowserKind::Chrome,
            "edge" => BrowserKind::Edge,
            "brave" => BrowserKind::Brave,
            _ => BrowserKind::Custom,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            BrowserKind::Chrome => "Google Chrome",
            BrowserKind::Edge => "Microsoft Edge",
            BrowserKind::Brave => "Brave Browser",
            BrowserKind::Custom => "Custom Chromium",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserProfile {
    pub id: String,
    pub display_name: String,
    pub browser_kind: BrowserKind,
    pub custom_executable_path: Option<String>,
    pub user_data_directory: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBrowserProfileInput {
    pub display_name: String,
    pub browser_kind: BrowserKind,
    pub custom_executable_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBrowserProfileInput {
    pub id: String,
    pub display_name: Option<String>,
    pub browser_kind: Option<BrowserKind>,
    pub custom_executable_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedBrowserInfo {
    pub kind: BrowserKind,
    pub name: String,
    pub executable_path: String,
    pub version: Option<String>,
    pub is_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountProfile {
    pub id: String,
    pub platform: PlatformType,
    pub display_name: String,
    pub account_identifier: Option<String>,
    pub login_method: LoginMethod,
    pub status: AccountStatus,
    #[serde(default)]
    pub auth_status: AuthStatus,
    #[serde(default)]
    pub runtime_status: RuntimeStatus,
    pub profile_path: String,
    pub browser_profile_path: Option<String>,
    pub browser_profile_id: Option<String>,
    pub custom_executable_path: Option<String>,
    pub launch_arguments: Vec<String>,
    pub environment_variables: std::collections::HashMap<String, String>,
    pub default_workspace_path: Option<String>,
    pub is_enabled: bool,
    pub last_launched_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapabilities {
    pub platform: PlatformType,
    pub display_name: String,
    pub description: String,
    pub primary_surface: ExecutionSurface,
    pub supported_surfaces: Vec<ExecutionSurface>,
    pub supports_desktop_isolation: bool,
    pub isolation_summary: String,
    pub instance_policy: InstancePolicy,
    pub supports_automated_status_probe: bool,
    pub supports_logout: bool,
    pub requires_browser_profile: bool,
    pub recommended_browser_profile: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountInput {
    pub platform: PlatformType,
    pub display_name: String,
    pub account_identifier: Option<String>,
    pub login_method: Option<LoginMethod>,
    pub default_workspace_path: Option<String>,
    pub custom_executable_path: Option<String>,
    pub browser_profile_id: Option<String>,
    pub initial_status: Option<AccountStatus>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAccountInput {
    pub id: String,
    pub display_name: Option<String>,
    pub account_identifier: Option<String>,
    pub default_workspace_path: Option<String>,
    pub custom_executable_path: Option<String>,
    pub browser_profile_id: Option<String>,
    pub is_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePreset {
    pub id: String,
    pub name: String,
    pub directory_path: String,
    pub preferred_codex_account_id: Option<String>,
    pub preferred_claude_account_id: Option<String>,
    pub preferred_antigravity_account_id: Option<String>,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceInput {
    pub name: String,
    pub directory_path: String,
    pub preferred_codex_account_id: Option<String>,
    pub preferred_claude_account_id: Option<String>,
    pub preferred_antigravity_account_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkspaceInput {
    pub id: String,
    pub name: Option<String>,
    pub directory_path: Option<String>,
    pub preferred_codex_account_id: Option<String>,
    pub preferred_claude_account_id: Option<String>,
    pub preferred_antigravity_account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessRecord {
    pub launch_id: String,
    pub account_id: String,
    pub platform: PlatformType,
    pub surface: ExecutionSurface,
    pub pid: Option<u32>,
    pub executable: String,
    pub launch_timestamp: String,
    pub is_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessConflictInfo {
    pub running_account_id: String,
    pub running_display_name: String,
    pub running_launch_id: String,
    pub running_pid: Option<u32>,
    pub platform: PlatformType,
    pub policy: InstancePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsInfo {
    pub app_version: String,
    pub os_version: String,
    pub data_dir: String,
    pub codex_executable: Option<String>,
    pub claude_executable: Option<String>,
    pub antigravity_executable: Option<String>,
    pub browser_executable: Option<String>,
    pub active_process_count: usize,
    pub detected_browsers: Vec<DetectedBrowserInfo>,
    pub browser_profile_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteTarget {
    pub id: String,
    pub target_type: String, // "account" | "workspace_platform"
    pub account_id: Option<String>,
    pub workspace_id: Option<String>,
    pub platform: Option<PlatformType>,
    pub sort_order: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: String,
    pub close_to_tray: bool,
    pub start_minimized: bool,
    pub first_close_shown: bool,
    pub global_shortcut: String,
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_language() -> String {
    "en".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            close_to_tray: true,
            start_minimized: false,
            first_close_shown: false,
            global_shortcut: "CommandOrControl+Alt+S".to_string(),
            language: "en".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentItem {
    pub id: String,
    pub kind: String, // "account" | "workspace"
    pub title: String,
    pub subtitle: String,
    pub platform: Option<PlatformType>,
    pub last_used_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileHealth {
    Healthy,
    AuthenticationRequired,
    ExecutableMissing,
    ProfileDirectoryMissing,
    ProfileCorrupted,
    BrowserProfileUnavailable,
    Locked,
    Unknown,
    Error,
}

impl ProfileHealth {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProfileHealth::Healthy => "healthy",
            ProfileHealth::AuthenticationRequired => "authentication_required",
            ProfileHealth::ExecutableMissing => "executable_missing",
            ProfileHealth::ProfileDirectoryMissing => "profile_directory_missing",
            ProfileHealth::ProfileCorrupted => "profile_corrupted",
            ProfileHealth::BrowserProfileUnavailable => "browser_profile_unavailable",
            ProfileHealth::Locked => "locked",
            ProfileHealth::Unknown => "unknown",
            ProfileHealth::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountHealthReport {
    pub account_id: String,
    pub platform: PlatformType,
    pub display_name: String,
    pub status: AccountStatus,
    pub health: ProfileHealth,
    pub details: String,
    pub can_repair: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemHealthReport {
    pub db_healthy: bool,
    pub db_integrity_ok: bool,
    pub schema_version: i32,
    pub db_path: String,
    pub backup_count: usize,
    pub last_backup_at: Option<String>,
    pub managed_root_exists: bool,
    pub managed_root_path: String,
    pub accounts: Vec<AccountHealthReport>,
    pub tray_status: String,
    pub shortcut_status: String,
    pub autostart_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportBundle {
    pub generated_at: String,
    pub app_version: String,
    pub os_version: String,
    pub schema_version: i32,
    pub db_integrity_ok: bool,
    pub platform_capabilities: Vec<PlatformCapabilities>,
    pub accounts: Vec<AccountHealthReport>,
    pub backup_count: usize,
    pub detected_browsers: Vec<DetectedBrowserInfo>,
    pub settings_summary: AppSettings,
}
