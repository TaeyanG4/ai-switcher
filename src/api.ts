import { invoke } from "@tauri-apps/api/core";
import { isEnabled as isAutostartActive, enable as enableAutostartPlugin, disable as disableAutostartPlugin } from "@tauri-apps/plugin-autostart";
import { register, unregister, isRegistered } from "@tauri-apps/plugin-global-shortcut";
import {
  AccountHealthReport,
  AccountProfile,
  AccountStatus,
  AppSettings,
  AuthFlowStartResult,
  BrowserProfile,
  CreateAccountInput,
  CreateBrowserProfileInput,
  CreateWorkspaceInput,
  DetectedBrowserInfo,
  DiagnosticsInfo,
  ExecutionSurface,
  FavoriteTarget,
  PlatformCapabilities,
  PlatformType,
  ProcessConflictInfo,
  ProcessRecord,
  RecentItem,
  SupportBundle,
  SystemHealthReport,
  UpdateAccountInput,
  UpdateBrowserProfileInput,
  UpdateWorkspaceInput,
  WorkspacePreset,
} from "./types";

export async function listPlatformCapabilities(): Promise<PlatformCapabilities[]> {
  return await invoke<PlatformCapabilities[]>("list_platform_capabilities");
}

export async function listAccounts(includeDisabled = false): Promise<AccountProfile[]> {
  return await invoke<AccountProfile[]>("list_accounts", {
    includeDisabled,
  });
}

export async function getAccount(id: string): Promise<AccountProfile> {
  return await invoke<AccountProfile>("get_account", { id });
}

export async function createAccount(input: CreateAccountInput): Promise<AccountProfile> {
  return await invoke<AccountProfile>("create_account", { input });
}

export async function updateAccount(input: UpdateAccountInput): Promise<AccountProfile> {
  return await invoke<AccountProfile>("update_account", { input });
}

export async function toggleAccountEnabled(id: string, isEnabled: boolean): Promise<void> {
  await invoke("toggle_account_enabled", { id, isEnabled });
}

export async function deleteAccount(
  id: string,
  deleteLocalData: boolean,
  deleteBrowserProfile?: boolean
): Promise<void> {
  await invoke("delete_account", { id, deleteLocalData, deleteBrowserProfile });
}

export async function cleanupDraftAccount(id: string): Promise<void> {
  await invoke("cleanup_draft_account", { id });
}

export async function startLoginFlow(id: string): Promise<AuthFlowStartResult> {
  return await invoke<AuthFlowStartResult>("start_login_flow", { id });
}

export async function checkAccountStatus(
  id: string,
  surface?: ExecutionSurface
): Promise<AccountStatus> {
  return await invoke<AccountStatus>("check_account_status", { id, surface });
}

export async function logoutAccount(id: string): Promise<AccountStatus> {
  return await invoke<AccountStatus>("logout_account", { id });
}

export async function checkLaunchConflict(
  accountId: string,
  surface?: ExecutionSurface
): Promise<ProcessConflictInfo | null> {
  return await invoke<ProcessConflictInfo | null>("check_launch_conflict", {
    accountId,
    surface,
  });
}

export async function launchProfile(
  accountId: string,
  surface?: ExecutionSurface,
  workspacePath?: string
): Promise<ProcessRecord> {
  return await invoke<ProcessRecord>("launch_profile", {
    accountId,
    surface,
    workspacePath,
  });
}

export async function terminateProcess(launchId: string, force = false): Promise<void> {
  await invoke("terminate_process", { launchId, force });
}

export async function listActiveProcesses(): Promise<ProcessRecord[]> {
  return await invoke<ProcessRecord[]>("list_active_processes");
}

export async function getDiagnostics(): Promise<DiagnosticsInfo> {
  return await invoke<DiagnosticsInfo>("get_diagnostics");
}

export async function listBrowserProfiles(): Promise<BrowserProfile[]> {
  return await invoke<BrowserProfile[]>("list_browser_profiles");
}

export async function getBrowserProfile(id: string): Promise<BrowserProfile> {
  return await invoke<BrowserProfile>("get_browser_profile", { id });
}

export async function createBrowserProfile(input: CreateBrowserProfileInput): Promise<BrowserProfile> {
  return await invoke<BrowserProfile>("create_browser_profile", { input });
}

export async function updateBrowserProfile(input: UpdateBrowserProfileInput): Promise<BrowserProfile> {
  return await invoke<BrowserProfile>("update_browser_profile", { input });
}

export async function deleteBrowserProfile(id: string, deleteLocalData: boolean): Promise<void> {
  await invoke("delete_browser_profile", { id, deleteLocalData });
}

export async function launchBrowserProfile(id: string, url?: string): Promise<number> {
  return await invoke<number>("launch_browser_profile", { id, url });
}

export async function detectAvailableBrowsers(): Promise<DetectedBrowserInfo[]> {
  return await invoke<DetectedBrowserInfo[]>("detect_available_browsers");
}

export async function listWorkspaces(): Promise<WorkspacePreset[]> {
  return await invoke<WorkspacePreset[]>("list_workspaces");
}

export async function getWorkspace(id: string): Promise<WorkspacePreset> {
  return await invoke<WorkspacePreset>("get_workspace", { id });
}

export async function createWorkspace(input: CreateWorkspaceInput): Promise<WorkspacePreset> {
  return await invoke<WorkspacePreset>("create_workspace", { input });
}

export async function updateWorkspace(input: UpdateWorkspaceInput): Promise<WorkspacePreset> {
  return await invoke<WorkspacePreset>("update_workspace", { input });
}

export async function deleteWorkspace(id: string): Promise<void> {
  await invoke("delete_workspace", { id });
}

export async function launchWorkspacePreset(
  workspaceId: string,
  platform: PlatformType,
  surface?: ExecutionSurface
): Promise<ProcessRecord> {
  return await invoke<ProcessRecord>("launch_workspace_preset", {
    workspaceId,
    platform,
    surface,
  });
}

export async function createWorkspaceDirectory(workspaceId: string): Promise<void> {
  await invoke("create_workspace_directory", { workspaceId });
}

export async function pickDirectory(defaultPath?: string): Promise<string | null> {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: defaultPath || undefined,
      title: "Select Directory",
    });
    if (typeof selected === "string") {
      return selected;
    }
  } catch (err) {
    console.error("Failed to open native directory picker:", err);
  }
  return null;
}

export async function getAppSettings(): Promise<AppSettings> {
  return await invoke<AppSettings>("get_app_settings");
}

export async function saveAppSettings(settings: AppSettings): Promise<void> {
  await invoke("save_app_settings", { settings });
}

export async function listFavorites(): Promise<FavoriteTarget[]> {
  return await invoke<FavoriteTarget[]>("list_favorites");
}

export async function toggleFavoriteAccount(accountId: string): Promise<boolean> {
  return await invoke<boolean>("toggle_favorite_account", { accountId });
}

export async function toggleFavoriteWorkspace(
  workspaceId: string,
  platform?: string
): Promise<boolean> {
  return await invoke<boolean>("toggle_favorite_workspace", { workspaceId, platform });
}

export async function getRecentLaunches(limit?: number): Promise<RecentItem[]> {
  return await invoke<RecentItem[]>("get_recent_launches", { limit });
}

export async function refreshTrayMenu(): Promise<void> {
  await invoke("refresh_tray_menu");
}

export async function isAutostartEnabled(): Promise<boolean> {
  try {
    return await isAutostartActive();
  } catch (err) {
    console.error("Failed to check autostart:", err);
    return false;
  }
}

export async function setAutostartEnabled(enable: boolean): Promise<void> {
  try {
    if (enable) {
      await enableAutostartPlugin();
    } else {
      await disableAutostartPlugin();
    }
  } catch (err) {
    console.error("Failed to set autostart:", err);
    throw err;
  }
}

export async function registerGlobalShortcut(
  shortcut: string,
  handler: () => void
): Promise<boolean> {
  try {
    if (await isRegistered(shortcut)) {
      await unregister(shortcut);
    }
    await register(shortcut, (event) => {
      if (event.state === "Pressed") {
        handler();
      }
    });
    return true;
  } catch (err) {
    console.error(`Failed to register global shortcut '${shortcut}':`, err);
    return false;
  }
}

export async function unregisterGlobalShortcut(shortcut: string): Promise<void> {
  try {
    if (await isRegistered(shortcut)) {
      await unregister(shortcut);
    }
  } catch (err) {
    console.error(`Failed to unregister global shortcut '${shortcut}':`, err);
  }
}

export async function checkAccountHealth(id: string): Promise<AccountHealthReport> {
  return await invoke<AccountHealthReport>("check_account_health", { id });
}

export async function repairAccountProfile(id: string): Promise<AccountHealthReport> {
  return await invoke<AccountHealthReport>("repair_account_profile", { id });
}

export async function getSystemHealth(): Promise<SystemHealthReport> {
  return await invoke<SystemHealthReport>("get_system_health");
}

export async function createDbBackup(): Promise<string> {
  return await invoke<string>("create_db_backup");
}

export async function checkDbIntegrity(): Promise<boolean> {
  return await invoke<boolean>("check_db_integrity");
}

export async function exportSupportBundle(): Promise<SupportBundle> {
  return await invoke<SupportBundle>("export_support_bundle");
}





