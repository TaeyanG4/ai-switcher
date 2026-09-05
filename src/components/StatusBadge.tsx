import React from "react";
import { AccountStatus, AuthStatus, RuntimeStatus } from "../types";
import { useI18n } from "../i18n/I18nContext";

interface Props {
  status?: AccountStatus;
  authStatus?: AuthStatus;
  runtimeStatus?: RuntimeStatus;
}

export const StatusBadge: React.FC<Props> = ({ status, authStatus, runtimeStatus }) => {
  const { t } = useI18n();

  const effectiveAuth = authStatus ? (
    authStatus === 'authenticated' ? 'ready' :
    authStatus === 'login_required' ? 'login_required' :
    authStatus === 'error' ? 'error' : 'unknown'
  ) : (status || 'unknown');

  const isRunning = runtimeStatus === 'running' || status === 'running';

  return (
    <div style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
      {isRunning && (
        <span className="status-badge status-running" title={`Runtime: ${t("status.running")}`}>
          <span className="status-dot">▶</span> {t("status.running")}
        </span>
      )}
      {effectiveAuth === "ready" && (
        <span className="status-badge status-ready" title={`Auth: ${t("status.ready")}`}>
          <span className="status-dot">●</span> {t("status.ready")}
        </span>
      )}
      {effectiveAuth === "login_required" && (
        <span className="status-badge status-login-required" title={`Auth: ${t("status.loginRequired")}`}>
          <span className="status-dot">○</span> {t("status.loginRequired")}
        </span>
      )}
      {effectiveAuth === "error" && (
        <span className="status-badge status-error" title={`Auth: ${t("status.error")}`}>
          <span className="status-dot">⚠</span> {t("status.error")}
        </span>
      )}
      {effectiveAuth === "unknown" && !isRunning && (
        <span className="status-badge status-unknown" title={`Auth: ${t("status.unknown")}`}>
          <span className="status-dot">?</span> {t("status.unknown")}
        </span>
      )}
    </div>
  );
};

