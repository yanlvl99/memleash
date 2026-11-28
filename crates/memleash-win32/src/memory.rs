// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Memory::{
    CreateMemoryResourceNotification, HighMemoryResourceNotification,
    LowMemoryResourceNotification, QueryMemoryResourceNotification,
};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::core::BOOL;

use crate::error::{Error, Result};

const MIB: u64 = 1024 * 1024;

/// Physical RAM figures reported by `GlobalMemoryStatusEx`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemMemory {
    /// Total installed physical memory in MiB.
    pub total_mb: u64,
    /// Currently available physical memory in MiB.
    pub available_mb: u64,
    /// Approximate percentage of physical memory in use (`0..=100`).
    pub load_percent: u32,
}

/// Kernel memory-availability state reported by Windows memory resource notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAvailability {
    /// `LowMemoryResourceNotification` is signaled (physical RAM is low).
    Low,
    /// Neither low nor high notification is signaled.
    Normal,
    /// `HighMemoryResourceNotification` is signaled (abundant physical RAM is available).
    High,
}

/// Reads system-wide physical memory totals and load via
/// [`GlobalMemoryStatusEx`](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-globalmemorystatusex).
///
/// # Errors
///
/// Returns [`Error::Win32`] if `GlobalMemoryStatusEx` fails.
pub fn system_memory() -> Result<SystemMemory> {
    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe { GlobalMemoryStatusEx(&mut status) }.map_err(|source| Error::Win32 {
        operation: "GlobalMemoryStatusEx",
        source,
    })?;

    Ok(SystemMemory {
        total_mb: status.ullTotalPhys / MIB,
        available_mb: status.ullAvailPhys / MIB,
        load_percent: status.dwMemoryLoad,
    })
}

/// Holds Windows low/high memory resource notification handles and closes them on drop.
pub struct MemoryWatcher {
    low: HANDLE,
    high: HANDLE,
}

// SAFETY: Memory resource notification handles are kernel objects safe to query and close from any thread.
unsafe impl Send for MemoryWatcher {}
unsafe impl Sync for MemoryWatcher {}

impl MemoryWatcher {
    /// Creates both `LowMemoryResourceNotification` and `HighMemoryResourceNotification` handles via
    /// [`CreateMemoryResourceNotification`](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-creatememoryresourcenotification).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Win32`] if `CreateMemoryResourceNotification` fails.
    pub fn new() -> Result<Self> {
        let low = unsafe { CreateMemoryResourceNotification(LowMemoryResourceNotification) }
            .map_err(|source| Error::Win32 {
                operation: "CreateMemoryResourceNotification(Low)",
                source,
            })?;
        let high = match unsafe { CreateMemoryResourceNotification(HighMemoryResourceNotification) }
        {
            Ok(handle) => handle,
            Err(source) => {
                let _ = unsafe { CloseHandle(low) };
                return Err(Error::Win32 {
                    operation: "CreateMemoryResourceNotification(High)",
                    source,
                });
            }
        };
        Ok(Self { low, high })
    }

    /// Queries both notification handles without blocking and returns the current [`MemoryAvailability`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::Win32`] if `QueryMemoryResourceNotification` fails.
    pub fn availability(&self) -> Result<MemoryAvailability> {
        let mut state = BOOL(0);
        unsafe { QueryMemoryResourceNotification(self.low, &mut state) }.map_err(|source| {
            Error::Win32 {
                operation: "QueryMemoryResourceNotification(Low)",
                source,
            }
        })?;
        if state.as_bool() {
            return Ok(MemoryAvailability::Low);
        }

        state = BOOL(0);
        unsafe { QueryMemoryResourceNotification(self.high, &mut state) }.map_err(|source| {
            Error::Win32 {
                operation: "QueryMemoryResourceNotification(High)",
                source,
            }
        })?;
        if state.as_bool() {
            return Ok(MemoryAvailability::High);
        }

        Ok(MemoryAvailability::Normal)
    }
}

impl Drop for MemoryWatcher {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.low) };
        let _ = unsafe { CloseHandle(self.high) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_memory_and_watcher_return_valid_values() {
        let mem = system_memory().expect("system_memory");
        assert!(mem.total_mb > 0);
        assert!(mem.available_mb <= mem.total_mb);
        assert!(mem.load_percent <= 100);

        let watcher = MemoryWatcher::new().expect("MemoryWatcher::new");
        let _ = watcher.availability().expect("availability");
    }

    #[test]
    fn memory_watcher_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MemoryWatcher>();
    }
}
