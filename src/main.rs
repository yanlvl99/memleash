// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use sentry_utils::{find_by_name, foreground_pid, inspect};

fn main() -> sentry_utils::Result<()> {
    let fg = foreground_pid();
    println!("Foreground PID: {fg:?}");

    let processes = find_by_name("spotify.exe")?;
    if processes.is_empty() {
        println!("Spotify is not running.");
        return Ok(());
    }

    println!("Found {} Spotify process(es):", processes.len());
    for proc in processes {
        let focus = if fg == Some(proc.id.pid) {
            " [FOREGROUND]"
        } else {
            ""
        };
        match inspect(proc.id) {
            Ok(m) => println!(
                "  PID {:>6} | PPID {:>6} | WS: {:>6.2} MB | Faults: {:>9} | CPU: {:>8.1} ms{}",
                proc.id.pid, proc.parent_pid, m.working_set_mb, m.page_fault_count, m.cpu_time_ms, focus
            ),
            Err(e) => println!(
                "  PID {:>6} | PPID {:>6} | error: {e}",
                proc.id.pid, proc.parent_pid
            ),
        }
    }

    Ok(())
}
