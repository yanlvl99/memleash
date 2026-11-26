// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

pub const fn mb_to_bytes(mb: usize) -> usize {
    mb.saturating_mul(1024 * 1024)
}
