// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use memleash::{AppProfile, MemoryWatcher, scan, system_memory};

fn main() -> memleash::Result<()> {
    let mem = system_memory()?;
    let watcher = MemoryWatcher::new()?;
    println!(
        "System RAM: {} / {} MB available ({}% load) | Availability: {:?}",
        mem.available_mb,
        mem.total_mb,
        mem.load_percent,
        watcher.availability()?
    );

    let target = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Spotify.exe".to_string());
    let profile = AppProfile::chromium(target);
    let snap = scan(&profile)?;

    if snap.processes.is_empty() {
        println!("{} is not running.", snap.exe_name);
        return Ok(());
    }

    println!(
        "Found {} {} process(es) | Window: {:?} ({}) | Active audio PIDs: {:?}",
        snap.processes.len(),
        snap.exe_name,
        snap.window_state,
        snap.window.as_ref().map(|w| w.title.as_str()).unwrap_or("none"),
        snap.active_audio_pids
    );

    for proc in &snap.processes {
        let focus = if proc.is_foreground {
            " [FOREGROUND]"
        } else {
            ""
        };
        match proc.metrics {
            Some(m) => println!(
                "  PID {:>6} | {:<18} | cap {:>2}-{:<2} MB | WS: {:>6.2} MB | CPU: {:>8.1} ms{}",
                proc.info.id.pid,
                proc.identity,
                proc.config.min_mb,
                proc.config.max_mb,
                m.working_set_mb,
                m.cpu_time_ms,
                focus
            ),
            None => println!(
                "  PID {:>6} | {:<18} | metrics unavailable",
                proc.info.id.pid, proc.identity
            ),
        }
    }

    Ok(())
}
