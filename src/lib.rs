// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use std::collections::HashSet;

pub use sentry_profiles as profiles;
pub use sentry_win32 as win32;

pub use sentry_profiles::{
    AppProfile, AppProfileBuilder, Classification, MatchRule, RoleConfig, SubLabelRule,
};
pub use sentry_win32::{
    MemoryAvailability, MemoryWatcher, ProcessId, ProcessInfo, ProcessMetrics, Result, SentryError,
    SystemMemory, WindowCandidate, WindowState, active_audio_pids, cap, command_line,
    empty_working_set, find_by_name, find_windows, foreground_pid, inspect, release, system_memory,
    window_state, working_set_mb,
};

#[derive(Debug, Clone)]
pub struct ScannedProcess {
    pub info: ProcessInfo,
    pub role: String,
    pub sub_label: Option<String>,
    pub identity: String,
    pub config: RoleConfig,
    pub metrics: Option<ProcessMetrics>,
    pub is_foreground: bool,
    pub is_audio: bool,
}

#[derive(Debug, Default)]
pub struct ActionReport {
    pub sucess: usize,
    pub skipped: usize,
    pub failures: Vec<(u32, SentryError)>,
}

#[derive(Debug, Clone)]
pub struct AppSnapshot {
    pub exe_name: String,
    pub window_state: WindowState,
    pub window: Option<WindowCandidate>,
    pub active_audio_pids: HashSet<u32>,
    pub processes: Vec<ScannedProcess>,
}

impl ActionReport {
    pub fn is_ok(&self) -> bool {
        self.failures.is_empty()
    }
}

pub fn scan(profile: &AppProfile) -> Result<AppSnapshot> {
    let raw_processes = find_by_name(profile.exe_name())?;
    let audio_pids = active_audio_pids().unwrap_or_default();
    let fg = foreground_pid();

    let pid_list: Vec<u32> = raw_processes.iter().map(|p| p.id.pid).collect();
    let pid_set: HashSet<u32> = pid_list.iter().copied().collect();
    let (win_state, main_window) = window_state(&pid_list);

    let mut processes = Vec::with_capacity(raw_processes.len());
    for info in raw_processes {
        let is_root = !pid_set.contains(&info.parent_pid);
        let is_audio = audio_pids.contains(&info.id.pid);
        let is_foreground = fg == Some(info.id.pid);
        let cmd = command_line(info.id).unwrap_or_default();
        let class = profile.classify(&cmd, is_root, is_audio);
        let metrics = inspect(info.id).ok();

        processes.push(ScannedProcess {
            info,
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
        processes,
    })
}
