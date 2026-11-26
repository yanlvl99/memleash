// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use sentry_utils::{find_by_name, working_set_mb};

fn main() -> sentry_utils::Result<()> {
    let processes = find_by_name("spotify.exe")?;

    if processes.is_empty() {
        println!("Spotify is not running.");
        return Ok(());
    }

    println!("Found {} Spotify process(es):", processes.len());
    for proc in processes {
        let ws = working_set_mb(proc.id)
            .map(|mb| format!("{mb:.2} MB"))
            .unwrap_or_else(|e| format!("error: {e}"));
        println!(
            "  PID {:>6} | PPID {:>6} | {} | Working Set: {}",
            proc.id.pid, proc.parent_pid, proc.exe_name, ws
        );
    }

    Ok(())
}
