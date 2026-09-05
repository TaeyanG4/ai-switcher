import React, { useState } from "react";
import { AlertTriangle, X } from "lucide-react";
import { AccountProfile } from "../types";
import { useI18n } from "../i18n/I18nContext";

interface Props {
  account: AccountProfile | null;
  isOpen: boolean;
  allAccounts?: AccountProfile[];
  onClose: () => void;
  onConfirm: (account: AccountProfile, deleteLocalData: boolean, deleteBrowserProfile?: boolean) => Promise<void>;
}

export const DeleteAccountModal: React.FC<Props> = ({
  account,
  isOpen,
  allAccounts = [],
  onClose,
  onConfirm,
}) => {
  const { t } = useI18n();
  const [deleteLocalData, setDeleteLocalData] = useState(false);
  const [deleteBrowserProfile, setDeleteBrowserProfile] = useState(false);
  const [deleting, setDeleting] = useState(false);

  if (!isOpen || !account) return null;

  const sharedCount = account.browserProfileId
    ? allAccounts.filter((a) => a.browserProfileId === account.browserProfileId).length
    : 0;
  const isBrowserProfileShared = sharedCount > 1;

  const handleConfirm = async () => {
    try {
      setDeleting(true);
      await onConfirm(account, deleteLocalData, deleteBrowserProfile);
      onClose();
    } finally {
      setDeleting(false);
    }
  };

  return (
    <div className="modal-overlay">
      <div className="modal-dialog">
        <div className="modal-header">
          <h3 className="text-danger flex items-center gap-2">
            <AlertTriangle size={18} />
            {t("action.delete")}
          </h3>
          <button className="btn-close" onClick={onClose} disabled={deleting}>
            <X size={16} />
          </button>
        </div>

        <div className="modal-body">
          <p>
            Are you sure you want to remove <strong>{account.displayName}</strong> from AI Switcher?
          </p>

          <div className="delete-options-box" style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            <label className="checkbox-row">
              <input
                type="checkbox"
                checked={deleteLocalData}
                onChange={(e) => setDeleteLocalData(e.target.checked)}
              />
              <div>
                <strong>Also delete local environment data & session tokens</strong>
                <div className="form-hint">
                  Permanently deletes the isolated folder <code>{account.profilePath}</code>.
                  Leave unchecked to keep session data on disk.
                </div>
              </div>
            </label>

            {account.browserProfileId && (
              <>
                {isBrowserProfileShared ? (
                  <div className="alert alert-warning" style={{ fontSize: 11 }}>
                    Linked browser profile is shared by {sharedCount - 1} other account(s) and will be preserved.
                  </div>
                ) : (
                  <label className="checkbox-row">
                    <input
                      type="checkbox"
                      checked={deleteBrowserProfile}
                      onChange={(e) => setDeleteBrowserProfile(e.target.checked)}
                    />
                    <div>
                      <strong>Also delete linked browser profile</strong>
                      <div className="form-hint">
                        Removes the dedicated browser user-data directory and cookies.
                      </div>
                    </div>
                  </label>
                )}
              </>
            )}
          </div>

          <div className="alert alert-warning" style={{ marginTop: 12 }}>
            <strong>Safety Notice:</strong> Deleting a profile from AI Switcher only removes local records. It never deletes or affects your remote OpenAI, Anthropic, or Google account.
          </div>
        </div>

        <div className="modal-footer">
          <button
            type="button"
            className="btn btn-secondary"
            onClick={onClose}
            disabled={deleting}
          >
            {t("action.cancel")}
          </button>
          <button
            type="button"
            className="btn btn-danger"
            onClick={handleConfirm}
            disabled={deleting}
          >
            {deleting ? t("action.saving") : t("action.delete")}
          </button>
        </div>
      </div>
    </div>
  );
};
