// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use sentry_utils::{command_line, find_by_name, foreground_pid, inspect};

fn extract_type(cmd: &str) -> &str {
    cmd.split_whitespace()
        .find_map(|arg| arg.strip_prefix("--type="))
        .unwrap_or("main")
}

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
        let kind = command_line(proc.id)
            .map(|cmd| extract_type(&cmd).to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        match inspect(proc.id) {
            Ok(m) => println!(
                "  PID {:>6} | {:<16} | WS: {:>6.2} MB | Faults: {:>9} | CPU: {:>8.1} ms{}",
                proc.id.pid, kind, m.working_set_mb, m.page_fault_count, m.cpu_time_ms, focus
            ),
            Err(e) => println!("  PID {:>6} | {:<16} | error: {e}", proc.id.pid, kind),
        }
    }

    Ok(())
}
