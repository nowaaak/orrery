use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::platform::{Arch, Os, Platform};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Allow,
    Disallow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: RuleAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<OsRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Features {
    pub has_custom_resolution: bool,
    pub is_quick_play_singleplayer: bool,
    pub is_quick_play_multiplayer: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct RuleContext {
    pub platform: Platform,
    pub features: Features,
}

impl RuleContext {
    pub fn new(platform: Platform) -> Self {
        Self {
            platform,
            features: Features::default(),
        }
    }
}

pub fn os_key(os: Os) -> &'static str {
    match os {
        Os::Windows => "windows",
        Os::MacOs => "osx",
        Os::Linux => "linux",
    }
}

pub fn rules_allow(rules: Option<&[Rule]>, ctx: &RuleContext) -> bool {
    let Some(rules) = rules.filter(|rules| !rules.is_empty()) else {
        return true;
    };
    let mut allowed = false;
    for rule in rules {
        if rule_matches(rule, ctx) {
            allowed = rule.action == RuleAction::Allow;
        }
    }
    allowed
}

fn rule_matches(rule: &Rule, ctx: &RuleContext) -> bool {
    if let Some(os) = &rule.os
        && !os_matches(os, ctx.platform)
    {
        return false;
    }
    if let Some(features) = &rule.features {
        return features
            .iter()
            .all(|(name, expected)| feature_value(name, ctx.features) == *expected);
    }
    true
}

fn os_matches(rule: &OsRule, platform: Platform) -> bool {
    if let Some(name) = &rule.name
        && name != os_key(platform.os)
    {
        return false;
    }
    if let Some(arch) = &rule.arch {
        let matches = match arch.as_str() {
            "x86" => platform.arch == Arch::X86,
            "x86_64" => platform.arch == Arch::X86_64,
            "arm64" | "aarch64" => platform.arch == Arch::Aarch64,
            _ => false,
        };
        if !matches {
            return false;
        }
    }
    if let Some(version) = &rule.version {
        return platform.os == Os::Windows && version.starts_with("^10\\.");
    }
    true
}

fn feature_value(name: &str, features: Features) -> bool {
    match name {
        "has_custom_resolution" => features.has_custom_resolution,
        "is_quick_play_singleplayer" => features.is_quick_play_singleplayer,
        "is_quick_play_multiplayer" => features.is_quick_play_multiplayer,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(os: Os, arch: Arch) -> RuleContext {
        RuleContext::new(Platform { os, arch })
    }

    fn parse(json: &str) -> Vec<Rule> {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn missing_rules_allow() {
        assert!(rules_allow(None, &ctx(Os::Linux, Arch::X86_64)));
        assert!(rules_allow(Some(&[]), &ctx(Os::Linux, Arch::X86_64)));
    }

    #[test]
    fn allow_all_except_osx() {
        let rules = parse(r#"[{"action":"allow"},{"action":"disallow","os":{"name":"osx"}}]"#);
        assert!(rules_allow(Some(&rules), &ctx(Os::Windows, Arch::X86_64)));
        assert!(!rules_allow(Some(&rules), &ctx(Os::MacOs, Arch::Aarch64)));
    }

    #[test]
    fn only_osx() {
        let rules = parse(r#"[{"action":"allow","os":{"name":"osx"}}]"#);
        assert!(rules_allow(Some(&rules), &ctx(Os::MacOs, Arch::X86_64)));
        assert!(!rules_allow(Some(&rules), &ctx(Os::Linux, Arch::X86_64)));
    }

    #[test]
    fn x86_arch_rule_only_matches_32_bit() {
        let rules = parse(r#"[{"action":"allow","os":{"arch":"x86"}}]"#);
        assert!(rules_allow(Some(&rules), &ctx(Os::Windows, Arch::X86)));
        assert!(!rules_allow(Some(&rules), &ctx(Os::Windows, Arch::X86_64)));
    }

    #[test]
    fn feature_rules_follow_context() {
        let rules = parse(r#"[{"action":"allow","features":{"has_custom_resolution":true}}]"#);
        let mut context = ctx(Os::Linux, Arch::X86_64);
        assert!(!rules_allow(Some(&rules), &context));
        context.features.has_custom_resolution = true;
        assert!(rules_allow(Some(&rules), &context));

        let demo = parse(r#"[{"action":"allow","features":{"is_demo_user":true}}]"#);
        assert!(!rules_allow(Some(&demo), &context));
    }

    #[test]
    fn os_version_rules_only_match_windows_10_pattern() {
        let windows = parse(r#"[{"action":"allow","os":{"name":"windows","version":"^10\\."}}]"#);
        assert!(rules_allow(Some(&windows), &ctx(Os::Windows, Arch::X86_64)));
        let old_mac = parse(
            r#"[{"action":"allow"},{"action":"disallow","os":{"name":"osx","version":"^10\\.5\\.\\d$"}}]"#,
        );
        assert!(rules_allow(Some(&old_mac), &ctx(Os::MacOs, Arch::Aarch64)));
    }
}
