use serde::Serialize;
use sysinfo::{MemoryRefreshKind, RefreshKind, System};
use ts_rs::TS;

const MB: u64 = 1024 * 1024;
const SYSTEM_RESERVE_MB: u32 = 3072;
const MIN_HEAP_MB: u32 = 1024;
const MAX_USEFUL_HEAP_MB: u32 = 10240;
const STEP_MB: u32 = 512;

pub const RECOMMENDED_JVM_ARGS: [&str; 4] = [
    "-XX:+UseG1GC",
    "-XX:+ParallelRefProcEnabled",
    "-XX:MaxGCPauseMillis=200",
    "-XX:+DisableExplicitGC",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RecommendationLimit {
    None,
    SystemMemory,
    UsefulMaximum,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SettingsRecommendation {
    pub system_memory_mb: u32,
    pub needed_mb: u32,
    pub max_memory_mb: u32,
    pub min_memory_mb: u32,
    pub jvm_args: Vec<String>,
    pub mod_count: u32,
    pub shaders: bool,
    pub limit: RecommendationLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionTier {
    Legacy,
    Modern,
    Current,
}

pub struct Workload<'a> {
    pub minecraft_version: &'a str,
    pub modded: bool,
    pub mod_count: u32,
    pub shaders: bool,
}

pub fn system_memory_mb() -> Option<u32> {
    let system = System::new_with_specifics(
        RefreshKind::nothing().with_memory(MemoryRefreshKind::nothing().with_ram()),
    )
    .ok()?;
    let total = system.total_memory() / MB;
    (total > 0).then(|| u32::try_from(total).unwrap_or(u32::MAX))
}

pub(crate) fn release_minor(version: &str) -> Option<u32> {
    let mut parts = version.split(['.', '-', ' ']);
    let first = parts.next().and_then(|p| p.parse::<u32>().ok());
    let second = parts.next().and_then(|p| p.parse::<u32>().ok());
    match (first, second) {
        (Some(1), Some(minor)) => Some(minor),
        _ => None,
    }
}

pub fn version_tier(version: &str) -> VersionTier {
    match release_minor(version) {
        Some(minor) if minor < 13 => VersionTier::Legacy,
        Some(minor) if minor < 18 => VersionTier::Modern,
        _ => VersionTier::Current,
    }
}

fn round_down(value: u32) -> u32 {
    value / STEP_MB * STEP_MB
}

fn needed_memory(workload: &Workload<'_>) -> u32 {
    let tier = version_tier(workload.minecraft_version);
    let base = match (workload.modded, tier) {
        (false, VersionTier::Legacy) => 2048,
        (false, VersionTier::Modern) => 3072,
        (false, VersionTier::Current) => 4096,
        (true, VersionTier::Legacy) => 3072,
        (true, _) => 4096,
    };
    let per_mods = if workload.modded {
        workload.mod_count.saturating_sub(20).div_ceil(40) * 1024
    } else {
        0
    };
    let shaders = if workload.shaders { 1024 } else { 0 };
    base + per_mods + shaders
}

pub fn recommend(system_memory_mb: u32, workload: &Workload<'_>) -> SettingsRecommendation {
    let needed = needed_memory(workload);
    let system_cap =
        round_down((system_memory_mb / 2).min(system_memory_mb.saturating_sub(SYSTEM_RESERVE_MB)))
            .max(MIN_HEAP_MB);

    let (max_memory, limit) = if needed > MAX_USEFUL_HEAP_MB && system_cap >= MAX_USEFUL_HEAP_MB {
        (MAX_USEFUL_HEAP_MB, RecommendationLimit::UsefulMaximum)
    } else if needed > system_cap {
        (system_cap, RecommendationLimit::SystemMemory)
    } else {
        (needed, RecommendationLimit::None)
    };

    SettingsRecommendation {
        system_memory_mb,
        needed_mb: needed,
        max_memory_mb: max_memory,
        min_memory_mb: max_memory,
        jvm_args: RECOMMENDED_JVM_ARGS.iter().map(|a| a.to_string()).collect(),
        mod_count: workload.mod_count,
        shaders: workload.shaders,
        limit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workload(version: &str, modded: bool, mods: u32, shaders: bool) -> Workload<'_> {
        Workload {
            minecraft_version: version,
            modded,
            mod_count: mods,
            shaders,
        }
    }

    #[test]
    fn version_tiers_cover_old_and_new_numbering() {
        assert_eq!(version_tier("1.8.9"), VersionTier::Legacy);
        assert_eq!(version_tier("1.12.2"), VersionTier::Legacy);
        assert_eq!(version_tier("1.16.5"), VersionTier::Modern);
        assert_eq!(version_tier("1.20.1"), VersionTier::Current);
        assert_eq!(version_tier("26.3"), VersionTier::Current);
        assert_eq!(version_tier("24w14a"), VersionTier::Current);
    }

    #[test]
    fn vanilla_on_large_system_gets_what_it_needs() {
        let r = recommend(32768, &workload("1.20.1", false, 0, false));
        assert_eq!(r.max_memory_mb, 4096);
        assert_eq!(r.min_memory_mb, 4096);
        assert_eq!(r.limit, RecommendationLimit::None);
        assert_eq!(
            recommend(16384, &workload("1.8.9", false, 0, false)).max_memory_mb,
            2048
        );
    }

    #[test]
    fn mods_and_shaders_increase_the_heap() {
        assert_eq!(
            recommend(32768, &workload("1.20.1", true, 15, false)).max_memory_mb,
            4096
        );
        assert_eq!(
            recommend(32768, &workload("1.20.1", true, 60, false)).max_memory_mb,
            5120
        );
        assert_eq!(
            recommend(32768, &workload("1.20.1", true, 60, true)).max_memory_mb,
            6144
        );
        assert_eq!(
            recommend(32768, &workload("1.20.1", true, 200, true)).max_memory_mb,
            10240
        );
    }

    #[test]
    fn small_systems_keep_memory_for_the_os() {
        let r = recommend(8192, &workload("1.20.1", true, 120, true));
        assert_eq!(r.max_memory_mb, 4096);
        assert_eq!(r.limit, RecommendationLimit::SystemMemory);

        let tiny = recommend(4096, &workload("1.20.1", false, 0, false));
        assert_eq!(tiny.max_memory_mb, 1024);
        assert_eq!(tiny.limit, RecommendationLimit::SystemMemory);
    }

    #[test]
    fn huge_modpacks_are_capped_at_a_useful_maximum() {
        let r = recommend(65536, &workload("1.20.1", true, 400, true));
        assert_eq!(r.max_memory_mb, 10240);
        assert_eq!(r.limit, RecommendationLimit::UsefulMaximum);
    }

    #[test]
    fn system_memory_is_detected() {
        assert!(system_memory_mb().is_some_and(|mb| mb >= 512));
    }
}
