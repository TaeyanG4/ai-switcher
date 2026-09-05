import React, { useEffect, useState } from "react";
import { CheckCircle, Globe, Play, Plus, RefreshCw, Trash2, X, XCircle } from "lucide-react";
import {
  createBrowserProfile,
  deleteBrowserProfile,
  detectAvailableBrowsers,
  launchBrowserProfile,
  listBrowserProfiles,
} from "../api";
import { BrowserKind, BrowserProfile, DetectedBrowserInfo } from "../types";

interface Props {
  isOpen: boolean;
  onClose: () => void;
  onNotification?: (text: string, type: "info" | "success" | "error") => void;
}

export const BrowserProfilesModal: React.FC<Props> = ({
  isOpen,
  onClose,
  onNotification,
}) => {
  const [profiles, setProfiles] = useState<BrowserProfile[]>([]);
  const [detectedBrowsers, setDetectedBrowsers] = useState<DetectedBrowserInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  // New Profile Form
  const [showAddForm, setShowAddForm] = useState(false);
  const [newDisplayName, setNewDisplayName] = useState("");
  const [newBrowserKind, setNewBrowserKind] = useState<BrowserKind>("chrome");
  const [newCustomPath, setNewCustomPath] = useState("");
  const [submitting, setSubmitting] = useState(false);

  // Delete State
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [deleteDataFiles, setDeleteDataFiles] = useState(true);

  const loadData = async () => {
    try {
      setLoading(true);
      setActionError(null);
      const [profs, detected] = await Promise.all([
        listBrowserProfiles(),
        detectAvailableBrowsers(),
      ]);
      setProfiles(profs);
      setDetectedBrowsers(detected);
    } catch (err: any) {
      console.error("Failed to load browser profiles:", err);
      setActionError(err?.message || String(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (isOpen) {
      loadData();
      setShowAddForm(false);
      setActionError(null);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newDisplayName.trim()) {
      setActionError("Please provide a display name for the browser profile.");
      return;
    }

    try {
      setSubmitting(true);
      setActionError(null);
      await createBrowserProfile({
        displayName: newDisplayName.trim(),
        browserKind: newBrowserKind,
        customExecutablePath: newCustomPath.trim() || undefined,
      });

      setNewDisplayName("");
      setNewCustomPath("");
      setShowAddForm(false);
      if (onNotification) {
        onNotification("Browser profile created successfully.", "success");
      }
      await loadData();
    } catch (err: any) {
      setActionError(err?.message || String(err));
    } finally {
      setSubmitting(false);
    }
  };

  const handleLaunch = async (id: string, url?: string) => {
    try {
      setActionError(null);
      await launchBrowserProfile(id, url);
      if (onNotification) {
        onNotification("Isolated browser launched.", "info");
      }
      await loadData();
    } catch (err: any) {
      setActionError(err?.message || String(err));
    }
  };

  const handleDelete = async (id: string) => {
    try {
      setActionError(null);
      await deleteBrowserProfile(id, deleteDataFiles);
      setDeletingId(null);
      if (onNotification) {
        onNotification("Browser profile deleted.", "info");
      }
      await loadData();
    } catch (err: any) {
      setActionError(err?.message || String(err));
    }
  };

  return (
    <div className="modal-overlay">
      <div className="modal-dialog modal-large">
        <div className="modal-header">
          <h3 className="flex items-center gap-2">
            <Globe size={18} />
            Isolated Browser Profiles
          </h3>
          <button className="btn-close" onClick={onClose}>
            <X size={16} />
          </button>
        </div>

        <div className="modal-body">
          {actionError && (
            <div className="alert alert-error mb-3" style={{ marginBottom: 12 }}>
              {actionError}
            </div>
          )}

          {/* System Detected Browsers Bar */}
          <div className="detected-browsers-bar">
            <span className="text-xs font-semibold text-muted">Detected Browsers:</span>
            <div className="flex gap-2">
              {detectedBrowsers.map((b) => (
                <span
                  key={b.kind}
                  className={`browser-pill ${b.isAvailable ? "available" : "unavailable"}`}
                  title={b.executablePath || "Not installed"}
                >
                  {b.isAvailable ? <CheckCircle size={12} /> : <XCircle size={12} />}
                  {b.name} {b.version ? `(${b.version})` : ""}
                </span>
              ))}
            </div>
          </div>

          <div className="browser-profiles-section-header">
            <h4>Configured Profiles ({profiles.length})</h4>
            {!showAddForm && (
              <button
                className="btn btn-primary btn-xs flex items-center gap-1"
                onClick={() => setShowAddForm(true)}
              >
                <Plus size={13} /> Add Browser Profile
              </button>
            )}
          </div>

          {/* Add Profile Inline Form */}
          {showAddForm && (
            <form onSubmit={handleCreate} className="browser-create-card">
              <h5>Create Isolated Browser Profile</h5>
              <div className="grid-2-col gap-2">
                <div className="form-group">
                  <label className="form-label text-xs">Display Name *</label>
                  <input
                    type="text"
                    className="form-input text-xs"
                    placeholder="e.g., Work Chrome, Client Brave"
                    value={newDisplayName}
                    onChange={(e) => setNewDisplayName(e.target.value)}
                    required
                    autoFocus
                  />
                </div>

                <div className="form-group">
                  <label className="form-label text-xs">Browser Engine</label>
                  <select
                    className="form-select text-xs"
                    value={newBrowserKind}
                    onChange={(e) => setNewBrowserKind(e.target.value as BrowserKind)}
                  >
                    <option value="chrome">Google Chrome</option>
                    <option value="edge">Microsoft Edge</option>
                    <option value="brave">Brave Browser</option>
                    <option value="custom">Custom Chromium Executable</option>
                  </select>
                </div>
              </div>

              {newBrowserKind === "custom" && (
                <div className="form-group mt-2">
                  <label className="form-label text-xs">Custom Executable Path</label>
                  <input
                    type="text"
                    className="form-input text-xs"
                    placeholder="C:\path\to\chromium.exe"
                    value={newCustomPath}
                    onChange={(e) => setNewCustomPath(e.target.value)}
                    required
                  />
                </div>
              )}

              <div className="flex justify-end gap-2 mt-3">
                <button
                  type="button"
                  className="btn btn-secondary btn-xs"
                  onClick={() => setShowAddForm(false)}
                  disabled={submitting}
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="btn btn-primary btn-xs"
                  disabled={submitting}
                >
                  {submitting ? "Creating..." : "Save Profile"}
                </button>
              </div>
            </form>
          )}

          {/* Profiles List */}
          {loading ? (
            <div className="loading-spinner">Loading browser profiles...</div>
          ) : profiles.length === 0 ? (
            <div className="empty-browser-notice">
              <p>No browser profiles configured yet.</p>
              <span className="text-xs text-muted">
                Create one to provide isolated cookies, authentication sessions, and web environments.
              </span>
            </div>
          ) : (
            <div className="browser-profiles-list">
              {profiles.map((p) => (
                <div key={p.id} className="browser-profile-item">
                  <div className="browser-profile-info">
                    <div className="flex items-center gap-2">
                      <span className="browser-kind-tag">{p.browserKind.toUpperCase()}</span>
                      <strong className="browser-profile-name">{p.displayName}</strong>
                    </div>
                    <div className="browser-profile-path" title={p.userDataDirectory}>
                      {p.userDataDirectory}
                    </div>
                    {p.lastUsedAt && (
                      <div className="browser-last-used text-xs text-muted">
                        Last opened: {new Date(p.lastUsedAt).toLocaleString()}
                      </div>
                    )}
                  </div>

                  <div className="browser-profile-actions">
                    <button
                      className="btn btn-secondary btn-xs flex items-center gap-1"
                      onClick={() => handleLaunch(p.id)}
                      title="Open isolated browser"
                    >
                      <Play size={12} /> Launch
                    </button>

                    <button
                      className="btn btn-secondary btn-xs flex items-center gap-1"
                      onClick={() => handleLaunch(p.id, "https://claude.ai/login")}
                      title="Open Claude Web login in this profile"
                    >
                      <Globe size={12} /> Claude Web
                    </button>

                    {deletingId === p.id ? (
                      <div className="flex items-center gap-2 delete-confirm-box">
                        <label className="text-xs flex items-center gap-1">
                          <input
                            type="checkbox"
                            checked={deleteDataFiles}
                            onChange={(e) => setDeleteDataFiles(e.target.checked)}
                          />
                          Delete Files
                        </label>
                        <button
                          className="btn btn-danger btn-xs"
                          onClick={() => handleDelete(p.id)}
                        >
                          Confirm
                        </button>
                        <button
                          className="btn btn-secondary btn-xs"
                          onClick={() => setDeletingId(null)}
                        >
                          Cancel
                        </button>
                      </div>
                    ) : (
                      <button
                        className="btn btn-icon btn-xs text-danger"
                        onClick={() => {
                          setDeletingId(p.id);
                          setDeleteDataFiles(true);
                        }}
                        title="Delete browser profile"
                      >
                        <Trash2 size={13} />
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="modal-footer">
          <button
            type="button"
            className="btn btn-secondary flex items-center gap-1"
            onClick={loadData}
            disabled={loading}
          >
            <RefreshCw size={14} className={loading ? "animate-spin" : ""} /> Refresh
          </button>
          <button type="button" className="btn btn-primary" onClick={onClose}>
            Done
          </button>
        </div>
      </div>
    </div>
  );
};
