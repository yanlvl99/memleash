// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use sentry_src::{AppProfile, AppSnapshot, WindowState, scan};

fn total_ws_mb(snap: &AppSnapshot) -> f64 {
    snap.processes
        .iter()
        .filter_map(|p| p.metrics.map(|m| m.working_set_mb))
        .sum()
}

fn main() -> sentry_src::Result<()> {
    let target = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Spotify.exe".to_string());
    let profile = AppProfile::chromium(target);
    let snap = scan(&profile)?;

    if snap.processes.is_empty() {
        println!("{} is not running.", snap.exe_name);
        return Ok(());
    }

    if snap.window_state == WindowState::Foreground {
        println!(
            "{} is in the foreground; skipping cap to avoid stuttering.",
            snap.exe_name
        );
        return Ok(());
    }

    println!(
        "{} ({:?}) | {} process(es) | Total WS before: {:.2} MB",
        snap.exe_name,
        snap.window_state,
        snap.processes.len(),
        total_ws_mb(&snap)
    );

    let cap_report = snap.cap_all(true);
    println!(
        "cap_all(trim=true): succeeded={} skipped={} failures={}",
        cap_report.succeeded,
        cap_report.skipped,
        cap_report.failures.len()
    );
    for (pid, err) in &cap_report.failures {
        println!("  cap failure PID {pid}: {err}");
    }

    let after = scan(&profile)?;
    println!("Total WS after cap + trim: {:.2} MB", total_ws_mb(&after));

    let release_report = snap.release_all();
    println!(
        "release_all(): succeeded={} skipped={} failures={}",
        release_report.succeeded,
        release_report.skipped,
        release_report.failures.len()
    );
    for (pid, err) in &release_report.failures {
        println!("  release failure PID {pid}: {err}");
    }

    Ok(())
}
