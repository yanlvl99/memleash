// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

pub mod audio;
pub mod discovery;
pub mod error;
pub mod memory;
pub mod process;
pub mod utils;
pub mod window;
pub use audio::active_audio_pids;
pub use discovery::{ProcessInfo, find_by_name};
pub use error::{Error, Result};
pub use memory::{MemoryAvailability, MemoryWatcher, SystemMemory, system_memory};
pub use process::{
    ProcessId, ProcessMetrics, cap, command_line, empty_working_set, inspect, release,
    working_set_mb,
};
pub use window::{
    WindowCandidate, WindowState, classify_windows, find_windows, foreground_pid, window_state,
};
