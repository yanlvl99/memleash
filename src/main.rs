// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use sentry_utils::{ProcessId, cap, empty_working_set, release, utils, working_set_mb};

fn main() -> sentry_utils::Result<()> {
    let me = ProcessId::from_pid(std::process::id())?;
    let mut buf = vec![1u8; utils::mb_to_bytes(64)];

    println!("PID: {}", me.pid);
    println!("Before cap: {:.2} MB", working_set_mb(me)?);

    cap(me, 16)?;
    empty_working_set(me)?;
    println!(
        "After cap(16 MB) + empty_working_set: {:.2} MB",
        working_set_mb(me)?
    );

    for byte in buf.iter_mut().step_by(4096) {
        *byte = 2;
    }
    println!(
        "After touching 64 MB while capped at 16 MB: {:.2} MB",
        working_set_mb(me)?
    );

    release(me)?;
    for byte in buf.iter_mut().step_by(4096) {
        *byte = 3;
    }
    println!(
        "After release + touching 64 MB: {:.2} MB",
        working_set_mb(me)?
    );

    Ok(())
}
