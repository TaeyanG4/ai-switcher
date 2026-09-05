import React, { useState } from "react";
import { Info } from "lucide-react";
import { useI18n } from "../i18n/I18nContext";

interface Props {
  isOpen: boolean;
  onConfirm: (dontShowAgain: boolean) => void;
}

export const FirstCloseDialog: React.FC<Props> = ({ isOpen, onConfirm }) => {
  const { t } = useI18n();
  const [dontShowAgain, setDontShowAgain] = useState(true);

  if (!isOpen) return null;

  return (
    <div className="modal-overlay" style={{ zIndex: 9999 }}>
      <div
        className="modal-content"
        style={{ maxWidth: 440, padding: "24px 28px" }}
        onClick={(e) => e.stopPropagation()}
      >
        <div style={{ display: "flex", gap: 14, alignItems: "flex-start" }}>
          <div
            style={{
              width: 40,
              height: 40,
              borderRadius: "50%",
              background: "rgba(59, 130, 246, 0.15)",
              color: "var(--accent-primary, #3b82f6)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              flexShrink: 0,
            }}
          >
            <Info size={22} />
          </div>
          <div>
            <h3 style={{ margin: "0 0 8px 0", fontSize: 17, fontWeight: 600 }}>
              {t("firstClose.title")}
            </h3>
            <p
              style={{
                margin: "0 0 14px 0",
                fontSize: 13,
                lineHeight: 1.5,
                color: "var(--text-secondary, #94a3b8)",
              }}
            >
              {t("firstClose.desc")}
            </p>
            <label
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                fontSize: 13,
                cursor: "pointer",
                userSelect: "none",
                marginBottom: 20,
              }}
            >
              <input
                type="checkbox"
                checked={dontShowAgain}
                onChange={(e) => setDontShowAgain(e.target.checked)}
              />
              <span>{t("firstClose.dontShowAgain")}</span>
            </label>
            <div style={{ display: "flex", justifyContent: "flex-end" }}>
              <button
                className="btn btn-primary"
                onClick={() => onConfirm(dontShowAgain)}
                style={{ minWidth: 100 }}
              >
                {t("firstClose.gotIt")}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
