// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use std::ffi::c_void;

use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GWL_EXSTYLE, GetForegroundWindow, GetWindow, GetWindowLongW,
    GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
    IsWindow, IsWindowVisible, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};
use windows::core::BOOL;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Foreground,
    BackgroundVisible,
    Minimized,
    Hidden,
    NotFound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowCandidate {
    pub pid: u32,
    pub hwnd: usize,
    pub title: String,
    pub on_screen: bool,
    pub iconic: bool,
    pub area: i64,
}

pub fn foreground_pid() -> Option<u32> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return None;
    }
    let mut pid = 0u32;
    let thread_id = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if thread_id == 0 || pid == 0 {
        None
    } else {
        Some(pid)
    }
}

fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked = 0u32;
    unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&raw mut cloaked).cast::<c_void>(),
            std::mem::size_of::<u32>() as u32,
        )
        .is_ok()
            && cloaked != 0
    }
}

struct WindowScan<'a> {
    pids: &'a [u32],
    found: Vec<WindowCandidate>,
}

unsafe extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let scan = unsafe { &mut *(lparam.0 as *mut WindowScan<'_>) };
    if unsafe { !IsWindow(Some(hwnd)).as_bool() } {
        return BOOL(1);
    }

    let mut pid = 0u32;
    let thread_id = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if thread_id == 0 || !scan.pids.contains(&pid) {
        return BOOL(1);
    }

    if let Ok(owner) = unsafe { GetWindow(hwnd, GW_OWNER) } {
        if !owner.0.is_null() {
            return BOOL(1);
        }
    }

    let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32;
    if ex_style & (WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) != 0 {
        return BOOL(1);
    }

    let title_len = unsafe { GetWindowTextLengthW(hwnd) };
    if title_len <= 0 {
        return BOOL(1);
    }

    let mut buffer = vec![0u16; title_len as usize + 1];
    let written = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    if written <= 0 {
        return BOOL(1);
    }
    let title = String::from_utf16_lossy(&buffer[..written as usize]);

    let visible = unsafe { IsWindowVisible(hwnd).as_bool() };
    let on_screen = visible && !is_cloaked(hwnd);
    let iconic = unsafe { IsIconic(hwnd).as_bool() };

    let mut rect = RECT::default();
    let area = if unsafe { GetWindowRect(hwnd, &mut rect) }.is_ok() {
        let width = i64::from(rect.right - rect.left).max(0);
        let height = i64::from(rect.bottom - rect.top).max(0);
        width.saturating_mul(height)
    } else {
        0
    };

    scan.found.push(WindowCandidate {
        pid,
        hwnd: hwnd.0 as usize,
        title,
        on_screen,
        iconic,
        area,
    });

    BOOL(1)
}

pub fn find_windows(pids: &[u32]) -> Vec<WindowCandidate> {
    if pids.is_empty() {
        return Vec::new();
    }
    let mut scan = WindowScan {
        pids,
        found: Vec::new(),
    };
    let _ = unsafe {
        EnumWindows(
            Some(enum_window_callback),
            LPARAM((&raw mut scan) as isize),
        )
    };
    scan.found
}

pub fn classify_windows(
    candidates: &[WindowCandidate],
    is_foreground: bool,
) -> (WindowState, Option<&WindowCandidate>) {
    let best_visible = candidates
        .iter()
        .filter(|w| w.on_screen && !w.iconic && w.area > 0)
        .max_by_key(|w| w.area);

    if is_foreground {
        let rep = best_visible.or_else(|| candidates.iter().max_by_key(|w| w.area));
        return (WindowState::Foreground, rep);
    }

    if let Some(win) = best_visible {
        return (WindowState::BackgroundVisible, Some(win));
    }

    if let Some(win) = candidates.iter().find(|w| w.on_screen && w.iconic) {
        return (WindowState::Minimized, Some(win));
    }

    if let Some(win) = candidates.iter().max_by_key(|w| w.area) {
        return (WindowState::Hidden, Some(win));
    }

    (WindowState::NotFound, None)
}

pub fn window_state(pids: &[u32]) -> (WindowState, Option<WindowCandidate>) {
    let is_foreground = foreground_pid().is_some_and(|fg| pids.contains(&fg));
    let candidates = find_windows(pids);
    let (state, rep) = classify_windows(&candidates, is_foreground);
    (state, rep.cloned())
}
