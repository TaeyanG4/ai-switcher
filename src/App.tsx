import { useEffect, useState } from "react";
import { Activity, FolderGit2, Globe, Plus, RefreshCw, Settings } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import {
  checkAccountStatus,
  checkLaunchConflict,
  deleteAccount,
  getAppSettings,
  launchProfile,
  listAccounts,
  listFavorites,
  logoutAccount,
  refreshTrayMenu,
  registerGlobalShortcut,
  saveAppSettings,
  startLoginFlow,
  terminateProcess,
  toggleAccountEnabled,
  toggleFavoriteAccount,
  updateAccount,
} from "./api";
import { AccountProfile, ExecutionSurface, ProcessConflictInfo } from "./types";
import { PlatformSection } from "./components/PlatformSection";
import { AccountWizardModal } from "./components/AccountWizardModal";
import { DeleteAccountModal } from "./components/DeleteAccountModal";
import { RenameModal } from "./components/RenameModal";
import { DiagnosticsModal } from "./components/DiagnosticsModal";
import { ConflictDialog } from "./components/ConflictDialog";
import { BrowserProfilesModal } from "./components/BrowserProfilesModal";
import { WorkspacePresetsModal } from "./components/WorkspacePresetsModal";
import { SettingsModal } from "./components/SettingsModal";
import { FirstCloseDialog } from "./components/FirstCloseDialog";
import { useI18n } from "./i18n/I18nContext";
import "./App.css";

export function App() {
  const { t } = useI18n();
  const [accounts, setAccounts] = useState<AccountProfile[]>([]);
  const [loading, setLoading] = useState(true);
  const [showDisabled, setShowDisabled] = useState(false);

  // Modals
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [isDiagModalOpen, setIsDiagModalOpen] = useState(false);
  const [isBrowserModalOpen, setIsBrowserModalOpen] = useState(false);
  const [isWorkspaceModalOpen, setIsWorkspaceModalOpen] = useState(false);
  const [isSettingsModalOpen, setIsSettingsModalOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<AccountProfile | null>(null);
  const [renameTarget, setRenameTarget] = useState<AccountProfile | null>(null);

  // Phase 9: Settings, Theme & Favorites
  const [theme, setTheme] = useState<"system" | "light" | "dark">("system");
  const [favoriteAccountIds, setFavoriteAccountIds] = useState<Set<string>>(new Set());
  const [isFirstCloseOpen, setIsFirstCloseOpen] = useState(false);

  // Process conflict state
  const [conflictState, setConflictState] = useState<{
    conflict: ProcessConflictInfo;
    targetAccount: AccountProfile;
    surface?: ExecutionSurface;
    workspacePath?: string;
  } | null>(null);

  // Toast notification
  const [notification, setNotification] = useState<{ text: string; type: "info" | "success" | "error" } | null>(null);

  const showNotification = (text: string, type: "info" | "success" | "error" = "info") => {
    setNotification({ text, type });
    setTimeout(() => {
      setNotification(null);
    }, 4000);
  };

  const applyTheme = (t: "system" | "light" | "dark") => {
    setTheme(t);
    const isDark =
      t === "dark" ||
      (t === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
    document.documentElement.classList.toggle("theme-dark", isDark);
    document.documentElement.classList.toggle("theme-light", !isDark);
    document.documentElement.setAttribute("data-theme", isDark ? "dark" : "light");
  };

  const loadFavorites = async () => {
    try {
      const favs = await listFavorites();
      const accFavs = new Set(
        favs
          .filter((f) => f.targetType === "account" && f.accountId)
          .map((f) => f.accountId!)
      );
      setFavoriteAccountIds(accFavs);
    } catch (err) {
      console.error("Failed to load favorites:", err);
    }
  };

  const handleToggleFavoriteAccount = async (account: AccountProfile) => {
    try {
      await toggleFavoriteAccount(account.id);
      const next = new Set(favoriteAccountIds);
      if (next.has(account.id)) {
        next.delete(account.id);
        showNotification(t("notify.favoriteRemoved", { name: account.displayName }), "info");
      } else {
        next.add(account.id);
        showNotification(t("notify.favoriteAdded", { name: account.displayName }), "success");
      }
      setFavoriteAccountIds(next);
      await refreshTrayMenu();
    } catch (err: any) {
      console.error("Failed to toggle favorite:", err);
      showNotification(t("notify.favoriteFailed"), "error");
    }
  };

  const loadAccounts = async () => {
    try {
      setLoading(true);
      const data = await listAccounts(showDisabled);
      setAccounts(data);
    } catch (err) {
      console.error("Failed to load accounts:", err);
      showNotification(t("notify.loadAccountsFailed"), "error");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    // 1. Initial settings, theme & favorites
    getAppSettings()
      .then((s) => {
        applyTheme(s.theme as "system" | "light" | "dark");
        if (s.globalShortcut) {
          registerGlobalShortcut(s.globalShortcut, () => {
            console.log("Global hotkey pressed");
          });
        }
      })
      .catch((err) => console.error("Failed to load app settings:", err));

    loadFavorites();

    // 2. Listen to system preference changes
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handleMediaChange = () => {
      getAppSettings().then((s) => {
        if (s.theme === "system") {
          applyTheme("system");
        }
      });
    };
    mediaQuery.addEventListener("change", handleMediaChange);

    // 3. Listen to tray open-settings event
    let unlistenSettings: (() => void) | undefined;
    listen("open-settings", () => {
      setIsSettingsModalOpen(true);
    }).then((fn) => {
      unlistenSettings = fn;
    });

    return () => {
      mediaQuery.removeEventListener("change", handleMediaChange);
      if (unlistenSettings) unlistenSettings();
    };
  }, []);

  useEffect(() => {
    loadAccounts();
  }, [showDisabled]);

  const handleToggleEnabled = async (account: AccountProfile) => {
    const nextState = !account.isEnabled;
    await toggleAccountEnabled(account.id, nextState);
    showNotification(
      nextState
        ? t("notify.profileEnabled", { name: account.displayName })
        : t("notify.profileDisabled", { name: account.displayName }),
      "info"
    );
    await loadAccounts();
  };

  const handleRename = async (id: string, newName: string, newIdentifier?: string) => {
    await updateAccount({
      id,
      displayName: newName,
      accountIdentifier: newIdentifier,
    });
    showNotification(t("notify.profileUpdated"), "success");
    await loadAccounts();
  };

  const handleDeleteConfirm = async (
    account: AccountProfile,
    deleteLocalData: boolean,
    deleteBrowserProfile?: boolean
  ) => {
    try {
      await deleteAccount(account.id, deleteLocalData, deleteBrowserProfile);
      showNotification(
        t("notify.profileDeleted", {
          name: account.displayName,
          extra: deleteLocalData ? t("notify.profileDeletedExtra") : "",
        }),
        "info"
      );
      await loadAccounts();
    } catch (err: any) {
      console.error("Delete error:", err);
      showNotification(t("notify.deleteFailed", { err: err?.details || err?.message || String(err) }), "error");
    }
  };

  const handleLogin = async (account: AccountProfile) => {
    try {
      showNotification(t("notify.loginOpening", { name: account.displayName }), "info");
      const res = await startLoginFlow(account.id);
      showNotification(
        res.message || t("notify.loginLaunchedDefault"),
        "info"
      );
      await loadAccounts();
    } catch (err: any) {
      console.error("Login launch error:", err);
      showNotification(t("notify.loginLaunchFailed", { err: err?.details || err?.message || String(err) }), "error");
    }
  };

  const handleCheckStatus = async (account: AccountProfile, surface?: ExecutionSurface) => {
    try {
      const surfaceLabel = surface ? ` (${surface})` : "";
      showNotification(t("notify.verifyingSession", { name: account.displayName, surface: surfaceLabel }), "info");
      const status = await checkAccountStatus(account.id, surface);
      if (status === "ready") {
        showNotification(t("notify.authReady", { name: account.displayName }), "success");
      } else if (status === "login_required") {
        showNotification(t("notify.authRequired", { name: account.displayName }), "info");
      } else {
        showNotification(t("notify.authStatus", { name: account.displayName, status }), "info");
      }
      await loadAccounts();
    } catch (err: any) {
      console.error("Status check error:", err);
      showNotification(t("notify.statusCheckFailed", { err: err?.details || err?.message || String(err) }), "error");
    }
  };

  const handleLogout = async (account: AccountProfile) => {
    try {
      showNotification(t("notify.loggingOut", { name: account.displayName }), "info");
      const status = await logoutAccount(account.id);
      showNotification(t("notify.loggedOut", { name: account.displayName, status }), "success");
      await loadAccounts();
    } catch (err: any) {
      console.error("Logout error:", err);
      showNotification(t("notify.logoutFailed", { err: err?.details || err?.message || String(err) }), "error");
    }
  };

  const doLaunch = async (account: AccountProfile, surface?: ExecutionSurface, workspacePath?: string) => {
    try {
      const record = await launchProfile(account.id, surface, workspacePath);
      showNotification(
        t("notify.launched", {
          name: account.displayName,
          surface: record.surface,
          pid: record.pid ? ` (PID ${record.pid})` : "",
        }),
        "success"
      );
      await loadAccounts();
    } catch (err: any) {
      console.error("Launch error:", err);
      showNotification(t("notify.launchFailed", { err: err?.details || err?.message || String(err) }), "error");
    }
  };

  const handleOpen = async (account: AccountProfile, surface?: ExecutionSurface) => {
    if (!account.isEnabled) {
      showNotification(t("notify.disabledProfile", { name: account.displayName }), "error");
      return;
    }

    try {
      // Step 1: Check same-platform conflict
      const conflict = await checkLaunchConflict(account.id, surface);
      if (conflict) {
        setConflictState({
          conflict,
          targetAccount: account,
          surface,
        });
        return;
      }

      // Step 2: No conflict -> Launch directly
      await doLaunch(account, surface);
    } catch (err: any) {
      console.error("Conflict check error:", err);
      await doLaunch(account, surface);
    }
  };

  const handleConfirmConflictSwitch = async (conflict: ProcessConflictInfo, targetAccount: AccountProfile) => {
    try {
      // Gracefully terminate the existing profile
      await terminateProcess(conflict.runningLaunchId, false);
      showNotification(
        t("notify.closedAndSwitching", {
          running: conflict.runningDisplayName,
          target: targetAccount.displayName,
        }),
        "info"
      );
      // Launch target profile
      await doLaunch(targetAccount, conflictState?.surface, conflictState?.workspacePath);
    } catch (err: any) {
      console.error("Switch error:", err);
      showNotification(t("notify.switchFailed", { err: err?.details || err?.message || String(err) }), "error");
    } finally {
      setConflictState(null);
    }
  };

  const handleOpenFolder = (account: AccountProfile) => {
    const inputPath = window.prompt(
      `Enter workspace directory for "${account.displayName}":`,
      account.defaultWorkspacePath || "C:\\Projects"
    );
    if (inputPath && inputPath.trim()) {
      doLaunch(account, undefined, inputPath.trim());
    }
  };

  const codexAccounts = accounts.filter((a) => a.platform === "codex");
  const claudeAccounts = accounts.filter((a) => a.platform === "claude");
  const antigravityAccounts = accounts.filter((a) => a.platform === "antigravity");

  return (
    <div className="app-container">
      {/* Top Navigation Bar */}
      <header className="app-header">
        <div className="header-left">
          <h1 className="app-title">{t("app.title")}</h1>
          <span className="app-subtitle">{t("app.subtitle")}</span>
        </div>

        <div className="header-right">
          <label className="checkbox-toggle">
            <input
              type="checkbox"
              checked={showDisabled}
              onChange={(e) => setShowDisabled(e.target.checked)}
            />
            {t("app.showDisabled")}
          </label>

          <button
            className="btn btn-secondary btn-sm"
            onClick={() => setIsWorkspaceModalOpen(true)}
            title={t("app.workspaces")}
          >
            <FolderGit2 size={14} />
            {t("app.workspaces")}
          </button>

          <button
            className="btn btn-secondary btn-sm"
            onClick={() => setIsBrowserModalOpen(true)}
            title={t("app.browsers")}
          >
            <Globe size={14} />
            {t("app.browsers")}
          </button>

          <button
            className="btn btn-secondary btn-sm"
            onClick={() => setIsDiagModalOpen(true)}
            title={t("app.diagnostics")}
          >
            <Activity size={14} />
            {t("app.diagnostics")}
          </button>

          <button
            className="btn btn-secondary btn-sm"
            onClick={() => setIsSettingsModalOpen(true)}
            title={t("app.settings")}
          >
            <Settings size={14} />
            {t("app.settings")}
          </button>

          <button
            className="btn btn-primary btn-sm"
            onClick={() => setIsAddModalOpen(true)}
            title={t("app.addAccount")}
          >
            <Plus size={14} />
            {t("app.addAccount")}
          </button>
        </div>
      </header>

      {/* Notification Toast */}
      {notification && (
        <div className={`notification-toast notification-${notification.type}`}>
          {notification.text}
        </div>
      )}

      {/* Main Content Area */}
      <main className="app-main">
        {loading ? (
          <div className="loading-state">
            <RefreshCw size={24} className="animate-spin" />
            <span>{t("action.loading")}</span>
          </div>
        ) : accounts.length === 0 && !showDisabled ? (
          <div className="welcome-hero">
            <h2>{t("app.title")}</h2>
            <p>
              {t("app.subtitle")}
            </p>
            <button
              className="btn btn-primary"
              onClick={() => setIsAddModalOpen(true)}
              style={{ marginTop: 12 }}
            >
              <Plus size={15} style={{ marginRight: 6 }} />
              {t("platform.addFirstAccount")}
            </button>
          </div>
        ) : (
          <div className="platform-list">
            <PlatformSection
              platform="codex"
              title={t("platform.codex")}
              accounts={codexAccounts}
              favoriteAccountIds={favoriteAccountIds}
              onToggleFavorite={handleToggleFavoriteAccount}
              onOpen={handleOpen}
              onOpenFolder={handleOpenFolder}
              onToggleEnabled={handleToggleEnabled}
              onDelete={(acc) => setDeleteTarget(acc)}
              onRename={(acc) => setRenameTarget(acc)}
              onCheckStatus={handleCheckStatus}
              onLogout={handleLogout}
              onLogin={handleLogin}
            />

            <PlatformSection
              platform="claude"
              title={t("platform.claude")}
              accounts={claudeAccounts}
              favoriteAccountIds={favoriteAccountIds}
              onToggleFavorite={handleToggleFavoriteAccount}
              onOpen={handleOpen}
              onOpenFolder={handleOpenFolder}
              onToggleEnabled={handleToggleEnabled}
              onDelete={(acc) => setDeleteTarget(acc)}
              onRename={(acc) => setRenameTarget(acc)}
              onCheckStatus={handleCheckStatus}
              onLogout={handleLogout}
              onLogin={handleLogin}
            />

            <PlatformSection
              platform="antigravity"
              title={t("platform.antigravity")}
              accounts={antigravityAccounts}
              favoriteAccountIds={favoriteAccountIds}
              onToggleFavorite={handleToggleFavoriteAccount}
              onOpen={handleOpen}
              onOpenFolder={handleOpenFolder}
              onToggleEnabled={handleToggleEnabled}
              onDelete={(acc) => setDeleteTarget(acc)}
              onRename={(acc) => setRenameTarget(acc)}
              onCheckStatus={handleCheckStatus}
              onLogout={handleLogout}
              onLogin={handleLogin}
            />
          </div>
        )}
      </main>

      {/* Status Bar */}
      <footer className="app-footer">
        <div className="footer-left">
          <span>{accounts.length} {t("status.ready")}</span>
          <span className="footer-dot">•</span>
          <span className="text-muted">{t("app.footer.trayActive")}</span>
          <span className="footer-dot">•</span>
          <span className="text-muted">
            {t("app.footer.theme", {
              theme:
                theme === "dark"
                  ? t("theme.dark")
                  : theme === "light"
                  ? t("theme.light")
                  : t("theme.system"),
            })}
          </span>
        </div>
        <div className="footer-right">
          <button className="footer-link" onClick={() => setIsDiagModalOpen(true)}>
            {t("app.footer.diagnostics")}
          </button>
        </div>
      </footer>

      {/* Modals */}
      <AccountWizardModal
        isOpen={isAddModalOpen}
        onClose={() => setIsAddModalOpen(false)}
        onSuccess={loadAccounts}
        onOpenAccount={(acc) => handleOpen(acc)}
      />

      <DeleteAccountModal
        account={deleteTarget}
        allAccounts={accounts}
        isOpen={Boolean(deleteTarget)}
        onClose={() => setDeleteTarget(null)}
        onConfirm={handleDeleteConfirm}
      />

      <RenameModal
        account={renameTarget}
        isOpen={Boolean(renameTarget)}
        onClose={() => setRenameTarget(null)}
        onSave={handleRename}
      />

      <DiagnosticsModal
        isOpen={isDiagModalOpen}
        onClose={() => setIsDiagModalOpen(false)}
        onNotification={showNotification}
      />

      <WorkspacePresetsModal
        isOpen={isWorkspaceModalOpen}
        onClose={() => setIsWorkspaceModalOpen(false)}
        onNotification={showNotification}
      />

      <BrowserProfilesModal
        isOpen={isBrowserModalOpen}
        onClose={() => setIsBrowserModalOpen(false)}
        onNotification={showNotification}
      />

      <ConflictDialog
        conflict={conflictState?.conflict || null}
        targetAccount={conflictState?.targetAccount || null}
        isOpen={Boolean(conflictState)}
        onClose={() => setConflictState(null)}
        onConfirmCloseAndSwitch={handleConfirmConflictSwitch}
      />

      <SettingsModal
        isOpen={isSettingsModalOpen}
        onClose={() => {
          setIsSettingsModalOpen(false);
          loadFavorites();
        }}
        onThemeChange={applyTheme}
        onNotification={showNotification}
      />

      <FirstCloseDialog
        isOpen={isFirstCloseOpen}
        onConfirm={async (dontShowAgain) => {
          setIsFirstCloseOpen(false);
          if (dontShowAgain) {
            const current = await getAppSettings();
            await saveAppSettings({ ...current, firstCloseShown: true });
          }
        }}
      />
    </div>
  );
}

export default App;
