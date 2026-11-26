// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use std::ffi::c_void;

use windows::Wdk::System::Threading::{NtQueryInformationProcess, ProcessBasicInformation};
use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::Memory::{
    QUOTA_LIMITS_HARDWS_MAX_DISABLE, QUOTA_LIMITS_HARDWS_MAX_ENABLE,
    QUOTA_LIMITS_HARDWS_MIN_DISABLE, SetProcessWorkingSetSizeEx,
};
use windows::Win32::System::ProcessStatus::{
    EmptyWorkingSet, GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
};
use windows::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
use windows::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, PEB, PROCESS_ACCESS_RIGHTS, PROCESS_BASIC_INFORMATION,
    PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA, PROCESS_VM_READ, RTL_USER_PROCESS_PARAMETERS,
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

#[derive(Debug, Clone, Copy)]
pub struct ProcessMetrics {
    pub working_set_mb: f64,
    pub page_fault_count: u64,
    pub cpu_time_ms: f64,
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

    fn times(&self) -> Result<(u64, f64)> {
        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();

        unsafe { GetProcessTimes(self.0, &mut creation, &mut exit, &mut kernel, &mut user) }
            .map_err(|source| SentryError::Win32 {
                operation: "GetProcessTimes",
                source,
            })?;

        let creation_time = crate::utils::filetime_to_u64(creation);
        let cpu_ticks = crate::utils::filetime_to_u64(kernel)
            .saturating_add(crate::utils::filetime_to_u64(user));
        let cpu_time_ms = cpu_ticks as f64 / 10_000.0;
        Ok((creation_time, cpu_time_ms))
    }

    fn creation_time(&self) -> Result<u64> {
        self.times().map(|(creation, _)| creation)
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

pub fn inspect(target: ProcessId) -> Result<ProcessMetrics> {
    let handle = ProcessHandle::open_verified(target, PROCESS_QUERY_INFORMATION)?;
    let (_, cpu_time_ms) = handle.times()?;
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
    Ok(ProcessMetrics {
        working_set_mb: counters.WorkingSetSize as f64 / (1024.0 * 1024.0),
        page_fault_count: counters.PageFaultCount as u64,
        cpu_time_ms,
    })
}

pub fn working_set_mb(target: ProcessId) -> Result<f64> {
    inspect(target).map(|m| m.working_set_mb)
}

fn read_remote_struct<T: Default>(handle: HANDLE, address: *const c_void) -> Result<T> {
    let mut value = T::default();
    unsafe {
        ReadProcessMemory(
            handle,
            address,
            (&raw mut value).cast::<c_void>(),
            std::mem::size_of::<T>(),
            None,
        )
    }
    .map_err(|source| SentryError::Win32 {
        operation: "ReadProcessMemory",
        source,
    })?;
    Ok(value)
}

pub fn command_line(target: ProcessId) -> Result<String> {
    let handle = ProcessHandle::open_verified(target, PROCESS_QUERY_INFORMATION | PROCESS_VM_READ)?;

    let mut pbi = PROCESS_BASIC_INFORMATION::default();
    let mut return_len = 0u32;
    let status = unsafe {
        NtQueryInformationProcess(
            handle.0,
            ProcessBasicInformation,
            (&raw mut pbi).cast::<c_void>(),
            std::mem::size_of::<PROCESS_BASIC_INFORMATION>() as u32,
            &raw mut return_len,
        )
    };
    status.ok().map_err(|source| SentryError::Win32 {
        operation: "NtQueryInformationProcess",
        source,
    })?;

    if pbi.PebBaseAddress.is_null() {
        return Ok(String::new());
    }

    let peb: PEB = read_remote_struct(handle.0, pbi.PebBaseAddress.cast::<c_void>())?;
    if peb.ProcessParameters.is_null() {
        return Ok(String::new());
    }

    let params: RTL_USER_PROCESS_PARAMETERS =
        read_remote_struct(handle.0, peb.ProcessParameters.cast::<c_void>())?;

    let byte_len = params.CommandLine.Length as usize;
    if byte_len == 0 || params.CommandLine.Buffer.is_null() {
        return Ok(String::new());
    }

    let char_len = byte_len / 2;
    let mut buffer = vec![0u16; char_len];
    unsafe {
        ReadProcessMemory(
            handle.0,
            params.CommandLine.Buffer.0.cast::<c_void>(),
            buffer.as_mut_ptr().cast::<c_void>(),
            char_len * 2,
            None,
        )
    }
    .map_err(|source| SentryError::Win32 {
        operation: "ReadProcessMemory",
        source,
    })?;

    Ok(String::from_utf16_lossy(&buffer))
}
