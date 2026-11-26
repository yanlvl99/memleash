// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use std::collections::HashSet;

use sentry_profiles::AppProfile;
use sentry_utils::{command_line, find_by_name, foreground_pid, inspect};

fn main() -> sentry_utils::Result<()> {
    let profile = AppProfile::chromium("spotify.exe");

    let fg = foreground_pid();
    println!("Foreground PID: {fg:?}");

    let processes = find_by_name(profile.exe_name())?;
    if processes.is_empty() {
        println!("{} is not running.", profile.exe_name());
        return Ok(());
    }

    let pids: HashSet<u32> = processes.iter().map(|p| p.id.pid).collect();
    println!("Found {} {} process(es):", processes.len(), profile.exe_name());

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
            Err(e) => println!("  PID {:>6} | {:<18} | error: {e}", proc.id.pid, class.identity),
        }
    }

    Ok(())
}
