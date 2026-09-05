import React from "react";
import { AccountProfile, ExecutionSurface, PlatformType } from "../types";
import { AccountCard } from "./AccountCard";
import { useI18n } from "../i18n/I18nContext";

interface Props {
  platform: PlatformType;
  title: string;
  accounts: AccountProfile[];
  favoriteAccountIds?: Set<string>;
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

export const PlatformSection: React.FC<Props> = ({
  platform,
  title,
  accounts,
  favoriteAccountIds,
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

  return (
    <section className="platform-section" data-platform={platform}>
      <div className="platform-header">
        <h2 className="platform-title">
          {title}
          <span className="account-count-badge">{accounts.length}</span>
        </h2>
      </div>

      <div className="platform-cards">
        {accounts.length === 0 ? (
          <div className="empty-state">{t("platform.noAccounts")}</div>
        ) : (
          accounts.map((acc) => (
            <AccountCard
              key={acc.id}
              account={acc}
              isFavorite={favoriteAccountIds?.has(acc.id) ?? false}
              onToggleFavorite={onToggleFavorite}
              onOpen={onOpen}
              onOpenFolder={onOpenFolder}
              onToggleEnabled={onToggleEnabled}
              onDelete={onDelete}
              onRename={onRename}
              onCheckStatus={onCheckStatus}
              onLogout={onLogout}
              onLogin={onLogin}
            />
          ))
        )}
      </div>
    </section>
  );
};
