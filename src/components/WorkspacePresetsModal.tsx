import React, { useEffect, useState } from "react";
import {
  Folder,
  Plus,
  RefreshCw,
  Trash2,
  Edit2,
  X,
  Play,
  Clock,
  FolderGit2,
  AlertCircle,
  Star,
} from "lucide-react";
import {
  createWorkspace,
  createWorkspaceDirectory,
  deleteWorkspace,
  launchWorkspacePreset,
  listAccounts,
  listFavorites,
  listWorkspaces,
  pickDirectory,
  refreshTrayMenu,
  toggleFavoriteWorkspace,
  updateWorkspace,
} from "../api";
import {
  AccountProfile,
  CreateWorkspaceInput,
  PlatformType,
  UpdateWorkspaceInput,
  WorkspacePreset,
} from "../types";

interface Props {
  isOpen: boolean;
  onClose: () => void;
  onNotification?: (text: string, type: "info" | "success" | "error") => void;
}

export const WorkspacePresetsModal: React.FC<Props> = ({
  isOpen,
  onClose,
  onNotification,
}) => {
  const [workspaces, setWorkspaces] = useState<WorkspacePreset[]>([]);
  const [accounts, setAccounts] = useState<AccountProfile[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Form State
  const [isFormOpen, setIsFormOpen] = useState(false);
  const [editingWorkspaceId, setEditingWorkspaceId] = useState<string | null>(null);
  const [formName, setFormName] = useState("");
  const [formPath, setFormPath] = useState("");
  const [formCodexId, setFormCodexId] = useState<string>("");
  const [formClaudeId, setFormClaudeId] = useState<string>("");
  const [formAntigravityId, setFormAntigravityId] = useState<string>("");
  const [saving, setSaving] = useState(false);

  // Deleting State
  const [deletingId, setDeletingId] = useState<string | null>(null);

  // Launching State
  const [launchingKey, setLaunchingKey] = useState<string | null>(null);

  // Missing Directory State
  const [missingDirState, setMissingDirState] = useState<{
    workspace: WorkspacePreset;
    platform: PlatformType;
    path: string;
  } | null>(null);

  // Favorites State
  const [favoriteWorkspaceIds, setFavoriteWorkspaceIds] = useState<Set<string>>(new Set());

  const loadData = async () => {
    try {
      setLoading(true);
      setError(null);
      const [wsList, accList, favList] = await Promise.all([
        listWorkspaces(),
        listAccounts(true),
        listFavorites(),
      ]);
      setWorkspaces(wsList);
      setAccounts(accList);
      const favWs = new Set(
        favList
          .filter((f) => f.targetType === "workspace_platform" && f.workspaceId)
          .map((f) => f.workspaceId!)
      );
      setFavoriteWorkspaceIds(favWs);
    } catch (err: any) {
      console.error("Failed to load workspaces:", err);
      setError(err?.message || String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleToggleFavorite = async (wsId: string) => {
    try {
      await toggleFavoriteWorkspace(wsId);
      const nextFavs = new Set(favoriteWorkspaceIds);
      if (nextFavs.has(wsId)) {
        nextFavs.delete(wsId);
      } else {
        nextFavs.add(wsId);
      }
      setFavoriteWorkspaceIds(nextFavs);
      await refreshTrayMenu();
    } catch (err: any) {
      console.error("Failed to toggle favorite workspace:", err);
    }
  };

  useEffect(() => {
    if (isOpen) {
      loadData();
      setIsFormOpen(false);
      setEditingWorkspaceId(null);
      setError(null);
      setDeletingId(null);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const codexAccounts = accounts.filter((a) => a.platform === "codex");
  const claudeAccounts = accounts.filter((a) => a.platform === "claude");
  const antigravityAccounts = accounts.filter((a) => a.platform === "antigravity");

  const handleOpenAddForm = () => {
    setEditingWorkspaceId(null);
    setFormName("");
    setFormPath("");
    setFormCodexId("");
    setFormClaudeId("");
    setFormAntigravityId("");
    setIsFormOpen(true);
    setError(null);
  };

  const handleOpenEditForm = (ws: WorkspacePreset) => {
    setEditingWorkspaceId(ws.id);
    setFormName(ws.name);
    setFormPath(ws.directoryPath);
    setFormCodexId(ws.preferredCodexAccountId || "");
    setFormClaudeId(ws.preferredClaudeAccountId || "");
    setFormAntigravityId(ws.preferredAntigravityAccountId || "");
    setIsFormOpen(true);
    setError(null);
  };

  const handleSaveForm = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!formName.trim()) {
      setError("Please provide a name for the workspace preset.");
      return;
    }
    if (!formPath.trim()) {
      setError("Please provide a directory path for the workspace preset.");
      return;
    }

    try {
      setSaving(true);
      setError(null);

      if (editingWorkspaceId) {
        const updateInput: UpdateWorkspaceInput = {
          id: editingWorkspaceId,
          name: formName.trim(),
          directoryPath: formPath.trim(),
          preferredCodexAccountId: formCodexId || null,
          preferredClaudeAccountId: formClaudeId || null,
          preferredAntigravityAccountId: formAntigravityId || null,
        };
        await updateWorkspace(updateInput);
        onNotification?.(`Updated workspace preset "${formName.trim()}".`, "success");
      } else {
        const createInput: CreateWorkspaceInput = {
          name: formName.trim(),
          directoryPath: formPath.trim(),
          preferredCodexAccountId: formCodexId || null,
          preferredClaudeAccountId: formClaudeId || null,
          preferredAntigravityAccountId: formAntigravityId || null,
        };
        await createWorkspace(createInput);
        onNotification?.(`Created workspace preset "${formName.trim()}".`, "success");
      }

      setIsFormOpen(false);
      await loadData();
    } catch (err: any) {
      console.error("Failed to save workspace preset:", err);
      setError(err?.message || String(err));
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string, name: string) => {
    try {
      setError(null);
      await deleteWorkspace(id);
      setDeletingId(null);
      onNotification?.(`Deleted workspace preset "${name}".`, "info");
      await loadData();
    } catch (err: any) {
      console.error("Failed to delete workspace preset:", err);
      setError(err?.message || String(err));
    }
  };

  const handleLaunch = async (ws: WorkspacePreset, platform: PlatformType) => {
    const key = `${ws.id}-${platform}`;
    try {
      setLaunchingKey(key);
      setError(null);
      const record = await launchWorkspacePreset(ws.id, platform);
      const platformLabel =
        platform === "codex"
          ? "OpenAI Codex"
          : platform === "claude"
          ? "Anthropic Claude"
          : "Google Antigravity";
      onNotification?.(
        `Launched ${platformLabel} in "${ws.name}" (PID: ${record.pid || "desktop app"}).`,
        "success"
      );
      // Reload to update last opened timestamp
      await loadData();
    } catch (err: any) {
      const errMsg = err?.message || (typeof err === "string" ? err : JSON.stringify(err));
      if (
        errMsg.includes("Workspace directory not found") ||
        err?.type === "WorkspaceDirectoryNotFound"
      ) {
        setMissingDirState({
          workspace: ws,
          platform,
          path: ws.directoryPath,
        });
        return;
      }
      console.error("Failed to launch workspace preset:", err);
      setError(errMsg);
      onNotification?.(`Launch failed: ${errMsg}`, "error");
    } finally {
      setLaunchingKey(null);
    }
  };

  const formatTimestamp = (ts?: string | null) => {
    if (!ts) return "Never opened";
    try {
      const d = new Date(ts);
      return d.toLocaleDateString() + " " + d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    } catch {
      return ts;
    }
  };

  const getAccountName = (id?: string | null) => {
    if (!id) return null;
    const acc = accounts.find((a) => a.id === id);
    return acc ? acc.displayName : null;
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div
        className="modal-dialog"
        style={{ maxWidth: 680, width: "95%" }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="modal-header">
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <div
              style={{
                width: 32,
                height: 32,
                borderRadius: 6,
                backgroundColor: "rgba(59, 130, 246, 0.12)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                color: "var(--accent-primary)",
              }}
            >
              <FolderGit2 size={18} />
            </div>
            <div>
              <h3 style={{ margin: 0, fontSize: 16 }}>Workspace Presets</h3>
              <p
                style={{
                  margin: 0,
                  fontSize: 11,
                  color: "var(--text-muted)",
                }}
              >
                Preconfigure project folders with preferred agent accounts for 1-click launch
              </p>
            </div>
          </div>
          <button className="btn-icon" onClick={onClose} title="Close">
            <X size={16} />
          </button>
        </div>

        {/* Modal Body */}
        <div className="modal-body" style={{ padding: 16, maxHeight: "75vh", overflowY: "auto" }}>
          {error && (
            <div
              style={{
                backgroundColor: "rgba(239, 68, 68, 0.1)",
                border: "1px solid rgba(239, 68, 68, 0.3)",
                color: "#ef4444",
                padding: "8px 12px",
                borderRadius: 6,
                marginBottom: 14,
                fontSize: 12,
                display: "flex",
                alignItems: "center",
                gap: 8,
              }}
            >
              <AlertCircle size={15} style={{ flexShrink: 0 }} />
              <div style={{ flex: 1 }}>{error}</div>
              <button
                className="btn-icon"
                onClick={() => setError(null)}
                style={{ color: "#ef4444", padding: 2 }}
              >
                <X size={14} />
              </button>
            </div>
          )}

          {/* Action Toolbar */}
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              marginBottom: 12,
            }}
          >
            <span style={{ fontSize: 12, color: "var(--text-secondary)", fontWeight: 500 }}>
              {workspaces.length} Preset{workspaces.length === 1 ? "" : "s"} Saved
            </span>
            <div style={{ display: "flex", gap: 8 }}>
              <button
                className="btn btn-secondary btn-sm"
                onClick={loadData}
                disabled={loading}
                title="Refresh Presets"
              >
                <RefreshCw size={13} className={loading ? "animate-spin" : ""} />
                Refresh
              </button>
              {!isFormOpen && (
                <button className="btn btn-primary btn-sm" onClick={handleOpenAddForm}>
                  <Plus size={13} />
                  Add Preset
                </button>
              )}
            </div>
          </div>

          {/* Create / Edit Form Card */}
          {isFormOpen && (
            <form
              onSubmit={handleSaveForm}
              style={{
                backgroundColor: "var(--bg-primary)",
                border: "1px solid var(--accent-primary)",
                borderRadius: 6,
                padding: 14,
                marginBottom: 16,
              }}
            >
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  marginBottom: 12,
                }}
              >
                <h5 style={{ margin: 0, fontSize: 13, fontWeight: 600 }}>
                  {editingWorkspaceId ? "Edit Workspace Preset" : "Create New Workspace Preset"}
                </h5>
                <button
                  type="button"
                  className="btn-icon"
                  onClick={() => setIsFormOpen(false)}
                  title="Cancel"
                >
                  <X size={14} />
                </button>
              </div>

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10, marginBottom: 12 }}>
                <div className="form-group" style={{ margin: 0 }}>
                  <label style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)" }}>
                    Preset Name *
                  </label>
                  <input
                    type="text"
                    className="form-input"
                    value={formName}
                    onChange={(e) => setFormName(e.target.value)}
                    placeholder="e.g. Forge Engine"
                    required
                    style={{ marginTop: 4, fontSize: 12 }}
                  />
                </div>

                <div className="form-group" style={{ margin: 0 }}>
                  <label style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)" }}>
                    Project Directory Path *
                  </label>
                  <div style={{ display: "flex", gap: 6, alignItems: "center", marginTop: 4 }}>
                    <input
                      type="text"
                      className="form-input"
                      value={formPath}
                      onChange={(e) => setFormPath(e.target.value)}
                      placeholder="e.g. H:\dev\forge-engine"
                      required
                      style={{ flex: 1, fontSize: 12 }}
                    />
                    <button
                      type="button"
                      className="btn btn-secondary btn-sm"
                      onClick={async () => {
                        const selected = await pickDirectory(formPath || undefined);
                        if (selected) {
                          setFormPath(selected);
                        }
                      }}
                      style={{ flexShrink: 0, padding: "5px 10px", whiteSpace: "nowrap" }}
                    >
                      <Folder size={12} style={{ marginRight: 4 }} />
                      Browse...
                    </button>
                  </div>
                </div>
              </div>

              {/* Preferred Agent Accounts Mapping */}
              <div style={{ marginTop: 8, marginBottom: 14 }}>
                <div
                  style={{
                    fontSize: 11,
                    fontWeight: 600,
                    color: "var(--text-secondary)",
                    marginBottom: 6,
                  }}
                >
                  Preferred Agent Profiles (Optional)
                </div>
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8 }}>
                  {/* Codex Account */}
                  <div>
                    <label style={{ fontSize: 10, color: "var(--text-muted)", display: "block", marginBottom: 3 }}>
                      OpenAI Codex
                    </label>
                    <select
                      className="form-input"
                      value={formCodexId}
                      onChange={(e) => setFormCodexId(e.target.value)}
                      style={{ fontSize: 11 }}
                    >
                      <option value="">(Default fallback)</option>
                      {codexAccounts.map((a) => (
                        <option key={a.id} value={a.id}>
                          {a.displayName} {a.isEnabled ? "" : "(Disabled)"}
                        </option>
                      ))}
                    </select>
                  </div>

                  {/* Claude Account */}
                  <div>
                    <label style={{ fontSize: 10, color: "var(--text-muted)", display: "block", marginBottom: 3 }}>
                      Anthropic Claude
                    </label>
                    <select
                      className="form-input"
                      value={formClaudeId}
                      onChange={(e) => setFormClaudeId(e.target.value)}
                      style={{ fontSize: 11 }}
                    >
                      <option value="">(Default fallback)</option>
                      {claudeAccounts.map((a) => (
                        <option key={a.id} value={a.id}>
                          {a.displayName} {a.isEnabled ? "" : "(Disabled)"}
                        </option>
                      ))}
                    </select>
                  </div>

                  {/* Antigravity Account */}
                  <div>
                    <label style={{ fontSize: 10, color: "var(--text-muted)", display: "block", marginBottom: 3 }}>
                      Google Antigravity
                    </label>
                    <select
                      className="form-input"
                      value={formAntigravityId}
                      onChange={(e) => setFormAntigravityId(e.target.value)}
                      style={{ fontSize: 11 }}
                    >
                      <option value="">(Default fallback)</option>
                      {antigravityAccounts.map((a) => (
                        <option key={a.id} value={a.id}>
                          {a.displayName} {a.isEnabled ? "" : "(Disabled)"}
                        </option>
                      ))}
                    </select>
                  </div>
                </div>
              </div>

              <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
                <button
                  type="button"
                  className="btn btn-secondary btn-sm"
                  onClick={() => setIsFormOpen(false)}
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="btn btn-primary btn-sm"
                  disabled={saving || !formName.trim() || !formPath.trim()}
                >
                  {saving ? (
                    <>
                      <RefreshCw size={12} className="animate-spin" />
                      Saving...
                    </>
                  ) : editingWorkspaceId ? (
                    "Update Preset"
                  ) : (
                    "Save Preset"
                  )}
                </button>
              </div>
            </form>
          )}

          {/* Preset Cards List */}
          {workspaces.length === 0 ? (
            <div
              style={{
                textAlign: "center",
                padding: "36px 16px",
                border: "1px dashed var(--border-color)",
                borderRadius: 6,
                color: "var(--text-muted)",
              }}
            >
              <Folder size={32} style={{ margin: "0 auto 8px auto", opacity: 0.5 }} />
              <div style={{ fontSize: 13, fontWeight: 500, color: "var(--text-secondary)" }}>
                No workspace presets configured yet
              </div>
              <p style={{ fontSize: 11, marginTop: 4, marginBottom: 12 }}>
                Create presets for your active repositories to launch Codex, Claude, and Antigravity
                with one click.
              </p>
              <button className="btn btn-primary btn-sm" onClick={handleOpenAddForm}>
                <Plus size={13} />
                Create Your First Preset
              </button>
            </div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
              {workspaces.map((ws) => {
                const codexName = getAccountName(ws.preferredCodexAccountId);
                const claudeName = getAccountName(ws.preferredClaudeAccountId);
                const antigravityName = getAccountName(ws.preferredAntigravityAccountId);

                const isDeleting = deletingId === ws.id;

                return (
                  <div
                    key={ws.id}
                    style={{
                      backgroundColor: "var(--bg-primary)",
                      border: "1px solid var(--border-color)",
                      borderRadius: 6,
                      padding: 12,
                      display: "flex",
                      flexDirection: "column",
                      gap: 8,
                      transition: "border-color 0.15s",
                    }}
                  >
                    {/* Header Row: Title & Actions */}
                    <div
                      style={{
                        display: "flex",
                        justifyContent: "space-between",
                        alignItems: "center",
                      }}
                    >
                      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                        <button
                          className={`btn-star ${favoriteWorkspaceIds.has(ws.id) ? "active" : ""}`}
                          onClick={() => handleToggleFavorite(ws.id)}
                          title={favoriteWorkspaceIds.has(ws.id) ? "Remove from Favorites" : "Add to Favorites"}
                          style={{
                            background: "none",
                            border: "none",
                            cursor: "pointer",
                            padding: "2px 4px",
                            display: "flex",
                            alignItems: "center",
                          }}
                        >
                          <Star
                            size={14}
                            fill={favoriteWorkspaceIds.has(ws.id) ? "#f59e0b" : "none"}
                            stroke={favoriteWorkspaceIds.has(ws.id) ? "#f59e0b" : "currentColor"}
                          />
                        </button>
                        <span style={{ fontSize: 14, fontWeight: 600, color: "var(--text-primary)" }}>
                          {ws.name}
                        </span>
                        <span
                          style={{
                            fontSize: 10,
                            color: "var(--text-muted)",
                            display: "flex",
                            alignItems: "center",
                            gap: 3,
                          }}
                        >
                          <Clock size={11} />
                          {formatTimestamp(ws.lastOpenedAt)}
                        </span>
                      </div>

                      <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                        {isDeleting ? (
                          <div
                            style={{
                              display: "flex",
                              alignItems: "center",
                              gap: 6,
                              backgroundColor: "rgba(239, 68, 68, 0.1)",
                              padding: "2px 6px",
                              borderRadius: 4,
                            }}
                          >
                            <span style={{ fontSize: 11, color: "#ef4444" }}>Confirm?</span>
                            <button
                              className="btn btn-danger btn-sm"
                              style={{ padding: "2px 6px", fontSize: 10 }}
                              onClick={() => handleDelete(ws.id, ws.name)}
                            >
                              Delete
                            </button>
                            <button
                              className="btn btn-secondary btn-sm"
                              style={{ padding: "2px 6px", fontSize: 10 }}
                              onClick={() => setDeletingId(null)}
                            >
                              Cancel
                            </button>
                          </div>
                        ) : (
                          <>
                            <button
                              className="btn-icon"
                              onClick={() => handleOpenEditForm(ws)}
                              title="Edit Preset"
                              style={{ padding: 4 }}
                            >
                              <Edit2 size={13} />
                            </button>
                            <button
                              className="btn-icon"
                              onClick={() => setDeletingId(ws.id)}
                              title="Delete Preset"
                              style={{ padding: 4, color: "var(--text-muted)" }}
                            >
                              <Trash2 size={13} />
                            </button>
                          </>
                        )}
                      </div>
                    </div>

                    {/* Directory Path */}
                    <div
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: 6,
                        fontFamily: "var(--font-mono)",
                        fontSize: 11,
                        color: "var(--text-muted)",
                        backgroundColor: "var(--bg-secondary)",
                        padding: "4px 8px",
                        borderRadius: 4,
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                      title={ws.directoryPath}
                    >
                      <Folder size={12} style={{ flexShrink: 0, color: "var(--text-secondary)" }} />
                      <span style={{ overflow: "hidden", textOverflow: "ellipsis" }}>
                        {ws.directoryPath}
                      </span>
                    </div>

                    {/* Mappings Badges */}
                    <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
                      <span
                        style={{
                          fontSize: 10,
                          padding: "2px 6px",
                          borderRadius: 4,
                          backgroundColor: codexName
                            ? "rgba(16, 185, 129, 0.12)"
                            : "var(--bg-secondary)",
                          color: codexName ? "var(--status-ready)" : "var(--text-muted)",
                          border: "1px solid var(--border-color)",
                        }}
                      >
                        Codex: <strong>{codexName || "Fallback"}</strong>
                      </span>

                      <span
                        style={{
                          fontSize: 10,
                          padding: "2px 6px",
                          borderRadius: 4,
                          backgroundColor: claudeName
                            ? "rgba(245, 158, 11, 0.12)"
                            : "var(--bg-secondary)",
                          color: claudeName ? "var(--status-login)" : "var(--text-muted)",
                          border: "1px solid var(--border-color)",
                        }}
                      >
                        Claude: <strong>{claudeName || "Fallback"}</strong>
                      </span>

                      <span
                        style={{
                          fontSize: 10,
                          padding: "2px 6px",
                          borderRadius: 4,
                          backgroundColor: antigravityName
                            ? "rgba(59, 130, 246, 0.12)"
                            : "var(--bg-secondary)",
                          color: antigravityName ? "var(--accent-primary)" : "var(--text-muted)",
                          border: "1px solid var(--border-color)",
                        }}
                      >
                        Antigravity: <strong>{antigravityName || "Fallback"}</strong>
                      </span>
                    </div>

                    {/* Launch Buttons */}
                    <div
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: 6,
                        marginTop: 4,
                        paddingTop: 8,
                        borderTop: "1px solid var(--border-subtle)",
                      }}
                    >
                      {/* Launch Codex */}
                      <button
                        className="btn btn-secondary btn-sm"
                        style={{ flex: 1, fontSize: 11, gap: 5 }}
                        disabled={launchingKey !== null || codexAccounts.length === 0}
                        onClick={() => handleLaunch(ws, "codex")}
                        title={
                          codexAccounts.length === 0
                            ? "No Codex accounts configured"
                            : `Launch Codex in ${ws.name}`
                        }
                      >
                        {launchingKey === `${ws.id}-codex` ? (
                          <RefreshCw size={11} className="animate-spin" />
                        ) : (
                          <Play size={11} />
                        )}
                        Open Codex
                      </button>

                      {/* Launch Claude */}
                      <button
                        className="btn btn-secondary btn-sm"
                        style={{ flex: 1, fontSize: 11, gap: 5 }}
                        disabled={launchingKey !== null || claudeAccounts.length === 0}
                        onClick={() => handleLaunch(ws, "claude")}
                        title={
                          claudeAccounts.length === 0
                            ? "No Claude accounts configured"
                            : `Launch Claude Desktop in ${ws.name}`
                        }
                      >
                        {launchingKey === `${ws.id}-claude` ? (
                          <RefreshCw size={11} className="animate-spin" />
                        ) : (
                          <Play size={11} />
                        )}
                        Open Claude
                      </button>

                      {/* Launch Antigravity */}
                      <button
                        className="btn btn-secondary btn-sm"
                        style={{ flex: 1, fontSize: 11, gap: 5 }}
                        disabled={launchingKey !== null || antigravityAccounts.length === 0}
                        onClick={() => handleLaunch(ws, "antigravity")}
                        title={
                          antigravityAccounts.length === 0
                            ? "No Antigravity accounts configured"
                            : `Launch Antigravity in ${ws.name}`
                        }
                      >
                        {launchingKey === `${ws.id}-antigravity` ? (
                          <RefreshCw size={11} className="animate-spin" />
                        ) : (
                          <Play size={11} />
                        )}
                        Open Antigravity
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>

      {/* Missing Directory Dialog */}
      {missingDirState && (
        <div
          className="modal-backdrop"
          style={{ zIndex: 1200 }}
          onClick={() => setMissingDirState(null)}
        >
          <div
            className="modal-dialog"
            style={{ maxWidth: 480, width: "90%" }}
            onClick={(e) => e.stopPropagation()}
          >
            <div className="modal-header">
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <AlertCircle size={18} style={{ color: "var(--status-login)" }} />
                <h3 style={{ margin: 0, fontSize: 15, color: "var(--text-primary)" }}>
                  Workspace Folder Not Found
                </h3>
              </div>
              <button
                className="btn-icon"
                onClick={() => setMissingDirState(null)}
                title="Close"
              >
                <X size={14} />
              </button>
            </div>

            <div className="modal-body" style={{ padding: 16 }}>
              <p style={{ margin: 0, fontSize: 13, color: "var(--text-primary)" }}>
                The directory for workspace <strong>"{missingDirState.workspace.name}"</strong> does not exist on disk:
              </p>
              <div
                style={{
                  marginTop: 10,
                  padding: "8px 12px",
                  backgroundColor: "var(--bg-secondary)",
                  borderRadius: 4,
                  fontFamily: "var(--font-mono)",
                  fontSize: 12,
                  color: "var(--text-muted)",
                  wordBreak: "break-all",
                }}
              >
                {missingDirState.path}
              </div>
              <p style={{ marginTop: 12, marginBottom: 0, fontSize: 12, color: "var(--text-secondary)" }}>
                Creating the folder requires explicit action. You can locate an existing folder on your computer or create this folder now.
              </p>
            </div>

            <div
              className="modal-footer"
              style={{
                display: "flex",
                justifyContent: "flex-end",
                gap: 8,
                padding: "12px 16px",
                borderTop: "1px solid var(--border-subtle)",
              }}
            >
              <button
                type="button"
                className="btn btn-secondary btn-sm"
                onClick={() => setMissingDirState(null)}
              >
                Cancel
              </button>

              <button
                type="button"
                className="btn btn-secondary btn-sm"
                onClick={async () => {
                  const ws = missingDirState.workspace;
                  const platform = missingDirState.platform;
                  const selected = await pickDirectory(ws.directoryPath);
                  if (selected) {
                    try {
                      await updateWorkspace({ id: ws.id, directoryPath: selected });
                      setMissingDirState(null);
                      onNotification?.(`Updated workspace directory to "${selected}".`, "info");
                      await loadData();
                      await handleLaunch({ ...ws, directoryPath: selected }, platform);
                    } catch (e: any) {
                      setError(e?.message || String(e));
                    }
                  }
                }}
              >
                <Folder size={12} style={{ marginRight: 4 }} />
                Locate Folder
              </button>

              <button
                type="button"
                className="btn btn-primary btn-sm"
                onClick={async () => {
                  const ws = missingDirState.workspace;
                  const platform = missingDirState.platform;
                  try {
                    await createWorkspaceDirectory(ws.id);
                    setMissingDirState(null);
                    onNotification?.(`Created workspace folder "${ws.directoryPath}".`, "success");
                    await handleLaunch(ws, platform);
                  } catch (e: any) {
                    setError(e?.message || String(e));
                  }
                }}
              >
                Create Folder
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
