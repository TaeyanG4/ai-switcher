import React, { useState } from "react";
import { Folder, MoreVertical, Play, Trash2, EyeOff, Eye, Edit2, Terminal, Globe, RotateCw, LogOut, KeyRound, Star } from "lucide-react";
import { AccountProfile, ExecutionSurface } from "../types";
import { StatusBadge } from "./StatusBadge";
import { useI18n } from "../i18n/I18nContext";

interface Props {
  account: AccountProfile;
  isFavorite?: boolean;
  onToggleFavorite?: (account: AccountProfile) => void;
  onOpen: (account: AccountProfile, surface?: ExecutionSurface) => void;
  onOpenFolder: (account: AccountProfile) => void;
  onToggleEnabled: (account: AccountProfile) => void;
  onDelete: (account: AccountProfile) => void;
  onRename: (account: AccountProfile) => void;
  onCheckStatus?: (account: AccountProfile, surface?: ExecutionSurface) => void;
  onLogout?: (account: AccountProfile) => void;
  onLogin?: (account: AccountProfile) => void;
}

export const AccountCard: React.FC<Props> = ({
  account,
  isFavorite = false,
  onToggleFavorite,
  onOpen,
  onOpenFolder,
  onToggleEnabled,
  onDelete,
  onRename,
  onCheckStatus,
  onLogout,
  onLogin,
}) => {
  const { t } = useI18n();
  const [showMenu, setShowMenu] = useState(false);

  const defaultSurfaceLabel =
    account.platform === "claude"
      ? t("account.launchDesktop")
      : account.platform === "codex"
      ? t("account.launchCodexDesktop")
      : account.platform === "antigravity"
      ? t("account.launchDesktop")
      : t("action.launch");

  const defaultSurfaceTitle =
    account.platform === "codex"
      ? `${t("account.launchCodexDesktop")} (${account.displayName}) - ${t("account.sharedSessionBadge")}`
      : account.platform === "antigravity"
      ? `${t("account.launchDesktop")} (${account.displayName})`
      : `${t("action.launch")} ${account.displayName}`;

  return (
    <div className={`account-card ${!account.isEnabled ? "account-disabled" : ""}`}>
      <div className="account-info">
        <div className="account-title-row">
          {onToggleFavorite && (
            <button
              className={`btn-star ${isFavorite ? "active" : ""}`}
              onClick={(e) => {
                e.stopPropagation();
                onToggleFavorite(account);
              }}
              title={isFavorite ? t("account.unfavorite") : t("account.favorite")}
            >
              <Star
                size={14}
                fill={isFavorite ? "#f59e0b" : "none"}
                stroke={isFavorite ? "#f59e0b" : "currentColor"}
              />
            </button>
          )}
          <span className="account-name">{account.displayName}</span>
          {account.accountIdentifier && (
            <span className="account-identifier">{account.accountIdentifier}</span>
          )}
          {account.platform === "codex" && (
            <span
              className="shared-session-pill"
              title={t("account.sharedSessionTitle")}
            >
              {t("account.sharedSessionBadge")}
            </span>
          )}
          {!account.isEnabled && <span className="disabled-pill">{t("status.disabled")}</span>}
        </div>
        <div className="account-meta">
          <StatusBadge
            status={account.status}
            authStatus={account.authStatus}
            runtimeStatus={account.runtimeStatus}
          />
          <span className="meta-text">
            {t("account.viaMethod", {
              method:
                account.loginMethod === "google"
                  ? t("loginMethod.google")
                  : account.loginMethod === "email"
                  ? t("loginMethod.email")
                  : account.loginMethod === "email_otp"
                  ? t("loginMethod.emailOtp")
                  : account.loginMethod === "phone"
                  ? t("loginMethod.phone")
                  : account.loginMethod === "passkey"
                  ? t("loginMethod.passkey")
                  : account.loginMethod === "oauth"
                  ? t("loginMethod.oauth")
                  : account.loginMethod === "other"
                  ? t("loginMethod.other")
                  : t("loginMethod.unknown"),
            })}
          </span>
          {account.defaultWorkspacePath && (
            <span className="meta-workspace" title={account.defaultWorkspacePath}>
              📁 {account.defaultWorkspacePath.split(/[\\/]/).pop()}
            </span>
          )}
        </div>
      </div>

      <div className="account-actions">
        {account.authStatus === "login_required" && onLogin && (
          <button
            className="btn btn-secondary"
            onClick={() => onLogin(account)}
            disabled={!account.isEnabled}
            title={t("action.login")}
            style={{ color: "var(--status-login)", borderColor: "rgba(245, 158, 11, 0.4)" }}
          >
            <KeyRound size={13} style={{ marginRight: 4 }} />
            {t("action.login")}
          </button>
        )}

        <button
          className="btn btn-primary"
          onClick={() => onOpen(account, "desktop_app")}
          disabled={!account.isEnabled}
          title={defaultSurfaceTitle}
        >
          <Play size={13} style={{ marginRight: 4 }} />
          {defaultSurfaceLabel}
        </button>

        <button
          className="btn btn-icon"
          onClick={() => onOpenFolder(account)}
          disabled={!account.isEnabled}
          title={t("account.defaultWorkspace")}
        >
          <Folder size={14} />
        </button>

        <div className="menu-container">
          <button
            className="btn btn-icon"
            onClick={() => setShowMenu(!showMenu)}
            title={t("account.moreOptions")}
          >
            <MoreVertical size={14} />
          </button>

          {showMenu && (
            <>
              <div className="menu-overlay" onClick={() => setShowMenu(false)} />
              <div className="dropdown-menu">
                {account.platform === "codex" && (
                  <>
                    <button
                      className="dropdown-item"
                      onClick={() => {
                        setShowMenu(false);
                        onOpen(account, "cli");
                      }}
                      title={t("account.launchCodexCliTitle")}
                    >
                      <Terminal size={13} />
                      {t("account.launchCodexCli")}
                    </button>

                    {onCheckStatus && (
                      <>
                        <button
                          className="dropdown-item"
                          onClick={() => {
                            setShowMenu(false);
                            onCheckStatus(account, "desktop_app");
                          }}
                          title={t("account.checkDesktopSessionTitle")}
                        >
                          <RotateCw size={13} />
                          {t("account.checkDesktopSession")}
                        </button>

                        <button
                          className="dropdown-item"
                          onClick={() => {
                            setShowMenu(false);
                            onCheckStatus(account, "cli");
                          }}
                          title={t("account.checkCliSessionTitle")}
                        >
                          <RotateCw size={13} />
                          {t("account.checkCliSession")}
                        </button>
                      </>
                    )}
                  </>
                )}

                {account.platform === "antigravity" && (
                  <button
                    className="dropdown-item"
                    onClick={() => {
                      setShowMenu(false);
                      onOpen(account, "desktop_app");
                    }}
                    title={t("account.launchDesktop")}
                  >
                    <Play size={13} />
                    {t("account.launchDesktop")}
                  </button>
                )}

                {account.platform === "claude" && (
                  <>
                    <button
                      className="dropdown-item"
                      onClick={() => {
                        setShowMenu(false);
                        onOpen(account, "desktop_app");
                      }}
                      title={t("account.launchDesktop")}
                    >
                      <Play size={13} />
                      {t("account.launchDesktop")}
                    </button>

                    <button
                      className="dropdown-item"
                      onClick={() => {
                        setShowMenu(false);
                        onOpen(account, "cli");
                      }}
                      title={t("account.launchCli")}
                    >
                      <Terminal size={13} />
                      {t("account.launchCli")}
                    </button>

                    <button
                      className="dropdown-item"
                      onClick={() => {
                        setShowMenu(false);
                        onOpen(account, "web");
                      }}
                      title={t("account.launchWeb")}
                    >
                      <Globe size={13} />
                      {t("account.launchWeb")}
                    </button>

                    {onCheckStatus && (
                      <>
                        <button
                          className="dropdown-item"
                          onClick={() => {
                            setShowMenu(false);
                            onCheckStatus(account, "desktop_app");
                          }}
                          title={t("account.checkDesktopSessionTitle")}
                        >
                          <RotateCw size={13} />
                          {t("account.checkDesktopSession")}
                        </button>

                        <button
                          className="dropdown-item"
                          onClick={() => {
                            setShowMenu(false);
                            onCheckStatus(account, "cli");
                          }}
                          title={t("account.checkCliSessionTitle")}
                        >
                          <RotateCw size={13} />
                          {t("account.checkCliSession")}
                        </button>
                      </>
                    )}
                  </>
                )}

                {(account.platform === "codex" || account.platform === "claude" || account.platform === "antigravity") && (
                  <div className="dropdown-divider" />
                )}

                {account.isEnabled && onLogin && (
                  <button
                    className="dropdown-item"
                    onClick={() => {
                      setShowMenu(false);
                      onLogin(account);
                    }}
                  >
                    <KeyRound size={13} />
                    {account.authStatus === "login_required" ? t("action.login") : t("account.reauthenticate")}
                  </button>
                )}

                {account.isEnabled && onCheckStatus && account.platform !== "codex" && account.platform !== "claude" && (
                  <button
                    className="dropdown-item"
                    onClick={() => {
                      setShowMenu(false);
                      onCheckStatus(account);
                    }}
                  >
                    <RotateCw size={13} />
                    {t("app.refresh")}
                  </button>
                )}

                {account.isEnabled && onLogout && (
                  <button
                    className="dropdown-item"
                    onClick={() => {
                      setShowMenu(false);
                      onLogout(account);
                    }}
                  >
                    <LogOut size={13} />
                    {t("action.logout")}
                  </button>
                )}

                {onToggleFavorite && (
                  <button
                    className="dropdown-item"
                    onClick={() => {
                      setShowMenu(false);
                      onToggleFavorite(account);
                    }}
                  >
                    <Star
                      size={13}
                      fill={isFavorite ? "#f59e0b" : "none"}
                      stroke={isFavorite ? "#f59e0b" : "currentColor"}
                    />
                    {isFavorite ? t("account.unfavorite") : t("account.favorite")}
                  </button>
                )}

                <button
                  className="dropdown-item"
                  onClick={() => {
                    setShowMenu(false);
                    onRename(account);
                  }}
                >
                  <Edit2 size={13} />
                  {t("action.rename")}
                </button>

                <button
                  className="dropdown-item"
                  onClick={() => {
                    setShowMenu(false);
                    onToggleEnabled(account);
                  }}
                >
                  {account.isEnabled ? (
                    <>
                      <EyeOff size={13} />
                      {t("account.disable")}
                    </>
                  ) : (
                    <>
                      <Eye size={13} />
                      {t("account.enable")}
                    </>
                  )}
                </button>

                <div className="dropdown-divider" />

                <button
                  className="dropdown-item text-danger"
                  onClick={() => {
                    setShowMenu(false);
                    onDelete(account);
                  }}
                >
                  <Trash2 size={13} />
                  {t("action.delete")}
                </button>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
};
