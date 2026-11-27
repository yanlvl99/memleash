// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use windows::Win32::Foundation::FILETIME;

pub const fn mb_to_bytes(mb: usize) -> usize {
    mb.saturating_mul(1024 * 1024)
}

pub const fn filetime_to_u64(ft: FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64
}

pub fn wide_to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mb_to_bytes_converts_and_saturates() {
        assert_eq!(mb_to_bytes(0), 0);
        assert_eq!(mb_to_bytes(16), 16 * 1024 * 1024);
        assert_eq!(mb_to_bytes(usize::MAX), usize::MAX);
    }

    #[test]
    fn wide_to_string_stops_at_nul() {
        let with_nul: Vec<u16> = "app.exe\0ignored".encode_utf16().collect();
        assert_eq!(wide_to_string(&with_nul), "app.exe");

        let no_nul: Vec<u16> = "abc".encode_utf16().collect();
        assert_eq!(wide_to_string(&no_nul), "abc");
    }

    #[test]
    fn filetime_combines_high_and_low_words() {
        let ft = FILETIME {
            dwLowDateTime: 0x0000_0005,
            dwHighDateTime: 0x0000_0002,
        };
        assert_eq!(filetime_to_u64(ft), (2u64 << 32) | 5);
    }
}
