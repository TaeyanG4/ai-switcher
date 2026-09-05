use std::sync::{Arc, Mutex};
use sysinfo::{Pid, System};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::{
    AccountProfile, ExecutionSurface, InstancePolicy, PlatformType, ProcessConflictInfo,
    ProcessRecord,
};

#[derive(Clone)]
pub struct ProcessManager {
    records: Arc<Mutex<Vec<ProcessRecord>>>,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Registers a newly spawned process
    pub fn register_launch(
        &self,
        account_id: &str,
        platform: PlatformType,
        surface: ExecutionSurface,
        pid: Option<u32>,
        executable: &str,
    ) -> ProcessRecord {
        let mut list = self.records.lock().unwrap();
        let record = ProcessRecord {
            launch_id: Uuid::new_v4().to_string(),
            account_id: account_id.to_string(),
            platform,
            surface,
            pid,
            executable: executable.to_string(),
            launch_timestamp: chrono::Utc::now().to_rfc3339(),
            is_running: true,
        };
        list.push(record.clone());
        record
    }

    /// Returns all launched records that are currently active
    pub fn list_active_processes(&self) -> Vec<ProcessRecord> {
        let mut list = self.records.lock().unwrap();
        let mut sys = System::new();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        // Update running states by checking OS process table
        for r in list.iter_mut() {
            if r.is_running {
                if let Some(pid_val) = r.pid {
                    let sys_pid = Pid::from_u32(pid_val);
                    if let Some(process) = sys.process(sys_pid) {
                        // Protect against PID reuse when absolute executable path is tracked
                        let exe_path = std::path::Path::new(&r.executable);
                        if exe_path.is_absolute() {
                            let expected_file_name = exe_path
                                .file_name()
                                .and_then(|f| f.to_str())
                                .map(|s| s.to_lowercase());
                            let actual_file_name = process
                                .exe()
                                .and_then(|p| p.file_name())
                                .and_then(|f| f.to_str())
                                .map(|s| s.to_lowercase());

                            if let (Some(expected), Some(actual)) =
                                (expected_file_name, actual_file_name)
                            {
                                if expected != actual {
                                    r.is_running = false;
                                }
                            }
                        }
                    } else {
                        r.is_running = false;
                    }
                }
            }
        }

        list.iter().filter(|r| r.is_running).cloned().collect()
    }

    /// Checks whether an account profile is currently running
    pub fn is_account_running(&self, account_id: &str) -> bool {
        let active = self.list_active_processes();
        active.iter().any(|r| r.account_id == account_id)
    }

    /// Checks for a same-platform conflict when launching `target_account`.
    /// INVARIANT: Other platforms (e.g. Codex vs Claude vs Antigravity) NEVER conflict!
    pub fn check_platform_conflict(
        &self,
        target_account: &AccountProfile,
        policy: InstancePolicy,
        all_accounts: &[AccountProfile],
    ) -> Option<ProcessConflictInfo> {
        // MultiInstance never conflicts
        if policy == InstancePolicy::MultiInstance {
            return None;
        }

        let active = self.list_active_processes();

        // Conflict occurs ONLY if another profile of the SAME platform is running
        for r in active {
            if r.platform == target_account.platform && r.account_id != target_account.id {
                let display_name = all_accounts
                    .iter()
                    .find(|a| a.id == r.account_id)
                    .map(|a| a.display_name.clone())
                    .unwrap_or_else(|| "Unknown Profile".to_string());

                return Some(ProcessConflictInfo {
                    running_account_id: r.account_id,
                    running_display_name: display_name,
                    running_launch_id: r.launch_id,
                    running_pid: r.pid,
                    platform: r.platform,
                    policy,
                });
            }
        }

        None
    }

    /// Terminates a process tracked by launch_id.
    /// NEVER automatically force-kills; force must be explicitly requested.
    pub fn terminate_process(&self, launch_id: &str, force: bool) -> Result<()> {
        let mut list = self.records.lock().unwrap();
        let record = list
            .iter_mut()
            .find(|r| r.launch_id == launch_id)
            .ok_or_else(|| {
                AppError::NotFound(format!("Launch record '{}' not found", launch_id))
            })?;

        if let Some(pid_val) = record.pid {
            let mut sys = System::new();
            sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
            let sys_pid = Pid::from_u32(pid_val);

            if let Some(proc) = sys.process(sys_pid) {
                let success = if force {
                    proc.kill_with(sysinfo::Signal::Kill).unwrap_or(false)
                } else {
                    // Attempt graceful termination (SIGTERM on sysinfo, or standard termination)
                    proc.kill_with(sysinfo::Signal::Term)
                        .or_else(|| proc.kill_with(sysinfo::Signal::Quit))
                        .unwrap_or_else(|| proc.kill())
                };

                if !success {
                    return Err(AppError::ProcessTerminationFailed(format!(
                        "Failed to terminate process PID {}",
                        pid_val
                    )));
                }
            }
        }

        record.is_running = false;
        Ok(())
    }
}
