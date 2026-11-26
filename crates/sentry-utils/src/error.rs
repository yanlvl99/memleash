// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SentryError {
    #[error("failed to open process {pid}: {source}")]
    OpenProcess {
        pid: u32,
        #[source]
        source: windows::core::Error,
    },

    #[error("process PID {pid} was reused (expected creation time {expected}, got {actual})")]
    PidReused {
        pid: u32,
        expected: u64,
        actual: u64,
    },

    #[error("invalid memory limit: {0} MB")]
    InvalidLimit(u32),

    #[error("Win32 call '{operation}' failed: {source}")]
    Win32 {
        operation: &'static str,
        #[source]
        source: windows::core::Error,
    },
}

pub type Result<T> = std::result::Result<T, SentryError>;
