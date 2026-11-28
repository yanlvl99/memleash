// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use thiserror::Error;

/// Errors returned by Win32/WDK/COM operations in `memleash-win32`.
#[derive(Debug, Error)]
pub enum Error {
    /// `OpenProcess` failed for the given PID (for example, access denied or process exited).
    #[error("failed to open process {pid}: {source}")]
    OpenProcess {
        pid: u32,
        #[source]
        source: windows::core::Error,
    },

    /// The target PID now belongs to a different process whose creation time does not match.
    #[error("process PID {pid} was reused (expected creation time {expected}, got {actual})")]
    PidReused {
        pid: u32,
        expected: u64,
        actual: u64,
    },

    /// Caller passed an invalid memory limit (such as `0` MB).
    #[error("invalid memory limit: {0} MB")]
    InvalidLimit(u32),

    /// A Win32, WDK, or COM call failed.
    #[error("Win32 call '{operation}' failed: {source}")]
    Win32 {
        operation: &'static str,
        #[source]
        source: windows::core::Error,
    },
}

/// Convenience `Result` alias using [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
