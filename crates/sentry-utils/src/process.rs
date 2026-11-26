// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
use windows::Win32::System::Memory::{
    QUOTA_LIMITS_HARDWS_MAX_DISABLE, QUOTA_LIMITS_HARDWS_MAX_ENABLE,
    QUOTA_LIMITS_HARDWS_MIN_DISABLE, SetProcessWorkingSetSizeEx,
};
use windows::Win32::System::ProcessStatus::{
    EmptyWorkingSet, GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
};
use windows::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
use windows::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, PROCESS_ACCESS_RIGHTS, PROCESS_QUERY_INFORMATION,
    PROCESS_SET_QUOTA,
};

use crate::error::{Result, SentryError};

const DEFAULT_MIN_PAGES: usize = 50;
const DEFAULT_MAX_PAGES: usize = 345;
const MIN_HARD_CAP_PAGES: usize = 20;

struct ProcessHandle(HANDLE);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProcessId {
    pub pid: u32,
    pub creation_time: u64,
}

impl ProcessId {
    pub fn from_pid(pid: u32) -> Result<Self> {
        let handle = ProcessHandle::open(pid, PROCESS_QUERY_INFORMATION)?;
        let creation_time = handle.creation_time()?;
        Ok(Self { pid, creation_time })
    }
}

impl ProcessHandle {
    fn open(pid: u32, access: PROCESS_ACCESS_RIGHTS) -> Result<Self> {
        let handle = unsafe { OpenProcess(access, false, pid) }
            .map_err(|source| SentryError::OpenProcess { pid, source })?;
        Ok(Self(handle))
    }

    fn creation_time(&self) -> Result<u64> {
        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();

        unsafe { GetProcessTimes(self.0, &mut creation, &mut exit, &mut kernel, &mut user) }
            .map_err(|source| SentryError::Win32 {
                operation: "GetProcessTimes",
                source,
            })?;
        Ok(((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64)
    }

    fn open_verified(target: ProcessId, access: PROCESS_ACCESS_RIGHTS) -> Result<Self> {
        let handle = Self::open(target.pid, access)?;
        let actual = handle.creation_time()?;
        if actual != target.creation_time {
            return Err(SentryError::PidReused {
                pid: target.pid,
                expected: target.creation_time,
                actual,
            });
        }
        Ok(handle)
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

fn page_size() -> usize {
    let mut info = SYSTEM_INFO::default();
    unsafe { GetSystemInfo(&mut info) };
    (info.dwPageSize as usize).max(4096)
}

pub fn empty_working_set(target: ProcessId) -> Result<()> {
    let handle =
        ProcessHandle::open_verified(target, PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA)?;
    unsafe { EmptyWorkingSet(handle.0) }.map_err(|source| SentryError::Win32 {
        operation: "EmptyWorkingSet",
        source,
    })?;
    Ok(())
}

pub fn cap(target: ProcessId, max_mb: u32) -> Result<()> {
    if max_mb == 0 {
        return Err(SentryError::InvalidLimit(max_mb));
    }
    let max_bytes = crate::utils::mb_to_bytes(max_mb as usize);
    let min_bytes = (MIN_HARD_CAP_PAGES * page_size()).min(max_bytes);
    let handle =
        ProcessHandle::open_verified(target, PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA)?;
    unsafe {
        SetProcessWorkingSetSizeEx(
            handle.0,
            min_bytes,
            max_bytes,
            QUOTA_LIMITS_HARDWS_MIN_DISABLE | QUOTA_LIMITS_HARDWS_MAX_ENABLE,
        )
    }
    .map_err(|source| SentryError::Win32 {
        operation: "SetProcessWorkingSetSizeEx",
        source,
    })?;
    Ok(())
}

pub fn release(target: ProcessId) -> Result<()> {
    let page = page_size();
    let min_bytes = DEFAULT_MIN_PAGES * page;
    let max_bytes = DEFAULT_MAX_PAGES * page;
    let handle =
        ProcessHandle::open_verified(target, PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA)?;
    unsafe {
        SetProcessWorkingSetSizeEx(
            handle.0,
            min_bytes,
            max_bytes,
            QUOTA_LIMITS_HARDWS_MIN_DISABLE | QUOTA_LIMITS_HARDWS_MAX_DISABLE,
        )
    }
    .map_err(|source| SentryError::Win32 {
        operation: "SetProcessWorkingSetSizeEx",
        source,
    })?;
    Ok(())
}

pub fn working_set_mb(target: ProcessId) -> Result<f64> {
    let handle = ProcessHandle::open_verified(target, PROCESS_QUERY_INFORMATION)?;
    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ..Default::default()
    };
    unsafe { GetProcessMemoryInfo(handle.0, &mut counters, counters.cb) }.map_err(|source| {
        SentryError::Win32 {
            operation: "GetProcessMemoryInfo",
            source,
        }
    })?;
    Ok(counters.WorkingSetSize as f64 / (1024.0 * 1024.0))
}
