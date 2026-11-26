// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

pub mod discovery;
pub mod error;
pub mod process;
pub mod utils;
pub mod window;
pub use discovery::{ProcessInfo, find_by_name};
pub use error::{Result, SentryError};
pub use process::{
    ProcessId, ProcessMetrics, cap, empty_working_set, inspect, release, working_set_mb,
};
pub use window::foreground_pid;
