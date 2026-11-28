// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

/// Working-set limits and management policy for a single process role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleConfig {
    /// Minimum allowed cap in megabytes.
    pub min_mb: u32,
    /// Maximum allowed cap in megabytes.
    pub max_mb: u32,
    /// Initial cap applied when capping a process of this role, in megabytes.
    pub initial_mb: u32,
    /// Whether this role should be managed by callers.
    pub managed: bool,
}

impl RoleConfig {
    /// Creates a managed `RoleConfig` with `initial_mb` set to `max_mb`.
    ///
    /// # Panics
    ///
    /// Panics if `min_mb == 0` or `max_mb < min_mb`.
    pub const fn new(min_mb: u32, max_mb: u32) -> Self {
        assert!(min_mb >= 1, "min_mb must be at least 1");
        assert!(max_mb >= min_mb, "max_mb must be greater or equal min_mb");
        Self {
            min_mb,
            max_mb,
            initial_mb: max_mb,
            managed: true,
        }
    }

    /// Non-panicking version of [`RoleConfig::new`]. Returns `None` if `min_mb == 0` or `max_mb < min_mb`.
    pub const fn try_new(min_mb: u32, max_mb: u32) -> Option<Self> {
        if min_mb == 0 || max_mb < min_mb {
            return None;
        }
        Some(Self {
            min_mb,
            max_mb,
            initial_mb: max_mb,
            managed: true,
        })
    }

    /// Overrides the initial cap in megabytes.
    ///
    /// # Panics
    ///
    /// Panics if `initial_mb` is outside `self.min_mb..=self.max_mb`.
    pub const fn with_initial_mb(mut self, initial_mb: u32) -> Self {
        assert!(
            initial_mb >= self.min_mb && initial_mb <= self.max_mb,
            "initial_mb must be within min_mb..=max_mb"
        );
        self.initial_mb = initial_mb;
        self
    }

    /// Marks this role as unmanaged so batch actions skip matching processes.
    pub const fn unmanaged(mut self) -> Self {
        self.managed = false;
        self
    }
}

/// Condition evaluated against a process to decide whether it belongs to a role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchRule {
    /// Matches when the process owns an active audio session.
    IsAudio,
    /// Matches the root process of the application's process tree.
    IsRoot,
    /// Matches when the command line contains the given substring (ASCII case-insensitive).
    CmdContains(String),
    /// Matches unconditionally.
    Any,
}

impl MatchRule {
    /// Shorthand for [`MatchRule::CmdContains`].
    pub fn cmd_contains(pattern: impl Into<String>) -> Self {
        Self::CmdContains(pattern.into())
    }

    /// Evaluates the rule against a process's command line and flags.
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

/// Rule for deriving a PID-free sub-label from a process's command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubLabelRule {
    /// Extracts a flag's value and prepends `prefix` (for example, `--renderer-client-id=6` -> `#6`).
    FlagValue { flag: String, prefix: String },
    /// Extracts a flag's value and takes the lowercased segment before `separator`
    /// (for example, `--utility-sub-type=network.mojom.NetworkService` -> `network`).
    FlagHead { flag: String, separator: char },
}

impl SubLabelRule {
    /// Shorthand for [`SubLabelRule::FlagValue`].
    pub fn flag_value(flag: impl Into<String>, prefix: impl Into<String>) -> Self {
        Self::FlagValue {
            flag: flag.into(),
            prefix: prefix.into(),
        }
    }

    /// Shorthand for [`SubLabelRule::FlagHead`].
    pub fn flag_head(flag: impl Into<String>, separator: char) -> Self {
        Self::FlagHead {
            flag: flag.into(),
            separator,
        }
    }

    /// Extracts the sub-label from `cmd`, or returns `None` if the flag is absent or empty.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_rules() {
        let renderer = MatchRule::cmd_contains("--type=renderer");
        assert!(renderer.matches("app.exe --TYPE=Renderer --extra", false, false));
        assert!(!renderer.matches("app.exe --type=gpu-process", false, false));
        assert!(MatchRule::IsRoot.matches("app.exe", true, false));
        assert!(!MatchRule::IsRoot.matches("app.exe", false, false));
        assert!(MatchRule::IsAudio.matches("app.exe", false, true));
        assert!(MatchRule::Any.matches("", false, false));
    }

    #[test]
    fn sub_label_extraction() {
        let renderer = SubLabelRule::flag_value("--renderer-client-id", "#");
        assert_eq!(
            renderer.extract("app.exe --type=renderer --renderer-client-id=6"),
            Some("#6".to_string())
        );
        assert_eq!(
            renderer.extract("app.exe --renderer-client-id 12"),
            Some("#12".to_string())
        );
        assert_eq!(renderer.extract("app.exe --type=renderer"), None);

        let utility = SubLabelRule::flag_head("--utility-sub-type", '.');
        assert_eq!(
            utility.extract("app.exe --utility-sub-type=network.mojom.NetworkService"),
            Some("network".to_string())
        );
        assert_eq!(utility.extract("app.exe --type=utility"), None);
    }

    #[test]
    fn role_config_try_new_validates_bounds() {
        assert_eq!(RoleConfig::try_new(0, 10), None);
        assert_eq!(RoleConfig::try_new(20, 10), None);
        let cfg = RoleConfig::try_new(4, 32).expect("valid bounds");
        assert_eq!(cfg.min_mb, 4);
        assert_eq!(cfg.max_mb, 32);
        assert_eq!(cfg.initial_mb, 32);
        assert!(cfg.managed);
    }

    #[test]
    #[should_panic(expected = "min_mb must be at least 1")]
    fn role_config_new_panics_on_zero_min() {
        let _ = RoleConfig::new(0, 10);
    }

    #[test]
    #[should_panic(expected = "max_mb must be greater or equal min_mb")]
    fn role_config_new_panics_on_min_greater_that_max() {
        let _ = RoleConfig::new(10, 7);
    }

    #[test]
    #[should_panic(expected = "initial_mb must be within min_mb..=max_mb")]
    fn role_config_with_initial_out_of_range_panics() {
        let _ = RoleConfig::new(8, 32).with_initial_mb(64);
    }
}
