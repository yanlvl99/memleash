// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 yanlvl99

use crate::role::{MatchRule, RoleConfig, SubLabelRule};

#[derive(Debug, Clone, PartialEq, Eq)]
struct RoleDefinition {
    name: String,
    rule: MatchRule,
    sub_label: Option<SubLabelRule>,
    config: RoleConfig,
}

/// Result of classifying a process against an [`AppProfile`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classification<'a> {
    /// Matched role name (e.g. `"Main"`, `"Renderer"`, `"Utility"`).
    pub role: &'a str,
    /// Optional sub-label extracted from the command line (e.g. `"#6"`, `"network"`).
    pub sub_label: Option<String>,
    /// Stable PID-free key: `"{role}:{sub_label}"` when a sub-label is present, otherwise `role`.
    pub identity: String,
    /// Limits and management policy for the matched role.
    pub config: &'a RoleConfig,
}

/// Ordered set of rules describing how to classify a target executable's processes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppProfile {
    exe_name: String,
    roles: Vec<RoleDefinition>,
    fallback_name: String,
    fallback_config: RoleConfig,
}

impl AppProfile {
    /// Starts building a custom profile for `exe_name` (e.g. `"Chrome.exe"`).
    pub fn builder(exe_name: impl Into<String>) -> AppProfileBuilder {
        AppProfileBuilder {
            exe_name: exe_name.into(),
            roles: Vec::new(),
            fallback_name: "Helper".to_string(),
            fallback_config: RoleConfig::new(4, 48),
        }
    }

    /// Profile for a single-process application where every process maps to `"Main"`.
    pub fn single(exe_name: impl Into<String>, config: RoleConfig) -> Self {
        Self::builder(exe_name)
            .role("Main", MatchRule::IsRoot, config)
            .fallback("Main", config)
            .build()
    }

    /// Preset for Chromium, Electron, and CEF applications (Chrome, Edge, Discord, Spotify, VS Code).
    ///
    /// Rules are evaluated in order: active audio (`Audio`), `audio.mojom.audioservice` (`Audio`),
    /// root process (`Main`), `--type=renderer` (`Renderer:#<id>`), `--type=gpu-process` (`GPU`),
    /// `--type=utility` (`Utility:<service>`), `crashpad` (`Crashpad`), and fallback (`Helper`).
    pub fn chromium(exe_name: impl Into<String>) -> Self {
        Self::builder(exe_name)
            .role("Audio", MatchRule::IsAudio, RoleConfig::new(25, 60))
            .role(
                "Audio",
                MatchRule::cmd_contains("audio.mojom.audioservice"),
                RoleConfig::new(25, 60),
            )
            .role("Main", MatchRule::IsRoot, RoleConfig::new(4, 38))
            .role_with_sub(
                "Renderer",
                MatchRule::cmd_contains("--type=renderer"),
                SubLabelRule::flag_value("--renderer-client-id", "#"),
                RoleConfig::new(4, 52),
            )
            .role(
                "GPU",
                MatchRule::cmd_contains("--type=gpu-process"),
                RoleConfig::new(4, 52),
            )
            .role_with_sub(
                "Utility",
                MatchRule::cmd_contains("--type=utility"),
                SubLabelRule::flag_head("--utility-sub-type", '.'),
                RoleConfig::new(4, 48),
            )
            .role(
                "Crashpad",
                MatchRule::cmd_contains("crashpad"),
                RoleConfig::new(4, 16),
            )
            .fallback("Helper", RoleConfig::new(4, 48))
            .build()
    }

    /// Target executable file name (e.g. `"Chrome.exe"`).
    pub fn exe_name(&self) -> &str {
        &self.exe_name
    }

    /// Classifies a process against the profile's rules in registration order.
    ///
    /// Returns the first matching rule, or the fallback role if none match.
    pub fn classify(&self, cmd: &str, is_root: bool, is_audio: bool) -> Classification<'_> {
        for def in &self.roles {
            if def.rule.matches(cmd, is_root, is_audio) {
                let sub_label = def.sub_label.as_ref().and_then(|rule| rule.extract(cmd));
                let identity = match &sub_label {
                    Some(sub) => format!("{}:{sub}", def.name),
                    None => def.name.clone(),
                };
                return Classification {
                    role: &def.name,
                    sub_label,
                    identity,
                    config: &def.config,
                };
            }
        }

        Classification {
            role: &self.fallback_name,
            sub_label: None,
            identity: self.fallback_name.clone(),
            config: &self.fallback_config,
        }
    }
}

/// Builder for constructing an [`AppProfile`] with custom role rules.
#[derive(Debug, Clone)]
pub struct AppProfileBuilder {
    exe_name: String,
    roles: Vec<RoleDefinition>,
    fallback_name: String,
    fallback_config: RoleConfig,
}

impl AppProfileBuilder {
    /// Appends a role rule. Earlier rules take precedence during [`AppProfile::classify`].
    pub fn role(mut self, name: impl Into<String>, rule: MatchRule, config: RoleConfig) -> Self {
        self.roles.push(RoleDefinition {
            name: name.into(),
            rule,
            sub_label: None,
            config,
        });
        self
    }

    /// Appends a role rule that also extracts a sub-label via `sub_label`.
    pub fn role_with_sub(
        mut self,
        name: impl Into<String>,
        rule: MatchRule,
        sub_label: SubLabelRule,
        config: RoleConfig,
    ) -> Self {
        self.roles.push(RoleDefinition {
            name: name.into(),
            rule,
            sub_label: Some(sub_label),
            config,
        });
        self
    }

    /// Sets the fallback role used when no rule matches (defaults to `"Helper"` with `4..=48` MB).
    pub fn fallback(mut self, name: impl Into<String>, config: RoleConfig) -> Self {
        self.fallback_name = name.into();
        self.fallback_config = config;
        self
    }

    /// Finishes building the [`AppProfile`].
    pub fn build(self) -> AppProfile {
        AppProfile {
            exe_name: self.exe_name,
            roles: self.roles,
            fallback_name: self.fallback_name,
            fallback_config: self.fallback_config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chromium_preset_classification() {
        let profile = AppProfile::chromium("app.exe");

        let main = profile.classify("app.exe", true, false);
        assert_eq!(main.identity, "Main");
        assert_eq!(main.config.max_mb, 38);

        let audio = profile.classify("app.exe --type=renderer", true, true);
        assert_eq!(audio.identity, "Audio");
        assert_eq!(audio.config.min_mb, 25);

        let renderer = profile.classify(
            "app.exe --type=renderer --renderer-client-id=6",
            false,
            false,
        );
        assert_eq!(renderer.role, "Renderer");
        assert_eq!(renderer.sub_label.as_deref(), Some("#6"));
        assert_eq!(renderer.identity, "Renderer:#6");

        let utility = profile.classify(
            "app.exe --type=utility --utility-sub-type=network.mojom.NetworkService",
            false,
            false,
        );
        assert_eq!(utility.identity, "Utility:network");

        let gpu = profile.classify("app.exe --type=gpu-process", false, false);
        assert_eq!(gpu.identity, "GPU");

        let crashpad = profile.classify("app.exe --type=crashpad-handler", false, false);
        assert_eq!(crashpad.identity, "Crashpad");

        let fallback = profile.classify("app.exe --something-else", false, false);
        assert_eq!(fallback.identity, "Helper");
    }

    #[test]
    fn single_preset_and_unmanaged_role() {
        let cfg = RoleConfig::new(8, 64).with_initial_mb(32).unmanaged();
        let profile = AppProfile::single("game.exe", cfg);
        let class = profile.classify("game.exe", true, false);
        assert_eq!(class.identity, "Main");
        assert_eq!(class.config.initial_mb, 32);
        assert!(!class.config.managed);
    }

    #[test]
    fn chromium_audio_service_flag_and_custom_builder() {
        let chromium = AppProfile::chromium("app.exe");
        let audio = chromium.classify(
            "app.exe --type=utility --utility-sub-type=audio.mojom.AudioService",
            false,
            false,
        );
        assert_eq!(audio.identity, "Audio");
        assert_eq!(audio.config.min_mb, 25);

        let custom = AppProfile::builder("custom.exe")
            .role(
                "Worker",
                MatchRule::cmd_contains("--worker"),
                RoleConfig::new(8, 32),
            )
            .fallback("Idle", RoleConfig::new(4, 16))
            .build();
        assert_eq!(custom.exe_name(), "custom.exe");
        assert_eq!(
            custom
                .classify("custom.exe --worker", false, false)
                .identity,
            "Worker"
        );
        assert_eq!(custom.classify("custom.exe", false, false).identity, "Idle");
    }
}
