// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use std::collections::HashSet;

use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::Media::Audio::{
    AudioSessionStateActive, DEVICE_STATE_ACTIVE, IAudioSessionControl2, IAudioSessionManager2,
    IMMDeviceEnumerator, MMDeviceEnumerator, eRender,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::core::Interface;

use crate::error::{Error, Result};

struct ComGuard {
    should_uninit: bool,
}

impl ComGuard {
    fn init() -> Result<Self> {
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if hr.is_ok() {
            Ok(Self {
                should_uninit: true,
            })
        } else if hr == RPC_E_CHANGED_MODE {
            Ok(Self {
                should_uninit: false,
            })
        } else {
            Err(Error::Win32 {
                operation: "CoInitializeEx",
                source: hr.into(),
            })
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.should_uninit {
            unsafe { CoUninitialize() };
        }
    }
}

/// Returns the PIDs currently owning an `AudioSessionStateActive`
/// [WASAPI](https://learn.microsoft.com/en-us/windows/win32/coreaudio/wasapi) render session.
///
/// # Errors
///
/// Returns [`Error::Win32`] if COM initialization, `MMDeviceEnumerator` creation,
/// or endpoint enumeration fails.
pub fn active_audio_pids() -> Result<HashSet<u32>> {
    let _com = ComGuard::init()?;

    let enumerator: IMMDeviceEnumerator = unsafe {
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
    }
    .map_err(|source| Error::Win32 {
        operation: "CoCreateInstance(MMDeviceEnumerator)",
        source,
    })?;

    let devices = unsafe { enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) }.map_err(
        |source| Error::Win32 {
            operation: "EnumAudioEndpoints",
            source,
        },
    )?;

    let device_count = unsafe { devices.GetCount() }.map_err(|source| Error::Win32 {
        operation: "IMMDeviceCollection::GetCount",
        source,
    })?;

    let mut pids = HashSet::new();

    for i in 0..device_count {
        let Ok(device) = (unsafe { devices.Item(i) }) else {
            continue;
        };
        let Ok(manager) = (unsafe { device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) })
        else {
            continue;
        };
        let Ok(sessions) = (unsafe { manager.GetSessionEnumerator() }) else {
            continue;
        };
        let Ok(count) = (unsafe { sessions.GetCount() }) else {
            continue;
        };

        for j in 0..count {
            let Ok(control) = (unsafe { sessions.GetSession(j) }) else {
                continue;
            };
            let Ok(state) = (unsafe { control.GetState() }) else {
                continue;
            };
            if state != AudioSessionStateActive {
                continue;
            }
            let Ok(control2) = control.cast::<IAudioSessionControl2>() else {
                continue;
            };
            if let Ok(pid) = unsafe { control2.GetProcessId() }
                && pid != 0
            {
                pids.insert(pid);
            }
        }
    }

    Ok(pids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_audio_pids_returns_ok_without_zero_pid() {
        let pids = active_audio_pids().expect("active_audio_pids");
        assert!(!pids.contains(&0));
    }
}
