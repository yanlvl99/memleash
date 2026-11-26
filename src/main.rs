// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use std::collections::HashSet;

use sentry_profiles::AppProfile;
use sentry_utils::{
    MemoryWatcher, command_line, find_by_name, foreground_pid, inspect, system_memory,
    window_state,
};

fn main() -> sentry_utils::Result<()> {
    let mem = system_memory()?;
    let watcher = MemoryWatcher::new()?;
    let availability = watcher.availability()?;
    println!(
        "System RAM: {} / {} MB available ({}% load) | Availability: {:?}",
        mem.available_mb, mem.total_mb, mem.load_percent, availability
    );

    let target = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Spotify.exe".to_string());
    let profile = AppProfile::chromium(target);
    let fg = foreground_pid();
    let processes = find_by_name(profile.exe_name())?;
    if processes.is_empty() {
        println!("{} is not running.", profile.exe_name());
        return Ok(());
    }

    let pid_list: Vec<u32> = processes.iter().map(|p| p.id.pid).collect();
    let pids: HashSet<u32> = pid_list.iter().copied().collect();
    let (win_state, main_win) = window_state(&pid_list);
    println!(
        "Found {} {} process(es) | Window: {:?} ({})",
        processes.len(),
        profile.exe_name(),
        win_state,
        main_win.as_ref().map(|w| w.title.as_str()).unwrap_or("none")
    );

    for proc in processes {
        let is_root = !pids.contains(&proc.parent_pid);
        let cmd = command_line(proc.id).unwrap_or_default();
        let class = profile.classify(&cmd, is_root, false);
        let focus = if fg == Some(proc.id.pid) {
            " [FOREGROUND]"
        } else {
            ""
        };

        match inspect(proc.id) {
            Ok(m) => println!(
                "  PID {:>6} | {:<18} | cap {:>2}-{:<2} MB | WS: {:>6.2} MB | CPU: {:>8.1} ms{}",
                proc.id.pid,
                class.identity,
                class.config.min_mb,
                class.config.max_mb,
                m.working_set_mb,
                m.cpu_time_ms,
                focus
            ),
            Err(e) => println!(
                "  PID {:>6} | {:<18} | error: {e}",
                proc.id.pid, class.identity
            ),
        }
    }

    Ok(())
}
