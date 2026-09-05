import React from "react";
import { AccountStatus } from "../types";
import { useI18n } from "../i18n/I18nContext";

interface Props {
  status: AccountStatus;
}

export const StatusBadge: React.FC<Props> = ({ status }) => {
  const { t } = useI18n();

  switch (status) {
    case "ready":
      return (
        <span className="status-badge status-ready" title={`Status: ${t("status.ready")}`}>
          <span className="status-dot">●</span> {t("status.ready")}
        </span>
      );
    case "running":
      return (
        <span className="status-badge status-running" title={`Status: ${t("status.running")}`}>
          <span className="status-dot">▶</span> {t("status.running")}
        </span>
      );
    case "login_required":
      return (
        <span className="status-badge status-login-required" title={`Status: ${t("status.loginRequired")}`}>
          <span className="status-dot">○</span> {t("status.loginRequired")}
        </span>
      );
    case "error":
      return (
        <span className="status-badge status-error" title={`Status: ${t("status.error")}`}>
          <span className="status-dot">⚠</span> {t("status.error")}
        </span>
      );
    case "unknown":
    default:
      return (
        <span className="status-badge status-unknown" title={`Status: ${t("status.unknown")}`}>
          <span className="status-dot">?</span> {t("status.unknown")}
        </span>
      );
  }
};
