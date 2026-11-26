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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classification<'a> {
    pub role: &'a str,
    pub sub_label: Option<String>,
    pub identity: String,
    pub config: &'a RoleConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppProfile {
    exe_name: String,
    roles: Vec<RoleDefinition>,
    fallback_name: String,
    fallback_config: RoleConfig,
}

impl AppProfile {
    pub fn builder(exe_name: impl Into<String>) -> AppProfileBuilder {
        AppProfileBuilder {
            exe_name: exe_name.into(),
            roles: Vec::new(),
            fallback_name: "Helper".to_string(),
            fallback_config: RoleConfig::new(4, 48),
        }
    }

    pub fn single(exe_name: impl Into<String>, config: RoleConfig) -> Self {
        Self::builder(exe_name)
            .role("Main", MatchRule::IsRoot, config)
            .fallback("Main", config)
            .build()
    }

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

    pub fn exe_name(&self) -> &str {
        &self.exe_name
    }

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

#[derive(Debug, Clone)]
pub struct AppProfileBuilder {
    exe_name: String,
    roles: Vec<RoleDefinition>,
    fallback_name: String,
    fallback_config: RoleConfig,
}

impl AppProfileBuilder {
    pub fn role(
        mut self,
        name: impl Into<String>,
        rule: MatchRule,
        config: RoleConfig,
    ) -> Self {
        self.roles.push(RoleDefinition {
            name: name.into(),
            rule,
            sub_label: None,
            config,
        });
        self
    }

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

    pub fn fallback(mut self, name: impl Into<String>, config: RoleConfig) -> Self {
        self.fallback_name = name.into();
        self.fallback_config = config;
        self
    }

    pub fn build(self) -> AppProfile {
        AppProfile {
            exe_name: self.exe_name,
            roles: self.roles,
            fallback_name: self.fallback_name,
            fallback_config: self.fallback_config,
        }
    }
}
