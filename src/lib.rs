// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use std::collections::HashSet;

pub use memleash_profiles as profiles;
pub use memleash_win32 as win32;

pub use memleash_profiles::{
    AppProfile, AppProfileBuilder, Classification, MatchRule, RoleConfig, SubLabelRule,
};
pub use memleash_win32::{
    Error, MemoryAvailability, MemoryWatcher, ProcessId, ProcessInfo, ProcessMetrics, Result,
    SystemMemory, WindowCandidate, WindowState, active_audio_pids, cap, command_line,
    empty_working_set, find_by_name, find_windows, foreground_pid, inspect, release, system_memory,
    window_state, working_set_mb,
};

#[derive(Debug, Clone)]
pub struct ScannedProcess {
    pub info: ProcessInfo,
    pub command_line: Option<String>,
    pub role: String,
    pub sub_label: Option<String>,
    pub identity: String,
    pub config: RoleConfig,
    pub metrics: Option<ProcessMetrics>,
    pub is_foreground: bool,
    pub is_audio: bool,
}

#[must_use = "batch actions may have per-process failures; inspect the report"]
#[derive(Debug, Default)]
pub struct ActionReport {
    pub succeeded: usize,
    pub skipped: usize,
    pub failures: Vec<(u32, Error)>,
}

#[derive(Debug, Clone)]
pub struct AppSnapshot {
    pub exe_name: String,
    pub window_state: WindowState,
    pub window: Option<WindowCandidate>,
    pub active_audio_pids: HashSet<u32>,
    pub audio_available: bool,
    pub processes: Vec<ScannedProcess>,
}

impl ActionReport {
    pub fn is_ok(&self) -> bool {
        self.failures.is_empty()
    }
}

impl AppSnapshot {
    fn run_for_managed<F>(&self, mut action: F) -> ActionReport
    where
        F: FnMut(&ScannedProcess) -> Result<()>,
    {
        let mut report = ActionReport::default();
        for proc in &self.processes {
            if !proc.config.managed {
                report.skipped += 1;
                continue;
            }
            match action(proc) {
                Ok(()) => report.succeeded += 1,
                Err(err) => report.failures.push((proc.info.id.pid, err)),
            }
        }
        report
    }

    pub fn cap_all(&self, trim: bool) -> ActionReport {
        self.run_for_managed(|proc| {
            cap(proc.info.id, proc.config.initial_mb)?;
            if trim {
                empty_working_set(proc.info.id)?;
            }
            Ok(())
        })
    }

    pub fn empty_all(&self) -> ActionReport {
        self.run_for_managed(|proc| empty_working_set(proc.info.id))
    }

    pub fn release_all(&self) -> ActionReport {
        self.run_for_managed(|proc| release(proc.info.id))
    }
}

pub fn scan(profile: &AppProfile) -> Result<AppSnapshot> {
    let raw_processes = find_by_name(profile.exe_name())?;
    let (audio_pids, audio_available) = match active_audio_pids() {
        Ok(pids) => (pids, true),
        Err(_) => (HashSet::new(), false),
    };
    let fg = foreground_pid();

    let pid_list: Vec<u32> = raw_processes.iter().map(|p| p.id.pid).collect();
    let pid_set: HashSet<u32> = pid_list.iter().copied().collect();
    let (win_state, main_window) = window_state(&pid_list);

    let mut processes = Vec::with_capacity(raw_processes.len());
    for info in raw_processes {
        let is_root = !pid_set.contains(&info.parent_pid);
        let is_audio = audio_pids.contains(&info.id.pid);
        let is_foreground = fg == Some(info.id.pid);
        let cmd = command_line(info.id).ok();
        let class = profile.classify(cmd.as_deref().unwrap_or(""), is_root, is_audio);
        let metrics = inspect(info.id).ok();

        processes.push(ScannedProcess {
            info,
            command_line: cmd,
            role: class.role.to_string(),
            sub_label: class.sub_label,
            identity: class.identity,
            config: *class.config,
            metrics,
            is_foreground,
            is_audio,
        });
    }

    Ok(AppSnapshot {
        exe_name: profile.exe_name().to_string(),
        window_state: win_state,
        window: main_window,
        active_audio_pids: audio_pids,
        audio_available,
        processes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_scanned(id: ProcessId, config: RoleConfig) -> ScannedProcess {
        ScannedProcess {
            info: ProcessInfo {
                id,
                parent_pid: 0,
                exe_name: "test.exe".to_string(),
            },
            command_line: None,
            role: "Main".to_string(),
            sub_label: None,
            identity: "Main".to_string(),
            config,
            metrics: None,
            is_foreground: false,
            is_audio: false,
        }
    }

    #[test]
    fn action_report_is_ok_reflects_failures() {
        let mut report = ActionReport::default();
        assert!(report.is_ok());

        report.failures.push((1, Error::InvalidLimit(0)));
        assert!(!report.is_ok());
    }

    #[test]
    fn snapshot_actions_skip_unmanaged_and_continue_after_failure() {
        let me = ProcessId::from_pid(std::process::id()).expect("current ProcessId");
        let stale = ProcessId {
            pid: me.pid,
            creation_time: 0,
        };

        let snap = AppSnapshot {
            exe_name: "test.exe".to_string(),
            window_state: WindowState::NotFound,
            window: None,
            active_audio_pids: HashSet::new(),
            audio_available: true,
            processes: vec![
                fake_scanned(me, RoleConfig::new(4, 32).unmanaged()),
                fake_scanned(stale, RoleConfig::new(4, 32)),
                fake_scanned(me, RoleConfig::new(4, 32)),
            ],
        };

        let report = snap.empty_all();
        assert_eq!(report.skipped, 1);
        assert_eq!(report.succeeded, 1);
        assert_eq!(report.failures.len(), 1);
        assert!(matches!(report.failures[0].1, Error::PidReused { .. }));
        assert!(!report.is_ok());
    }

    #[test]
    fn scan_finds_current_process_and_handles_missing_exe() {
        let exe = std::env::current_exe().expect("current_exe");
        let name = exe
            .file_name()
            .expect("exe file name")
            .to_string_lossy()
            .into_owned();
        let current_pid = std::process::id();

        let profile = AppProfile::single(name, RoleConfig::new(4, 32));
        let snap = scan(&profile).expect("scan current exe");
        let current = snap
            .processes
            .iter()
            .find(|p| p.info.id.pid == current_pid)
            .expect("current PID in snapshot");
        assert_eq!(current.identity, "Main");
        assert!(current.command_line.is_some());
        assert!(current.metrics.is_some());

        let missing_profile =
            AppProfile::single("__memleash_nonexistent__.exe", RoleConfig::new(4, 32));
        let missing = scan(&missing_profile).expect("scan missing exe");
        assert!(missing.processes.is_empty());
        assert_eq!(missing.window_state, WindowState::NotFound);
    }
}
