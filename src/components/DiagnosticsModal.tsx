import React, { useEffect, useState } from "react";
import {
  Activity,
  CheckCircle,
  Database,
  Download,
  RefreshCw,
  Wrench,
  X,
  XCircle,
} from "lucide-react";
import {
  createDbBackup,
  exportSupportBundle,
  getDiagnostics,
  getSystemHealth,
  repairAccountProfile,
} from "../api";
import { DiagnosticsInfo, SystemHealthReport } from "../types";
import { useI18n } from "../i18n/I18nContext";

interface Props {
  isOpen: boolean;
  onClose: () => void;
  onNotification?: (text: string, type: "info" | "success" | "error") => void;
}

export const DiagnosticsModal: React.FC<Props> = ({ isOpen, onClose, onNotification }) => {
  const { t } = useI18n();
  const [diag, setDiag] = useState<DiagnosticsInfo | null>(null);
  const [health, setHealth] = useState<SystemHealthReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [repairingId, setRepairingId] = useState<string | null>(null);
  const [backingUp, setBackingUp] = useState(false);

  const fetchDiag = async () => {
    try {
      setLoading(true);
      const [diagData, healthData] = await Promise.all([
        getDiagnostics(),
        getSystemHealth().catch((e) => {
          console.error("Health check error:", e);
          return null;
        }),
      ]);
      setDiag(diagData);
      setHealth(healthData);
    } catch (err) {
      console.error("Failed to load diagnostics:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (isOpen) {
      fetchDiag();
    }
  }, [isOpen]);

  const handleCreateBackup = async () => {
    try {
      setBackingUp(true);
      const path = await createDbBackup();
      if (onNotification) {
        onNotification(`Database backup created: ${path}`, "success");
      }
      await fetchDiag();
    } catch (err: any) {
      console.error("Backup failed:", err);
      if (onNotification) {
        onNotification(`Backup failed: ${err?.details || err?.message || String(err)}`, "error");
      }
    } finally {
      setBackingUp(false);
    }
  };

  const handleRepairAccount = async (accountId: string, displayName: string) => {
    try {
      setRepairingId(accountId);
      const res = await repairAccountProfile(accountId);
      if (onNotification) {
        onNotification(
          `Repaired "${displayName}". Status: ${res.status}, Health: ${res.health}.`,
          "success"
        );
      }
      await fetchDiag();
    } catch (err: any) {
      console.error("Repair failed:", err);
      if (onNotification) {
        onNotification(`Repair failed: ${err?.details || err?.message || String(err)}`, "error");
      }
    } finally {
      setRepairingId(null);
    }
  };

  const handleExportSupportBundle = async () => {
    try {
      const bundle = await exportSupportBundle();
      const jsonStr = JSON.stringify(bundle, null, 2);
      const blob = new Blob([jsonStr], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `ai-switcher-support-bundle-${new Date().toISOString().slice(0, 10)}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      if (onNotification) {
        onNotification("Support bundle exported (100% sanitized, zero secrets).", "success");
      }
    } catch (err: any) {
      console.error("Export bundle failed:", err);
      if (onNotification) {
        onNotification(`Export failed: ${err?.details || err?.message || String(err)}`, "error");
      }
    }
  };

  if (!isOpen) return null;

  return (
    <div className="modal-overlay">
      <div className="modal-dialog" style={{ maxWidth: 680 }}>
        <div className="modal-header">
          <h3 className="flex items-center gap-2">
            <Activity size={18} />
            {t("diagCenter.title")}
          </h3>
          <button className="btn-close" onClick={onClose}>
            <X size={16} />
          </button>
        </div>

        <div className="modal-body" style={{ maxHeight: "70vh", overflowY: "auto" }}>
          {loading ? (
            <div className="loading-spinner">Loading system diagnostics...</div>
          ) : diag ? (
            <div className="diagnostics-list">
              {/* System & Database Section */}
              <div className="diag-row">
                <span className="diag-label">AI Switcher Version:</span>
                <span className="diag-value font-mono">{diag.appVersion}</span>
              </div>
              <div className="diag-row">
                <span className="diag-label">Host OS:</span>
                <span className="diag-value">{diag.osVersion}</span>
              </div>
              <div className="diag-row">
                <span className="diag-label">Profile Storage:</span>
                <span className="diag-value font-mono text-xs">{diag.dataDir}</span>
              </div>

              {health && (
                <>
                  <div className="diag-row">
                    <span className="diag-label">Database Schema:</span>
                    <span className="diag-value font-mono">
                      v{health.schemaVersion} ({health.dbIntegrityOk ? "Integrity OK" : "Integrity Warning"})
                    </span>
                  </div>
                  <div className="diag-row">
                    <span className="diag-label">Database Backups:</span>
                    <span className="diag-value flex items-center gap-2">
                      <span>{health.backupCount} retained</span>
                      <button
                        type="button"
                        className="btn btn-secondary btn-sm flex items-center gap-1"
                        onClick={handleCreateBackup}
                        disabled={backingUp}
                        style={{ padding: "2px 8px", fontSize: "11px" }}
                      >
                        <Database size={12} />
                        {backingUp ? "Backing up..." : "Backup Now"}
                      </button>
                    </span>
                  </div>
                </>
              )}

              <div className="diag-divider" />
              <h4>Detected Executables</h4>

              <div className="diag-row">
                <span className="diag-label">OpenAI Codex:</span>
                <span className="diag-value">
                  {diag.codexExecutable ? (
                    <span className="flex items-center gap-1 text-success">
                      <CheckCircle size={14} /> {diag.codexExecutable}
                    </span>
                  ) : (
                    <span className="flex items-center gap-1 text-danger">
                      <XCircle size={14} /> Not detected in PATH
                    </span>
                  )}
                </span>
              </div>

              <div className="diag-row">
                <span className="diag-label">Anthropic Claude:</span>
                <span className="diag-value">
                  {diag.claudeExecutable ? (
                    <span className="flex items-center gap-1 text-success">
                      <CheckCircle size={14} /> {diag.claudeExecutable}
                    </span>
                  ) : (
                    <span className="flex items-center gap-1 text-danger">
                      <XCircle size={14} /> Not detected
                    </span>
                  )}
                </span>
              </div>

              <div className="diag-row">
                <span className="diag-label">Google Antigravity:</span>
                <span className="diag-value">
                  {diag.antigravityExecutable ? (
                    <span className="flex items-center gap-1 text-success">
                      <CheckCircle size={14} /> {diag.antigravityExecutable}
                    </span>
                  ) : (
                    <span className="flex items-center gap-1 text-danger">
                      <XCircle size={14} /> Not detected
                    </span>
                  )}
                </span>
              </div>

              <div className="diag-divider" />
              <h4>Account Profile Health & Repair</h4>

              {health && health.accounts && health.accounts.length > 0 ? (
                <div className="flex flex-col gap-2" style={{ marginTop: 8 }}>
                  {health.accounts.map((acc) => (
                    <div
                      key={acc.accountId}
                      className="p-2 rounded border"
                      style={{
                        background: "var(--color-bg-subtle)",
                        borderColor: "var(--color-border)",
                        display: "flex",
                        justifyContent: "space-between",
                        alignItems: "center",
                      }}
                    >
                      <div>
                        <div className="flex items-center gap-2">
                          <span className="font-semibold text-sm">{acc.displayName}</span>
                          <span className="text-xs text-muted">({acc.platform})</span>
                          <span
                            className={`badge ${
                              acc.health === "healthy"
                                ? "badge-ready"
                                : acc.health === "profile_directory_missing"
                                ? "badge-error"
                                : "badge-unknown"
                            }`}
                            style={{ fontSize: "10px", padding: "1px 6px" }}
                          >
                            {acc.health.replace(/_/g, " ")}
                          </span>
                        </div>
                        <div className="text-xs text-muted" style={{ marginTop: 2 }}>
                          {acc.details}
                        </div>
                      </div>

                      {acc.canRepair && (
                        <button
                          type="button"
                          className="btn btn-secondary btn-sm flex items-center gap-1"
                          onClick={() => handleRepairAccount(acc.accountId, acc.displayName)}
                          disabled={repairingId === acc.accountId}
                          style={{ padding: "4px 8px", fontSize: "11px", whiteSpace: "nowrap" }}
                        >
                          <Wrench size={12} />
                          {repairingId === acc.accountId ? "Repairing..." : "Repair Structure"}
                        </button>
                      )}
                    </div>
                  ))}
                </div>
              ) : (
                <div className="text-xs text-muted">No account profiles registered yet.</div>
              )}

              <div className="diag-divider" />
              <h4>Detected Chromium Browsers</h4>

              {diag.detectedBrowsers && diag.detectedBrowsers.length > 0 ? (
                diag.detectedBrowsers.map((b) => (
                  <div key={b.kind} className="diag-row">
                    <span className="diag-label">{b.name}:</span>
                    <span className="diag-value">
                      {b.isAvailable ? (
                        <span className="flex items-center gap-1 text-success">
                          <CheckCircle size={14} /> {b.executablePath} {b.version ? `(v${b.version})` : ""}
                        </span>
                      ) : (
                        <span className="flex items-center gap-1 text-muted">
                          <XCircle size={14} /> Not detected
                        </span>
                      )}
                    </span>
                  </div>
                ))
              ) : (
                <div className="diag-row">
                  <span className="diag-label">Chromium Browsers:</span>
                  <span className="diag-value text-muted">None detected</span>
                </div>
              )}

              <div className="diag-divider" />
              <div className="diag-row">
                <span className="diag-label">Configured Browser Profiles:</span>
                <span className="diag-value font-semibold">{diag.browserProfileCount}</span>
              </div>
              <div className="diag-row">
                <span className="diag-label">Active Managed Processes:</span>
                <span className="diag-value font-semibold">{diag.activeProcessCount}</span>
              </div>
            </div>
          ) : (
            <div>Failed to load diagnostics data.</div>
          )}
        </div>

        <div className="modal-footer flex items-center justify-between">
          <button
            type="button"
            className="btn btn-secondary flex items-center gap-1"
            onClick={handleExportSupportBundle}
            title="Export sanitized diagnostic bundle with zero secrets"
          >
            <Download size={14} />
            {t("diagCenter.exportBundle")}
          </button>

          <div className="flex items-center gap-2">
            <button
              type="button"
              className="btn btn-secondary flex items-center gap-1"
              onClick={fetchDiag}
              disabled={loading}
            >
              <RefreshCw size={14} className={loading ? "animate-spin" : ""} />
              {t("app.refresh")}
            </button>
            <button type="button" className="btn btn-primary" onClick={onClose}>
              {t("action.close")}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

