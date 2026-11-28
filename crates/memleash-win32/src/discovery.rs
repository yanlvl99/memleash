// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use crate::error::{Error, Result};
use crate::process::ProcessId;
use crate::utils::wide_to_string;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};

struct SnapshotHandle(HANDLE);

impl Drop for SnapshotHandle {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub id: ProcessId,
    pub parent_pid: u32,
    pub exe_name: String,
}

pub fn find_by_name(exe_name: &str) -> Result<Vec<ProcessInfo>> {
    let handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }.map_err(|source| {
        Error::Win32 {
            operation: "CreateToolhelp32Snapshot",
            source,
        }
    })?;
    let snapshot = SnapshotHandle(handle);

    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };

    if unsafe { Process32FirstW(snapshot.0, &mut entry) }.is_err() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    loop {
        let name = wide_to_string(&entry.szExeFile);
        if name.eq_ignore_ascii_case(exe_name)
            && let Ok(id) = ProcessId::from_pid(entry.th32ProcessID)
        {
            results.push(ProcessInfo {
                id,
                parent_pid: entry.th32ParentProcessID,
                exe_name: name,
            });
        }
        if unsafe { Process32NextW(snapshot.0, &mut entry) }.is_err() {
            break;
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_by_name_finds_current_exe_and_returns_empty_for_missing() {
        let exe = std::env::current_exe().expect("current_exe");
        let name = exe
            .file_name()
            .expect("exe file name")
            .to_string_lossy()
            .into_owned();
        let current_pid = std::process::id();

        let found = find_by_name(&name.to_ascii_uppercase()).expect("find_by_name");
        assert!(found.iter().any(|p| p.id.pid == current_pid));

        let missing = find_by_name("__memleash_nonexistent__.exe").expect("find missing");
        assert!(missing.is_empty());
    }
}
