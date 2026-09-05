import React, { useState } from "react";
import { AlertCircle, X } from "lucide-react";
import { AccountProfile, ProcessConflictInfo } from "../types";
import { useI18n } from "../i18n/I18nContext";

interface Props {
  conflict: ProcessConflictInfo | null;
  targetAccount: AccountProfile | null;
  isOpen: boolean;
  onClose: () => void;
  onConfirmCloseAndSwitch: (conflict: ProcessConflictInfo, targetAccount: AccountProfile) => Promise<void>;
}

export const ConflictDialog: React.FC<Props> = ({
  conflict,
  targetAccount,
  isOpen,
  onClose,
  onConfirmCloseAndSwitch,
}) => {
  const { t } = useI18n();
  const [switching, setSwitching] = useState(false);

  if (!isOpen || !conflict || !targetAccount) return null;

  const handleSwitch = async () => {
    try {
      setSwitching(true);
      await onConfirmCloseAndSwitch(conflict, targetAccount);
      onClose();
    } finally {
      setSwitching(false);
    }
  };

  const platformName =
    conflict.platform === "codex"
      ? t("platform.codex")
      : conflict.platform === "claude"
      ? t("platform.claude")
      : t("platform.antigravity");

  return (
    <div className="modal-overlay">
      <div className="modal-dialog">
        <div className="modal-header">
          <h3 className="flex items-center gap-2 text-warning">
            <AlertCircle size={18} />
            {t("conflict.title")}
          </h3>
          <button className="btn-close" onClick={onClose} disabled={switching}>
            <X size={16} />
          </button>
        </div>

        <div className="modal-body">
          <p>
            {t("conflict.desc")}
          </p>

          <div className="delete-options-box" style={{ margin: "8px 0" }}>
            <div>
              <strong>{t("conflict.currentlyActive")}</strong> {conflict.runningDisplayName}
              {conflict.runningPid && (
                <span className="text-muted text-xs" style={{ marginLeft: 6 }}>
                  (PID: {conflict.runningPid})
                </span>
              )}
            </div>
            <div style={{ marginTop: 4 }}>
              <strong>{t("conflict.requestedTarget")}</strong> {targetAccount.displayName}
            </div>
          </div>

          <p className="text-secondary text-xs">
            {t("conflict.confirmSwitch", {
              platform: platformName,
              running: conflict.runningDisplayName,
              target: targetAccount.displayName,
            })}
          </p>
        </div>

        <div className="modal-footer">
          <button
            type="button"
            className="btn btn-secondary"
            onClick={onClose}
            disabled={switching}
          >
            {t("action.cancel")}
          </button>
          <button
            type="button"
            className="btn btn-primary"
            onClick={handleSwitch}
            disabled={switching}
          >
            {switching ? t("action.saving") : t("conflict.force")}
          </button>
        </div>
      </div>
    </div>
  );
};
