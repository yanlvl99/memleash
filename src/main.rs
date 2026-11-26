// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use sentry_profiles::{MatchRule, SubLabelRule};
use sentry_utils::{command_line, find_by_name, foreground_pid, inspect};

fn main() -> sentry_utils::Result<()> {
    let fg = foreground_pid();
    println!("Foreground PID: {fg:?}");

    let is_renderer = MatchRule::cmd_contains("--type=renderer");
    let is_utility = MatchRule::cmd_contains("--type=utility");
    let renderer_sub = SubLabelRule::flag_value("--renderer-client-id", "#");
    let utility_sub = SubLabelRule::flag_head("--utility-sub-type", '.');

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
        let cmd = command_line(proc.id).unwrap_or_default();
        let label = if is_renderer.matches(&cmd, false, false) {
            match renderer_sub.extract(&cmd) {
                Some(sub) => format!("Renderer:{sub}"),
                None => "Renderer".to_string(),
            }
        } else if is_utility.matches(&cmd, false, false) {
            match utility_sub.extract(&cmd) {
                Some(sub) => format!("Utility:{sub}"),
                None => "Utility".to_string(),
            }
        } else {
            "Other".to_string()
        };

        match inspect(proc.id) {
            Ok(m) => println!(
                "  PID {:>6} | {:<16} | WS: {:>6.2} MB | Faults: {:>9} | CPU: {:>8.1} ms{}",
                proc.id.pid, label, m.working_set_mb, m.page_fault_count, m.cpu_time_ms, focus
            ),
            Err(e) => println!("  PID {:>6} | {:<16} | error: {e}", proc.id.pid, label),
        }
    }

    Ok(())
}
