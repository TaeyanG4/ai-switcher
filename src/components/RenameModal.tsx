import React, { useState, useEffect } from "react";
import { X } from "lucide-react";
import { AccountProfile } from "../types";
import { useI18n } from "../i18n/I18nContext";

interface Props {
  account: AccountProfile | null;
  isOpen: boolean;
  onClose: () => void;
  onSave: (id: string, newName: string, newIdentifier?: string) => Promise<void>;
}

export const RenameModal: React.FC<Props> = ({ account, isOpen, onClose, onSave }) => {
  const { t } = useI18n();
  const [name, setName] = useState("");
  const [identifier, setIdentifier] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (account) {
      setName(account.displayName);
      setIdentifier(account.accountIdentifier || "");
    }
  }, [account]);

  if (!isOpen || !account) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;

    try {
      setSaving(true);
      await onSave(account.id, name.trim(), identifier.trim() || undefined);
      onClose();
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="modal-overlay">
      <div className="modal-dialog">
        <div className="modal-header">
          <h3>{t("action.rename")}</h3>
          <button className="btn-close" onClick={onClose} disabled={saving}>
            <X size={16} />
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="modal-body">
            <div className="form-group">
              <label className="form-label">{t("wizard.displayName")} *</label>
              <input
                type="text"
                className="form-input"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
                autoFocus
              />
            </div>

            <div className="form-group">
              <label className="form-label">{t("wizard.emailOrId")}</label>
              <input
                type="text"
                className="form-input"
                value={identifier}
                onChange={(e) => setIdentifier(e.target.value)}
                placeholder="e.g., user@company.com"
              />
            </div>
          </div>

          <div className="modal-footer">
            <button
              type="button"
              className="btn btn-secondary"
              onClick={onClose}
              disabled={saving}
            >
              {t("action.cancel")}
            </button>
            <button type="submit" className="btn btn-primary" disabled={saving}>
              {saving ? t("action.saving") : t("action.save")}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
