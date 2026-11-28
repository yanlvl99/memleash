// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

//! Process role matching, flag-based sub-identities, and application profiles.
//!
//! This crate has no Win32 dependencies; callers supply the process command line,
//! root status, and active-audio flag to [`AppProfile::classify`].

pub mod profile;
pub mod role;

pub use profile::{AppProfile, AppProfileBuilder, Classification};
pub use role::{MatchRule, RoleConfig, SubLabelRule};
