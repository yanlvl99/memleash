// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleConfig {
    pub min_mb: u32,
    pub max_mb: u32,
    pub initial_mb: u32,
    pub managed: bool,
}

impl RoleConfig {
    pub const fn new(min_mb: u32, max_mb: u32) -> Self {
        Self {
            min_mb,
            max_mb,
            initial_mb: max_mb,
            managed: true,
        }
    }

    pub const fn with_initial_mb(mut self, initial_mb: u32) -> Self {
        self.initial_mb = initial_mb;
        self
    }

    pub const fn unmanaged(mut self) -> Self {
        self.managed = false;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchRule {
    IsAudio,
    IsRoot,
    CmdContains(String),
    Any,
}

impl MatchRule {
    pub fn cmd_contains(pattern: impl Into<String>) -> Self {
        Self::CmdContains(pattern.into())
    }

    pub fn matches(&self, cmd: &str, is_root: bool, is_audio: bool) -> bool {
        match self {
            Self::IsAudio => is_audio,
            Self::IsRoot => is_root,
            Self::CmdContains(pattern) => cmd
                .to_ascii_lowercase()
                .contains(&pattern.to_ascii_lowercase()),
            Self::Any => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubLabelRule {
    FlagValue { flag: String, prefix: String },
    FlagHead { flag: String, separator: char },
}

impl SubLabelRule {
    pub fn flag_value(flag: impl Into<String>, prefix: impl Into<String>) -> Self {
        Self::FlagValue {
            flag: flag.into(),
            prefix: prefix.into(),
        }
    }

    pub fn flag_head(flag: impl Into<String>, separator: char) -> Self {
        Self::FlagHead {
            flag: flag.into(),
            separator,
        }
    }

    pub fn extract(&self, cmd: &str) -> Option<String> {
        match self {
            Self::FlagValue { flag, prefix } => {
                let value = extract_flag_value(cmd, flag)?;
                Some(format!("{prefix}{value}"))
            }
            Self::FlagHead { flag, separator } => {
                let value = extract_flag_value(cmd, flag)?;
                let head = value.split(*separator).next().unwrap_or("").trim();
                (!head.is_empty()).then(|| head.to_ascii_lowercase())
            }
        }
    }
}

fn extract_flag_value(cmd: &str, flag: &str) -> Option<String> {
    let lower = cmd.to_ascii_lowercase();
    let lower_flag = flag.to_ascii_lowercase();
    let index = lower.find(&lower_flag)?;
    let after = cmd[index + lower_flag.len()..].trim_start();
    let after = after.strip_prefix('=').unwrap_or(after).trim_start();
    let token = after.split_whitespace().next()?;
    let token = token.trim_matches('"');
    (!token.is_empty()).then(|| token.to_string())
}
