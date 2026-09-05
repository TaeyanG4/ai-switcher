import React, { useEffect, useState } from "react";
import {
  X,
  Settings,
  Monitor,
  Sun,
  Moon,
  Keyboard,
  Check,
  AlertTriangle,
  RotateCw,
  Globe,
} from "lucide-react";
import {
  getAppSettings,
  saveAppSettings,
  isAutostartEnabled,
  setAutostartEnabled,
  registerGlobalShortcut,
  getDiagnostics,
} from "../api";
import { AppSettings, DiagnosticsInfo } from "../types";
import { useI18n } from "../i18n/I18nContext";
import { SUPPORTED_LANGUAGES, Language } from "../i18n/translations";

interface Props {
  isOpen: boolean;
  onClose: () => void;
  onThemeChange: (theme: "system" | "light" | "dark") => void;
  onNotification?: (text: string, type: "info" | "success" | "error") => void;
}

export const SettingsModal: React.FC<Props> = ({
  isOpen,
  onClose,
  onThemeChange,
  onNotification,
}) => {
  const { language, setLanguage, t } = useI18n();
  const [activeTab, setActiveTab] = useState<"general" | "appearance" | "language" | "shortcuts" | "diagnostics">("general");
  const [settings, setSettings] = useState<AppSettings>({
    theme: "system",
    closeToTray: true,
    startMinimized: false,
    firstCloseShown: false,
    globalShortcut: "CommandOrControl+Alt+S",
    language: "en",
  });
  const [autostartActive, setAutostartActive] = useState<boolean>(false);
  const [diagnostics, setDiagnostics] = useState<DiagnosticsInfo | null>(null);
  const [shortcutInput, setShortcutInput] = useState("");
  const [shortcutStatus, setShortcutStatus] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (isOpen) {
      loadSettings();
    }
  }, [isOpen]);

  const loadSettings = async () => {
    try {
      setLoading(true);
      const [appSettings, autoEnabled, diag] = await Promise.all([
        getAppSettings(),
        isAutostartEnabled(),
        getDiagnostics().catch(() => null),
      ]);
      setSettings(appSettings);
      setAutostartActive(autoEnabled);
      setShortcutInput(appSettings.globalShortcut || "CommandOrControl+Alt+S");
      setDiagnostics(diag);
    } catch (err: any) {
      console.error("Failed to load settings:", err);
      if (onNotification) onNotification("Failed to load settings: " + err?.message, "error");
    } finally {
      setLoading(false);
    }
  };

  const handleToggleAutostart = async () => {
    const nextVal = !autostartActive;
    try {
      await setAutostartEnabled(nextVal);
      setAutostartActive(nextVal);
      if (onNotification) {
        onNotification(
          nextVal ? "Windows Autostart enabled." : "Windows Autostart disabled.",
          "success"
        );
      }
    } catch (err: any) {
      console.error("Failed to toggle autostart:", err);
      if (onNotification) onNotification("Failed to update autostart: " + err?.message, "error");
    }
  };

  const handleUpdateSetting = async <K extends keyof AppSettings>(
    key: K,
    value: AppSettings[K]
  ) => {
    const updated = { ...settings, [key]: value };
    setSettings(updated);
    try {
      await saveAppSettings(updated);
      if (key === "theme") {
        onThemeChange(value as "system" | "light" | "dark");
      }
      if (key === "language") {
        await setLanguage(value as Language);
      }
    } catch (err: any) {
      console.error(`Failed to save setting ${key}:`, err);
      if (onNotification) onNotification("Failed to save setting: " + err?.message, "error");
    }
  };

  const handleRegisterShortcut = async () => {
    const shortcutToRegister = shortcutInput.trim();
    if (!shortcutToRegister) {
      setShortcutStatus("Shortcut cannot be empty.");
      return;
    }

    setSaving(true);
    setShortcutStatus(null);
    try {
      const success = await registerGlobalShortcut(shortcutToRegister, () => {
        console.log("Global hotkey triggered");
      });

      if (success) {
        await handleUpdateSetting("globalShortcut", shortcutToRegister);
        setShortcutStatus("Registered successfully!");
        if (onNotification) onNotification(`Shortcut '${shortcutToRegister}' registered.`, "success");
      } else {
        setShortcutStatus("Failed to register. Key combination may be reserved or invalid.");
      }
    } catch (err: any) {
      setShortcutStatus(`Error: ${err?.message || String(err)}`);
    } finally {
      setSaving(false);
    }
  };

  const handleResetTrayNotice = async () => {
    await handleUpdateSetting("firstCloseShown", false);
    if (onNotification) onNotification("First-close tray explanation re-enabled.", "info");
  };

  if (!isOpen) return null;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal-content settings-modal"
        onClick={(e) => e.stopPropagation()}
        style={{ maxWidth: 640 }}
      >
        <div className="modal-header">
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <Settings size={20} className="header-icon" />
            <h2 className="modal-title">{t("settings.title")}</h2>
          </div>
          <button className="btn btn-icon" onClick={onClose}>
            <X size={16} />
          </button>
        </div>

        {/* Tabs */}
        <div className="settings-tabs">
          <button
            className={`settings-tab-btn ${activeTab === "general" ? "active" : ""}`}
            onClick={() => setActiveTab("general")}
          >
            {t("settings.tabs.general")}
          </button>
          <button
            className={`settings-tab-btn ${activeTab === "appearance" ? "active" : ""}`}
            onClick={() => setActiveTab("appearance")}
          >
            {t("settings.tabs.appearance")}
          </button>
          <button
            className={`settings-tab-btn ${activeTab === "language" ? "active" : ""}`}
            onClick={() => setActiveTab("language")}
          >
            <Globe size={13} style={{ marginRight: 4, verticalAlign: "middle" }} />
            {t("settings.tabs.language")}
          </button>
          <button
            className={`settings-tab-btn ${activeTab === "shortcuts" ? "active" : ""}`}
            onClick={() => setActiveTab("shortcuts")}
          >
            {t("settings.tabs.shortcuts")}
          </button>
          <button
            className={`settings-tab-btn ${activeTab === "diagnostics" ? "active" : ""}`}
            onClick={() => setActiveTab("diagnostics")}
          >
            {t("settings.tabs.diagnostics")}
          </button>
        </div>

        <div className="settings-body" style={{ minHeight: 280 }}>
          {loading ? (
            <div className="loading-container">
              <RotateCw className="spin" size={24} />
              <span>{t("action.loading")}</span>
            </div>
          ) : (
            <>
              {/* TAB 1: GENERAL */}
              {activeTab === "general" && (
                <div className="settings-section">
                  <div className="settings-item">
                    <div className="settings-item-info">
                      <span className="settings-item-title">{t("settings.autostart.title")}</span>
                      <span className="settings-item-desc">{t("settings.autostart.desc")}</span>
                    </div>
                    <label className="toggle-switch">
                      <input
                        type="checkbox"
                        checked={autostartActive}
                        onChange={handleToggleAutostart}
                      />
                      <span className="slider"></span>
                    </label>
                  </div>

                  <div className="settings-item">
                    <div className="settings-item-info">
                      <span className="settings-item-title">{t("settings.startMinimized.title")}</span>
                      <span className="settings-item-desc">{t("settings.startMinimized.desc")}</span>
                    </div>
                    <label className="toggle-switch">
                      <input
                        type="checkbox"
                        checked={settings.startMinimized}
                        onChange={(e) => handleUpdateSetting("startMinimized", e.target.checked)}
                      />
                      <span className="slider"></span>
                    </label>
                  </div>

                  <div className="settings-item">
                    <div className="settings-item-info">
                      <span className="settings-item-title">{t("settings.closeToTray.title")}</span>
                      <span className="settings-item-desc">{t("settings.closeToTray.desc")}</span>
                    </div>
                    <label className="toggle-switch">
                      <input
                        type="checkbox"
                        checked={settings.closeToTray}
                        onChange={(e) => handleUpdateSetting("closeToTray", e.target.checked)}
                      />
                      <span className="slider"></span>
                    </label>
                  </div>

                  <div className="settings-item" style={{ borderBottom: "none" }}>
                    <div className="settings-item-info">
                      <span className="settings-item-title">{t("settings.firstCloseNotice.title")}</span>
                      <span className="settings-item-desc">{t("settings.firstCloseNotice.desc")}</span>
                    </div>
                    <button
                      className="btn btn-secondary btn-sm"
                      onClick={handleResetTrayNotice}
                    >
                      {t("settings.firstCloseNotice.resetBtn")}
                    </button>
                  </div>
                </div>
              )}

              {/* TAB 2: APPEARANCE */}
              {activeTab === "appearance" && (
                <div className="settings-section">
                  <div className="theme-selector-grid">
                    <div
                      className={`theme-card ${settings.theme === "system" ? "selected" : ""}`}
                      onClick={() => handleUpdateSetting("theme", "system")}
                    >
                      <Monitor size={28} />
                      <span className="theme-card-title">{t("settings.theme.system")}</span>
                      <span className="theme-card-desc">{t("settings.theme.systemDesc")}</span>
                      {settings.theme === "system" && <Check size={16} className="theme-check" />}
                    </div>

                    <div
                      className={`theme-card ${settings.theme === "light" ? "selected" : ""}`}
                      onClick={() => handleUpdateSetting("theme", "light")}
                    >
                      <Sun size={28} />
                      <span className="theme-card-title">{t("settings.theme.light")}</span>
                      <span className="theme-card-desc">{t("settings.theme.lightDesc")}</span>
                      {settings.theme === "light" && <Check size={16} className="theme-check" />}
                    </div>

                    <div
                      className={`theme-card ${settings.theme === "dark" ? "selected" : ""}`}
                      onClick={() => handleUpdateSetting("theme", "dark")}
                    >
                      <Moon size={28} />
                      <span className="theme-card-title">{t("settings.theme.dark")}</span>
                      <span className="theme-card-desc">{t("settings.theme.darkDesc")}</span>
                      {settings.theme === "dark" && <Check size={16} className="theme-check" />}
                    </div>
                  </div>
                </div>
              )}

              {/* TAB 3: LANGUAGE */}
              {activeTab === "language" && (
                <div className="settings-section">
                  <div className="settings-item-info" style={{ marginBottom: 16 }}>
                    <span className="settings-item-title">{t("settings.language.title")}</span>
                    <span className="settings-item-desc">{t("settings.language.desc")}</span>
                  </div>

                  <div className="language-selector-grid">
                    {SUPPORTED_LANGUAGES.map((lang) => {
                      const isSelected = (settings.language || language) === lang.code;
                      return (
                        <div
                          key={lang.code}
                          className={`language-card ${isSelected ? "selected" : ""}`}
                          onClick={() => handleUpdateSetting("language", lang.code)}
                        >
                          <span className="language-card-native">{lang.nativeName}</span>
                          <span className="language-card-name">{lang.label}</span>
                          {isSelected && <Check size={16} className="language-check" />}
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}

              {/* TAB 4: SHORTCUTS */}
              {activeTab === "shortcuts" && (
                <div className="settings-section">
                  <div className="settings-item-info" style={{ marginBottom: 12 }}>
                    <span className="settings-item-title">{t("settings.shortcuts.title")}</span>
                    <span className="settings-item-desc">{t("settings.shortcuts.desc")}</span>
                  </div>

                  <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
                    <div style={{ position: "relative", flex: 1 }}>
                      <Keyboard
                        size={16}
                        style={{ position: "absolute", left: 10, top: 10, opacity: 0.5 }}
                      />
                      <input
                        type="text"
                        className="form-input"
                        style={{ paddingLeft: 34 }}
                        value={shortcutInput}
                        onChange={(e) => setShortcutInput(e.target.value)}
                        placeholder={t("settings.shortcuts.placeholder")}
                      />
                    </div>
                    <button
                      className="btn btn-primary"
                      onClick={handleRegisterShortcut}
                      disabled={saving}
                    >
                      {saving ? t("action.saving") : t("settings.shortcuts.applyBtn")}
                    </button>
                  </div>

                  {shortcutStatus && (
                    <div
                      style={{
                        marginTop: 10,
                        fontSize: 13,
                        color: shortcutStatus.includes("success") ? "var(--status-ready)" : "var(--status-error)",
                        display: "flex",
                        alignItems: "center",
                        gap: 6,
                      }}
                    >
                      {shortcutStatus.includes("success") ? <Check size={14} /> : <AlertTriangle size={14} />}
                      {shortcutStatus}
                    </div>
                  )}

                  <div className="shortcut-help-box" style={{ marginTop: 18 }}>
                    <strong>{t("settings.shortcuts.modifiersTitle")}</strong>
                    <ul>
                      <li><code>{t("settings.shortcuts.ctrl")}</code></li>
                      <li><code>{t("settings.shortcuts.alt")}</code></li>
                      <li><code>{t("settings.shortcuts.shift")}</code></li>
                      <li>{t("settings.shortcuts.example")}</li>
                    </ul>
                  </div>
                </div>
              )}

              {/* TAB 5: DIAGNOSTICS */}
              {activeTab === "diagnostics" && (
                <div className="settings-section">
                  <div className="diag-grid">
                    <div className="diag-card">
                      <span className="diag-label">{t("settings.diag.appVersion")}</span>
                      <span className="diag-value">v0.1.0</span>
                    </div>
                    <div className="diag-card">
                      <span className="diag-label">{t("settings.diag.activeProcesses")}</span>
                      <span className="diag-value">{diagnostics?.activeProcessCount ?? 0}</span>
                    </div>
                  </div>

                  <div style={{ marginTop: 14 }}>
                    <span className="diag-label">{t("settings.diag.storageLocations")}</span>
                    <div className="diag-path-box">
                      <strong>{t("settings.diag.baseDataDir")}</strong>
                      <div className="diag-code-path">{diagnostics?.dataDir || "C:\\Users\\...\\AppData\\Roaming\\AI-Switcher"}</div>
                    </div>
                  </div>

                  <div style={{ marginTop: 14 }}>
                    <span className="diag-label">{t("settings.diag.capabilities")}</span>
                    <div className="diag-capabilities-list">
                      <div className="diag-cap-item">
                        <span className="diag-platform codex">{t("platform.codex")}</span>
                        <span className="diag-cap-desc">Desktop app (single-instance session) / CLI (CODEX_HOME isolation)</span>
                      </div>
                      <div className="diag-cap-item">
                        <span className="diag-platform claude">{t("platform.claude")}</span>
                        <span className="diag-cap-desc">Desktop Code (isolated multi-instance) / Web / CLI</span>
                      </div>
                      <div className="diag-cap-item">
                        <span className="diag-platform antigravity">{t("platform.antigravity")}</span>
                        <span className="diag-cap-desc">Desktop app (isolated multi-instance profiles, verified CWD-only)</span>
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </>
          )}
        </div>

        <div className="modal-footer">
          <button className="btn btn-secondary" onClick={onClose}>
            {t("action.close")}
          </button>
        </div>
      </div>
    </div>
  );
};
