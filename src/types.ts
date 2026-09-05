export type PlatformType = 'codex' | 'claude' | 'antigravity';

export type ExecutionSurface = 'desktop_app' | 'cli' | 'web';

export type InstancePolicy = 'multi_instance' | 'single_instance' | 'unknown';

export type LoginMethod =
  | 'google'
  | 'email'
  | 'email_otp'
  | 'phone'
  | 'passkey'
  | 'oauth'
  | 'other'
  | 'unknown';

export type AccountStatus =
  | 'ready'
  | 'running'
  | 'login_required'
  | 'unknown'
  | 'error';

export type AuthStatus =
  | 'authenticated'
  | 'login_required'
  | 'pending'
  | 'unknown'
  | 'error';

export type RuntimeStatus = 'stopped' | 'running' | 'unknown';

export interface AuthFlowStartResult {
  platform: PlatformType;
  flowType: string;
  processStarted: boolean;
  helperPid?: number | null;
  verificationMode: string;
  message: string;
}

export type BrowserKind = 'chrome' | 'edge' | 'brave' | 'custom';

export interface BrowserProfile {
  id: string;
  displayName: string;
  browserKind: BrowserKind;
  customExecutablePath?: string | null;
  userDataDirectory: string;
  createdAt: string;
  updatedAt: string;
  lastUsedAt?: string | null;
}

export interface CreateBrowserProfileInput {
  displayName: string;
  browserKind: BrowserKind;
  customExecutablePath?: string | null;
}

export interface UpdateBrowserProfileInput {
  id: string;
  displayName?: string;
  browserKind?: BrowserKind;
  customExecutablePath?: string | null;
}

export interface DetectedBrowserInfo {
  kind: BrowserKind;
  name: string;
  executablePath: string;
  version?: string | null;
  isAvailable: boolean;
}

export interface AccountAuthState {
  accountId: string;
  surface: ExecutionSurface;
  status: AuthStatus;
  verificationMethod?: string | null;
  verifiedAt?: string | null;
  lastError?: string | null;
}

export interface AccountProfile {
  id: string;
  platform: PlatformType;
  displayName: string;
  accountIdentifier?: string | null;
  loginMethod: LoginMethod;
  status: AccountStatus;
  authStatus?: AuthStatus;
  runtimeStatus?: RuntimeStatus;
  authStates?: AccountAuthState[];
  profilePath: string;
  browserProfilePath?: string | null;
  browserProfileId?: string | null;
  customExecutablePath?: string | null;
  launchArguments: string[];
  environmentVariables: Record<string, string>;
  defaultWorkspacePath?: string | null;
  isEnabled: boolean;
  lastLaunchedAt?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface PlatformCapabilities {
  platform: PlatformType;
  displayName: string;
  description: string;
  primarySurface: ExecutionSurface;
  supportedSurfaces: ExecutionSurface[];
  supportsDesktopIsolation: boolean;
  isolationSummary: string;
  instancePolicy: InstancePolicy;
  supportsAutomatedStatusProbe: boolean;
  supportsLogout: boolean;
  requiresBrowserProfile: boolean;
  recommendedBrowserProfile: boolean;
}

export type WizardStep = 1 | 2 | 3 | 4 | 5 | 6 | 7;

export interface CreateAccountInput {
  platform: PlatformType;
  displayName: string;
  accountIdentifier?: string | null;
  loginMethod?: LoginMethod;
  defaultWorkspacePath?: string | null;
  customExecutablePath?: string | null;
  browserProfileId?: string | null;
  initialStatus?: AccountStatus;
}

export interface UpdateAccountInput {
  id: string;
  displayName?: string;
  accountIdentifier?: string | null;
  defaultWorkspacePath?: string | null;
  customExecutablePath?: string | null;
  browserProfileId?: string | null;
  isEnabled?: boolean;
}

export interface ProcessRecord {
  launchId: string;
  accountId: string;
  platform: PlatformType;
  surface: ExecutionSurface;
  pid?: number | null;
  executable: string;
  launchTimestamp: string;
  isRunning: boolean;
}

export interface ProcessConflictInfo {
  runningAccountId: string;
  runningDisplayName: string;
  runningLaunchId: string;
  runningPid?: number | null;
  platform: PlatformType;
  policy: InstancePolicy;
}

export interface DiagnosticsInfo {
  appVersion: string;
  osVersion: string;
  dataDir: string;
  codexExecutable?: string | null;
  claudeExecutable?: string | null;
  antigravityExecutable?: string | null;
  browserExecutable?: string | null;
  activeProcessCount: number;
  detectedBrowsers?: DetectedBrowserInfo[];
  browserProfileCount?: number;
}

export interface WorkspacePreset {
  id: string;
  name: string;
  directoryPath: string;
  preferredCodexAccountId?: string | null;
  preferredClaudeAccountId?: string | null;
  preferredAntigravityAccountId?: string | null;
  lastOpenedAt?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CreateWorkspaceInput {
  name: string;
  directoryPath: string;
  preferredCodexAccountId?: string | null;
  preferredClaudeAccountId?: string | null;
  preferredAntigravityAccountId?: string | null;
}

export interface UpdateWorkspaceInput {
  id: string;
  name?: string;
  directoryPath?: string;
  preferredCodexAccountId?: string | null;
  preferredClaudeAccountId?: string | null;
  preferredAntigravityAccountId?: string | null;
}

export interface AppSettings {
  theme: "system" | "light" | "dark";
  closeToTray: boolean;
  startMinimized: boolean;
  firstCloseShown: boolean;
  globalShortcut: string;
  language: "en" | "ko" | "ja" | "zh";
}

export interface FavoriteTarget {
  id: string;
  targetType: "account" | "workspace_platform";
  accountId?: string | null;
  workspaceId?: string | null;
  platform?: PlatformType | null;
  sortOrder: number;
  createdAt: string;
}

export interface RecentItem {
  id: string;
  kind: "account" | "workspace";
  title: string;
  subtitle: string;
  platform?: PlatformType | null;
  surface?: ExecutionSurface | null;
  lastUsedAt: string;
}

export type ProfileHealth =
  | "healthy"
  | "authentication_required"
  | "executable_missing"
  | "profile_directory_missing"
  | "profile_corrupted"
  | "browser_profile_unavailable"
  | "locked"
  | "unknown"
  | "error";

export interface AccountHealthReport {
  accountId: string;
  platform: PlatformType;
  displayName: string;
  status: AccountStatus;
  health: ProfileHealth;
  details: string;
  canRepair: boolean;
}

export interface SystemHealthReport {
  dbHealthy: boolean;
  dbIntegrityOk: boolean;
  schemaVersion: number;
  dbPath: string;
  backupCount: number;
  lastBackupAt?: string | null;
  managedRootExists: boolean;
  managedRootPath: string;
  accounts: AccountHealthReport[];
  trayStatus: string;
  shortcutStatus: string;
  autostartStatus: string;
}

export interface SupportBundle {
  generatedAt: string;
  appVersion: string;
  osVersion: string;
  schemaVersion: number;
  dbIntegrityOk: boolean;
  platformCapabilities: PlatformCapabilities[];
  accounts: AccountHealthReport[];
  backupCount: number;
  detectedBrowsers: DetectedBrowserInfo[];
  settingsSummary: AppSettings;
}



