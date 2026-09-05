import React, { useEffect, useState } from "react";
import {
  AlertCircle,
  AlertTriangle,
  ArrowLeft,
  ArrowRight,
  Check,
  CheckCircle2,
  Folder,
  Play,
  RefreshCw,
  Shield,
  X,
} from "lucide-react";
import {
  checkAccountStatus,
  cleanupDraftAccount,
  createAccount,
  createBrowserProfile,
  listBrowserProfiles,
  listPlatformCapabilities,
  pickDirectory,
  startLoginFlow,
} from "../api";
import {
  AccountProfile,
  AccountStatus,
  AuthFlowStartResult,
  BrowserKind,
  BrowserProfile,
  LoginMethod,
  PlatformCapabilities,
  PlatformType,
  WizardStep,
} from "../types";
import { StatusBadge } from "./StatusBadge";

interface Props {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => Promise<void>;
  onOpenAccount?: (account: AccountProfile) => void;
}

export const AccountWizardModal: React.FC<Props> = ({
  isOpen,
  onClose,
  onSuccess,
  onOpenAccount,
}) => {
  // Stepper State
  const [step, setStep] = useState<WizardStep>(1);
  const [capabilities, setCapabilities] = useState<PlatformCapabilities[]>([]);
  const [browserProfiles, setBrowserProfiles] = useState<BrowserProfile[]>([]);
  const [loadingInitial, setLoadingInitial] = useState(false);

  // Step 1: Platform
  const [platform, setPlatform] = useState<PlatformType>("antigravity");

  // Step 2: Account Details
  const [displayName, setDisplayName] = useState("");
  const [accountIdentifier, setAccountIdentifier] = useState("");
  const [loginMethod, setLoginMethod] = useState<LoginMethod>("google");
  const [defaultWorkspacePath, setDefaultWorkspacePath] = useState("");

  // Step 3: Browser Environment
  const [browserMode, setBrowserMode] = useState<"new" | "existing" | "none">("new");
  const [newBrowserName, setNewBrowserName] = useState("");
  const [newBrowserKind, setNewBrowserKind] = useState<BrowserKind>("chrome");
  const [selectedBrowserProfileId, setSelectedBrowserProfileId] = useState<string>("");

  // Step 4+: Scaffolding & Flow State
  const [createdAccount, setCreatedAccount] = useState<AccountProfile | null>(null);
  const [scaffolding, setScaffolding] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Step 5: Official Authentication
  const [isLaunchingAuth, setIsLaunchingAuth] = useState(false);
  const [authLaunched, setAuthLaunched] = useState(false);
  const [authFlowResult, setAuthFlowResult] = useState<AuthFlowStartResult | null>(null);

  // Step 6: Verification
  const [isVerifying, setIsVerifying] = useState(false);
  const [verificationStatus, setVerificationStatus] = useState<AccountStatus | null>(null);

  // Cancellation prompt
  const [showCancelConfirm, setShowCancelConfirm] = useState(false);
  const [isCleaningUp, setIsCleaningUp] = useState(false);

  // Load capabilities & browser profiles when modal opens
  useEffect(() => {
    if (isOpen) {
      setLoadingInitial(true);
      Promise.all([listPlatformCapabilities(), listBrowserProfiles()])
        .then(([caps, profiles]) => {
          setCapabilities(caps);
          setBrowserProfiles(profiles);
        })
        .catch((err) => {
          console.error("Failed to load capabilities or browser profiles:", err);
        })
        .finally(() => setLoadingInitial(false));
    } else {
      resetForm();
    }
  }, [isOpen]);

  const resetForm = () => {
    setStep(1);
    setPlatform("antigravity");
    setDisplayName("");
    setAccountIdentifier("");
    setLoginMethod("google");
    setDefaultWorkspacePath("");
    setBrowserMode("new");
    setNewBrowserName("");
    setNewBrowserKind("chrome");
    setSelectedBrowserProfileId("");
    setCreatedAccount(null);
    setScaffolding(false);
    setError(null);
    setIsLaunchingAuth(false);
    setAuthLaunched(false);
    setAuthFlowResult(null);
    setIsVerifying(false);
    setVerificationStatus(null);
    setShowCancelConfirm(false);
    setIsCleaningUp(false);
  };

  if (!isOpen) return null;

  // Handle Safe Cancellation
  const handleRequestClose = () => {
    if (step < 4 || !createdAccount) {
      resetForm();
      onClose();
    } else {
      setShowCancelConfirm(true);
    }
  };

  const handleKeepForLater = async () => {
    await onSuccess();
    resetForm();
    onClose();
  };

  const handleDeleteDraftAndClose = async () => {
    if (createdAccount) {
      try {
        setIsCleaningUp(true);
        await cleanupDraftAccount(createdAccount.id);
        await onSuccess();
      } catch (err) {
        console.error("Failed to cleanup draft account:", err);
      } finally {
        setIsCleaningUp(false);
      }
    }
    resetForm();
    onClose();
  };

  // Step 4 Execution: Profile Scaffolding
  const handleScaffoldProfile = async () => {
    try {
      setScaffolding(true);
      setError(null);

      let browserProfileId: string | undefined = undefined;

      // Create isolated browser profile if requested
      if (browserMode === "new") {
        const bpName =
          newBrowserName.trim() || `${displayName.trim()} Browser`;
        const newBp = await createBrowserProfile({
          displayName: bpName,
          browserKind: newBrowserKind,
        });
        browserProfileId = newBp.id;
      } else if (browserMode === "existing" && selectedBrowserProfileId) {
        browserProfileId = selectedBrowserProfileId;
      }

      // Create account profile with LoginRequired status
      const account = await createAccount({
        platform,
        displayName: displayName.trim(),
        accountIdentifier: accountIdentifier.trim() || undefined,
        loginMethod,
        defaultWorkspacePath: defaultWorkspacePath.trim() || undefined,
        browserProfileId,
        initialStatus: "login_required",
      });

      setCreatedAccount(account);
      setStep(5);
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setScaffolding(false);
    }
  };

  // Step 5: Official Authentication Launch
  const handleStartLogin = async () => {
    if (!createdAccount) return;
    try {
      setIsLaunchingAuth(true);
      setError(null);
      const res = await startLoginFlow(createdAccount.id);
      setAuthFlowResult(res);
      setAuthLaunched(true);
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setIsLaunchingAuth(false);
    }
  };

  // Step 6: Non-Intrusive Status Verification
  const handleVerifyStatus = async () => {
    if (!createdAccount) return;
    try {
      setIsVerifying(true);
      setError(null);
      const status = await checkAccountStatus(createdAccount.id);
      setVerificationStatus(status);
      if (status === "ready") {
        setStep(7);
      }
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setIsVerifying(false);
    }
  };

  // Step 3 Next handler: validate and proceed to scaffolding
  const handleStep3Submit = () => {
    setStep(4);
    handleScaffoldProfile();
  };

  // Step titles for Stepper
  const steps = [
    { num: 1, title: "Platform" },
    { num: 2, title: "Details" },
    { num: 3, title: "Browser" },
    { num: 4, title: "Scaffold" },
    { num: 5, title: "Authenticate" },
    { num: 6, title: "Verify" },
    { num: 7, title: "Ready" },
  ];

  return (
    <div className="modal-overlay">
      <div className="modal-dialog wizard-dialog">
        {/* Wizard Header */}
        <div className="modal-header">
          <div>
            <h3>Add New Account Profile</h3>
            <span className="text-muted text-xs">
              Step {step} of 7: {steps[step - 1]?.title}
            </span>
          </div>
          <button
            className="btn-close"
            onClick={handleRequestClose}
            disabled={scaffolding || isCleaningUp}
          >
            <X size={16} />
          </button>
        </div>

        {/* Wizard Stepper Progress Bar */}
        <div className="wizard-stepper">
          {steps.map((s, idx) => (
            <React.Fragment key={s.num}>
              <div
                className={`wizard-step-indicator ${
                  step === s.num
                    ? "active"
                    : step > s.num
                    ? "completed"
                    : ""
                }`}
              >
                <div className="wizard-step-circle">
                  {step > s.num ? <Check size={11} /> : s.num}
                </div>
                <span>{s.title}</span>
              </div>
              {idx < steps.length - 1 && <div className="wizard-step-separator" />}
            </React.Fragment>
          ))}
        </div>

        {/* Wizard Content Body */}
        <div className="modal-body">
          {error && <div className="alert alert-error">{error}</div>}

          {/* STEP 1: CHOOSE PLATFORM */}
          {step === 1 && (
            <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
              <p className="text-muted">
                Select the AI coding assistant platform for this profile.
              </p>

              {loadingInitial ? (
                <div style={{ textAlign: "center", padding: "24px 0" }}>
                  <RefreshCw size={24} className="animate-spin" style={{ color: "var(--accent-primary)", margin: "0 auto 8px" }} />
                  <span className="text-muted text-xs">Loading platform capabilities...</span>
                </div>
              ) : (
                <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
                  {capabilities.map((cap) => (
                    <div
                      key={cap.platform}
                      className={`wizard-card ${platform === cap.platform ? "selected" : ""}`}
                      onClick={() => setPlatform(cap.platform)}
                    >
                      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                        <strong>{cap.displayName}</strong>
                        <div style={{ display: "flex", gap: 4 }}>
                          {cap.supportedSurfaces.map((s) => (
                            <span key={s} className="wizard-badge badge-blue">
                              {s === "desktop_app" ? "Desktop App" : s.toUpperCase()}
                            </span>
                          ))}
                        </div>
                      </div>
                      <div className="radio-subtext">{cap.description}</div>
                      <div style={{ marginTop: 4, display: "flex", gap: 6, flexWrap: "wrap" }}>
                        {cap.supportsDesktopIsolation ? (
                          <span className="wizard-badge badge-green">✓ Desktop Isolation Verified</span>
                        ) : (
                          <span className="wizard-badge badge-amber">Desktop Single-Instance</span>
                        )}
                        <span className="wizard-badge badge-purple">{cap.isolationSummary}</span>
                      </div>
                      {cap.platform === "codex" && (
                        <div className="alert alert-warning" style={{ marginTop: 8, fontSize: 11 }}>
                          <strong>Notice:</strong> Codex Desktop on Windows operates as a single instance and shares the system-level session. Independent account profiles are isolated via Codex CLI.
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* STEP 2: ACCOUNT DETAILS */}
          {step === 2 && (
            <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
              <div className="form-group">
                <label className="form-label">Profile Display Name *</label>
                <input
                  type="text"
                  className="form-input"
                  placeholder="e.g., Personal Main, Work Client B, Research"
                  value={displayName}
                  onChange={(e) => setDisplayName(e.target.value)}
                  autoFocus
                  required
                />
                <span className="form-hint">A recognizable label for switching profiles.</span>
              </div>

              <div className="form-group">
                <label className="form-label">Account Identifier (Optional Hint)</label>
                <input
                  type="text"
                  className="form-input"
                  placeholder="e.g., user@company.com or @handle"
                  value={accountIdentifier}
                  onChange={(e) => setAccountIdentifier(e.target.value)}
                />
                <span className="form-hint">Displayed as a hint on the card. AI Switcher never reads secrets.</span>
              </div>

              <div className="form-group">
                <label className="form-label">Login Method Metadata</label>
                <select
                  className="form-select"
                  value={loginMethod}
                  onChange={(e) => setLoginMethod(e.target.value as LoginMethod)}
                >
                  <option value="google">Google OAuth</option>
                  <option value="email">Email / Password</option>
                  <option value="email_otp">Email Verification OTP</option>
                  <option value="phone">Phone Authentication</option>
                  <option value="passkey">Passkey</option>
                  <option value="oauth">OAuth 2.0 Provider</option>
                  <option value="other">Other / Custom</option>
                </select>
                <span className="form-hint">Descriptive metadata only. No credentials are stored.</span>
              </div>

              <div className="form-group">
                <label className="form-label">Default Workspace Directory (Optional)</label>
                <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
                  <input
                    type="text"
                    className="form-input"
                    placeholder="e.g., C:\Projects\MyProject"
                    value={defaultWorkspacePath}
                    onChange={(e) => setDefaultWorkspacePath(e.target.value)}
                    style={{ flex: 1 }}
                  />
                  <button
                    type="button"
                    className="btn btn-secondary btn-sm"
                    onClick={async () => {
                      const selected = await pickDirectory(defaultWorkspacePath || undefined);
                      if (selected) {
                        setDefaultWorkspacePath(selected);
                      }
                    }}
                    style={{ flexShrink: 0, padding: "6px 12px", whiteSpace: "nowrap" }}
                  >
                    <Folder size={13} style={{ marginRight: 4 }} />
                    Browse...
                  </button>
                </div>
                <span className="form-hint">Default directory opened when launching this profile.</span>
              </div>
            </div>
          )}

          {/* STEP 3: BROWSER ENVIRONMENT */}
          {step === 3 && (
            <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
              <p className="text-muted">
                Choose how web browser sessions (documentation, OAuth logins, web portals) are isolated for this account.
              </p>

              <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
                {/* Option 1: Create Dedicated */}
                <div
                  className={`wizard-card ${browserMode === "new" ? "selected" : ""}`}
                  onClick={() => setBrowserMode("new")}
                >
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                    <strong>Create Dedicated Isolated Browser Profile (Recommended)</strong>
                    <span className="wizard-badge badge-green">Recommended</span>
                  </div>
                  <div className="radio-subtext">
                    Keeps cookies, web logins, and search history 100% separate from your personal browser and other AI profiles.
                  </div>

                  {browserMode === "new" && (
                    <div style={{ marginTop: 10, display: "flex", flexDirection: "column", gap: 8 }}>
                      <div>
                        <label className="form-label">Browser Profile Name</label>
                        <input
                          type="text"
                          className="form-input"
                          placeholder={`${displayName.trim() || "Account"} Browser`}
                          value={newBrowserName}
                          onChange={(e) => setNewBrowserName(e.target.value)}
                          style={{ width: "100%", marginTop: 4 }}
                        />
                      </div>
                      <div>
                        <label className="form-label">Browser Engine</label>
                        <select
                          className="form-select"
                          value={newBrowserKind}
                          onChange={(e) => setNewBrowserKind(e.target.value as BrowserKind)}
                          style={{ width: "100%", marginTop: 4 }}
                        >
                          <option value="chrome">Google Chrome</option>
                          <option value="edge">Microsoft Edge</option>
                          <option value="brave">Brave Browser</option>
                        </select>
                      </div>
                    </div>
                  )}
                </div>

                {/* Option 2: Link Existing */}
                <div
                  className={`wizard-card ${browserMode === "existing" ? "selected" : ""}`}
                  onClick={() => setBrowserMode("existing")}
                >
                  <strong>Link an Existing Browser Profile</strong>
                  <div className="radio-subtext">
                    Share an existing browser environment across accounts.
                  </div>

                  {browserMode === "existing" && (
                    <div style={{ marginTop: 8 }}>
                      {browserProfiles.length === 0 ? (
                        <div className="text-muted text-xs">No existing browser profiles found.</div>
                      ) : (
                        <select
                          className="form-select"
                          value={selectedBrowserProfileId}
                          onChange={(e) => setSelectedBrowserProfileId(e.target.value)}
                          style={{ width: "100%" }}
                        >
                          <option value="">Select an existing browser profile...</option>
                          {browserProfiles.map((bp) => (
                            <option key={bp.id} value={bp.id}>
                              {bp.displayName} ({bp.browserKind.toUpperCase()})
                            </option>
                          ))}
                        </select>
                      )}
                      {selectedBrowserProfileId && (
                        <div className="alert alert-warning" style={{ marginTop: 8, fontSize: 11 }}>
                          ⚠️ <strong>Shared Profile Warning:</strong> Reusing a browser profile means cookies, active logins, and web history will be shared between linked accounts.
                        </div>
                      )}
                    </div>
                  )}
                </div>

                {/* Option 3: None */}
                <div
                  className={`wizard-card ${browserMode === "none" ? "selected" : ""}`}
                  onClick={() => setBrowserMode("none")}
                >
                  <strong>None / System Default Browser</strong>
                  <div className="radio-subtext">
                    Use your default system browser without AI Switcher profile isolation.
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* STEP 4: PROFILE CREATION & SCAFFOLDING */}
          {step === 4 && (
            <div style={{ padding: "32px 16px", textAlign: "center" }}>
              {scaffolding ? (
                <div>
                  <RefreshCw size={32} className="animate-spin" style={{ color: "var(--accent-primary)", margin: "0 auto 16px" }} />
                  <h4>Initializing Profile Environment...</h4>
                  <p className="text-muted text-xs" style={{ marginTop: 8 }}>
                    Scaffolding isolated user directories, security locks, and database records.
                  </p>
                </div>
              ) : error ? (
                <div>
                  <AlertCircle size={32} style={{ color: "var(--status-error)", margin: "0 auto 16px" }} />
                  <h4>Initialization Failed</h4>
                  <p className="text-muted text-xs" style={{ marginTop: 8 }}>{error}</p>
                  <button className="btn btn-primary" onClick={handleScaffoldProfile} style={{ marginTop: 16 }}>
                    Retry Initialization
                  </button>
                </div>
              ) : null}
            </div>
          )}

          {/* STEP 5: OFFICIAL AUTHENTICATION */}
          {step === 5 && (
            <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
              <div className="auth-instruction-box">
                <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                  <Shield size={20} style={{ color: "var(--status-ready)" }} />
                  <strong style={{ fontSize: 13 }}>Official Platform Authentication</strong>
                </div>

                <p style={{ fontSize: 12, lineHeight: 1.5 }}>
                  {platform === "antigravity" && (
                    <>
                      Google Antigravity Desktop will launch in an isolated window with your new profile. Sign in with your Google account inside Antigravity.
                    </>
                  )}
                  {platform === "claude" && (
                    <>
                      Claude Desktop will launch in an isolated window with your new profile. Sign in with your Anthropic or Google account inside Claude.
                    </>
                  )}
                  {platform === "codex" && (
                    <>
                      An external terminal will launch running <code>codex login</code> configured with your isolated profile. Follow the prompts in the terminal to complete your sign-in.
                    </>
                  )}
                </p>

                <div className="alert alert-warning" style={{ fontSize: 11 }}>
                  <strong>Security Assurance:</strong> AI Switcher never intercepts, logs, or stores your passwords, tokens, or credentials.
                </div>
              </div>

              <div style={{ display: "flex", flexDirection: "column", gap: 10, marginTop: 4 }}>
                <button
                  className="btn btn-primary"
                  onClick={handleStartLogin}
                  disabled={isLaunchingAuth}
                  style={{ padding: "10px 16px", fontSize: 13 }}
                >
                  {isLaunchingAuth ? (
                    <RefreshCw size={15} className="animate-spin" />
                  ) : (
                    <Play size={15} />
                  )}
                  {authLaunched ? "Relaunch Official Login" : "Start Official Login"}
                </button>

                {authLaunched && (
                  <div className="alert alert-info" style={{ fontSize: 12, lineHeight: 1.4 }}>
                    <strong>{authFlowResult?.message || "Official sign-in opened. Complete sign-in, then verify."}</strong>
                    <div style={{ marginTop: 4, fontSize: 11 }}>
                      Complete your sign-in inside the official application or terminal, then click the button below to verify your session.
                    </div>
                  </div>
                )}

                {authLaunched && (
                  <button
                    className="btn btn-secondary"
                    onClick={() => {
                      setStep(6);
                      handleVerifyStatus();
                    }}
                    style={{ padding: "10px 16px" }}
                  >
                    <CheckCircle2 size={15} style={{ color: "var(--status-ready)" }} />
                    I Have Completed Sign-In — Verify Profile
                  </button>
                )}
              </div>
            </div>
          )}

          {/* STEP 6: NON-INTRUSIVE VERIFICATION */}
          {step === 6 && (
            <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
              <div style={{ textAlign: "center", padding: "16px 8px" }}>
                {isVerifying ? (
                  <div>
                    <RefreshCw size={28} className="animate-spin" style={{ color: "var(--accent-primary)", margin: "0 auto 12px" }} />
                    <h4>Verifying Profile Authentication...</h4>
                    <p className="text-muted text-xs" style={{ marginTop: 6 }}>
                      Non-intrusively probing session status without extracting credentials.
                    </p>
                  </div>
                ) : verificationStatus === "ready" ? (
                  <div className="auth-status-card auth-status-ready">
                    <CheckCircle2 size={28} style={{ color: "var(--status-ready)" }} />
                    <div style={{ textAlign: "left" }}>
                      <strong style={{ color: "var(--status-ready)" }}>Authentication Verified!</strong>
                      <div className="text-muted text-xs">
                        Profile credentials have been saved in the isolated environment.
                      </div>
                    </div>
                  </div>
                ) : (
                  <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
                    <div className="auth-status-card auth-status-pending">
                      <AlertTriangle size={24} style={{ color: "var(--status-login)" }} />
                      <div style={{ textAlign: "left" }}>
                        <strong style={{ color: "var(--status-login)" }}>Session Not Yet Detected</strong>
                        <div className="text-muted text-xs">
                          We didn't detect an active session yet. Did you complete the login in the official app or terminal?
                        </div>
                      </div>
                    </div>

                    <div style={{ display: "flex", gap: 8, justifyContent: "center", marginTop: 8 }}>
                      <button className="btn btn-secondary" onClick={handleVerifyStatus}>
                        <RefreshCw size={13} />
                        Re-check Status
                      </button>
                      <button className="btn btn-secondary" onClick={handleStartLogin}>
                        <Play size={13} />
                        Relaunch Login
                      </button>
                      <button className="btn btn-secondary" onClick={() => setStep(7)}>
                        Skip Verification for Now
                      </button>
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* STEP 7: READY / COMPLETION */}
          {step === 7 && createdAccount && (
            <div style={{ display: "flex", flexDirection: "column", gap: 16, textAlign: "center", padding: "16px 8px" }}>
              <div style={{ margin: "0 auto" }}>
                <CheckCircle2 size={40} style={{ color: "var(--status-ready)" }} />
              </div>

              <div>
                <h3>Profile Ready!</h3>
                <p className="text-muted" style={{ fontSize: 12, marginTop: 4 }}>
                  Account profile <strong>{createdAccount.displayName}</strong> is configured and registered in AI Switcher.
                </p>
              </div>

              <div style={{ display: "inline-flex", justifyContent: "center", gap: 8 }}>
                <StatusBadge status={verificationStatus || createdAccount.status} />
                <span className="wizard-badge badge-blue">{createdAccount.platform.toUpperCase()}</span>
                <span className="wizard-badge badge-purple">{createdAccount.loginMethod}</span>
              </div>

              <div style={{ display: "flex", gap: 10, justifyContent: "center", marginTop: 12 }}>
                <button
                  className="btn btn-primary"
                  onClick={async () => {
                    await onSuccess();
                    onClose();
                    if (onOpenAccount) onOpenAccount(createdAccount);
                  }}
                  style={{ padding: "8px 18px" }}
                >
                  <Play size={14} />
                  Open {createdAccount.displayName} Now
                </button>
                <button
                  className="btn btn-secondary"
                  onClick={async () => {
                    await onSuccess();
                    onClose();
                  }}
                  style={{ padding: "8px 18px" }}
                >
                  Done
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Wizard Navigation Footer */}
        {step <= 3 && (
          <div className="modal-footer">
            <button
              type="button"
              className="btn btn-secondary"
              onClick={handleRequestClose}
            >
              Cancel
            </button>

            <div style={{ display: "flex", gap: 8 }}>
              {step > 1 && (
                <button
                  type="button"
                  className="btn btn-secondary"
                  onClick={() => setStep((s) => (s - 1) as WizardStep)}
                >
                  <ArrowLeft size={14} />
                  Back
                </button>
              )}

              {step === 1 && (
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => setStep(2)}
                >
                  Next
                  <ArrowRight size={14} />
                </button>
              )}

              {step === 2 && (
                <button
                  type="button"
                  className="btn btn-primary"
                  disabled={!displayName.trim()}
                  onClick={() => setStep(3)}
                >
                  Next
                  <ArrowRight size={14} />
                </button>
              )}

              {step === 3 && (
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={handleStep3Submit}
                >
                  Create Profile & Scaffold
                  <ArrowRight size={14} />
                </button>
              )}
            </div>
          </div>
        )}

        {/* Step 5 Footer */}
        {step === 5 && (
          <div className="modal-footer">
            <button
              type="button"
              className="btn btn-secondary"
              onClick={handleRequestClose}
            >
              Cancel Setup
            </button>

            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => setStep(7)}
            >
              Set Up Later
            </button>
          </div>
        )}

        {/* Step 6 Footer */}
        {step === 6 && verificationStatus === "ready" && (
          <div className="modal-footer">
            <button
              type="button"
              className="btn btn-primary"
              onClick={() => setStep(7)}
            >
              Continue to Completion
              <ArrowRight size={14} />
            </button>
          </div>
        )}
      </div>

      {/* Safe Cancellation Confirmation Modal */}
      {showCancelConfirm && (
        <div className="modal-overlay" style={{ zIndex: 1100 }}>
          <div className="modal-dialog" style={{ maxWidth: 460 }}>
            <div className="modal-header">
              <h3 className="text-danger flex items-center gap-2">
                <AlertTriangle size={18} />
                Cancel Registration
              </h3>
            </div>
            <div className="modal-body">
              <p>
                An isolated profile environment has already been scaffolded for{" "}
                <strong>{createdAccount?.displayName || "this account"}</strong>.
              </p>
              <div style={{ marginTop: 12, display: "flex", flexDirection: "column", gap: 10 }}>
                <button
                  className="btn btn-secondary"
                  onClick={handleKeepForLater}
                  disabled={isCleaningUp}
                  style={{ textAlign: "left", justifyContent: "flex-start", padding: "10px" }}
                >
                  <div>
                    <strong>Keep Profile for Later</strong>
                    <div className="text-muted text-xs">
                      Saves account as "Login Required" so you can finish logging in at any time.
                    </div>
                  </div>
                </button>

                <button
                  className="btn btn-danger"
                  onClick={handleDeleteDraftAndClose}
                  disabled={isCleaningUp}
                  style={{ textAlign: "left", justifyContent: "flex-start", padding: "10px" }}
                >
                  <div>
                    <strong>Delete Local Setup & Discard</strong>
                    <div className="text-xs" style={{ opacity: 0.9 }}>
                      {isCleaningUp
                        ? "Cleaning up files..."
                        : "Permanently removes the newly scaffolded profile folder and deletes the record."}
                    </div>
                  </div>
                </button>
              </div>
            </div>
            <div className="modal-footer">
              <button
                type="button"
                className="btn btn-secondary"
                onClick={() => setShowCancelConfirm(false)}
                disabled={isCleaningUp}
              >
                Return to Wizard
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
