use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Serialize;
use ts_rs::TS;

const MAX_REPORT_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CrashHint {
    OutOfMemory,
    HeapTooLarge,
    ModConflict,
    JavaVersion,
    Graphics,
    NativeCrash,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CrashReport {
    pub hint: CrashHint,
    pub summary: Option<String>,
    pub report_file: Option<String>,
}

fn newest_since(dir: &Path, since: SystemTime, matches: impl Fn(&str) -> bool) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_str().is_some_and(&matches))
        .filter_map(|entry| {
            let modified = entry.metadata().ok()?.modified().ok()?;
            (modified >= since).then(|| (modified, entry.path()))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

fn read_limited(path: &Path) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    if metadata.len() > MAX_REPORT_BYTES {
        return None;
    }
    std::fs::read(path)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

fn classify(text: &str) -> CrashHint {
    let lower = text.to_lowercase();
    let has = |needle: &str| lower.contains(needle);
    if has("could not reserve enough space")
        || has("invalid maximum heap size")
        || has("invalid initial heap size")
    {
        CrashHint::HeapTooLarge
    } else if has("outofmemoryerror") || has("out of memory") {
        CrashHint::OutOfMemory
    } else if has("unsupportedclassversionerror")
        || has("has been compiled by a more recent version of the java runtime")
    {
        CrashHint::JavaVersion
    } else if has("incompatible mods found")
        || has("mod resolution encountered")
        || has("incompatible mod set")
        || has("mixin apply failed")
        || has("mixintransformererror")
        || has("could not execute entrypoint")
    {
        CrashHint::ModConflict
    } else if has("glfw error")
        || has("pixel format not accelerated")
        || has("opengl")
        || has("exception_access_violation")
            && (has("atio6axx") || has("nvoglv") || has("ig9icd") || has("ig75icd"))
    {
        CrashHint::Graphics
    } else {
        CrashHint::Unknown
    }
}

fn summarize(report: &str) -> Option<String> {
    let description = report
        .lines()
        .find_map(|line| line.trim().strip_prefix("Description:").map(str::trim));
    let exception = report.lines().map(str::trim).find(|line| {
        (line.starts_with("java.")
            || line.starts_with("net.")
            || line.contains("Exception")
            || line.contains("Error:"))
            && !line.starts_with("at ")
    });
    match (description, exception) {
        (Some(d), Some(e)) => Some(format!("{d}: {e}")),
        (Some(d), None) => Some(d.to_string()),
        (None, Some(e)) => Some(e.to_string()),
        (None, None) => None,
    }
}

pub fn analyze(game_dir: &Path, started: SystemTime, log_tail: &[String]) -> CrashReport {
    let log_text = log_tail.join("\n");
    let crash_file = newest_since(&game_dir.join("crash-reports"), started, |name| {
        name.starts_with("crash-") && name.ends_with(".txt")
    });
    if let Some(path) = crash_file
        && let Some(report) = read_limited(&path)
    {
        let combined = format!("{report}\n{log_text}");
        return CrashReport {
            hint: classify(&combined),
            summary: summarize(&report),
            report_file: Some(path.to_string_lossy().into_owned()),
        };
    }

    let native = newest_since(game_dir, started, |name| {
        name.starts_with("hs_err_pid") && name.ends_with(".log")
    });
    if let Some(path) = native {
        let text = read_limited(&path).unwrap_or_default();
        let hint = match classify(&format!("{text}\n{log_text}")) {
            CrashHint::Unknown => CrashHint::NativeCrash,
            other => other,
        };
        return CrashReport {
            hint,
            summary: None,
            report_file: Some(path.to_string_lossy().into_owned()),
        };
    }

    let summary = log_tail
        .iter()
        .rev()
        .find(|line| {
            line.contains("Exception") || line.contains("Error") || line.contains("/FATAL]")
        })
        .map(|line| line.trim().chars().take(300).collect());
    CrashReport {
        hint: classify(&log_text),
        summary,
        report_file: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn crash_report_is_found_and_classified() {
        let dir = tempfile::tempdir().unwrap();
        let started = SystemTime::now() - Duration::from_secs(5);
        let reports = dir.path().join("crash-reports");
        std::fs::create_dir_all(&reports).unwrap();
        std::fs::write(
            reports.join("crash-2026-10-31_20.00.00-client.txt"),
            "---- Minecraft Crash Report ----\nDescription: Initializing game\n\njava.lang.OutOfMemoryError: Java heap space\n\tat x.y(Z.java:1)\n",
        )
        .unwrap();

        let report = analyze(dir.path(), started, &[]);
        assert_eq!(report.hint, CrashHint::OutOfMemory);
        assert_eq!(
            report.summary.as_deref(),
            Some("Initializing game: java.lang.OutOfMemoryError: Java heap space")
        );
        assert!(report.report_file.unwrap().ends_with("client.txt"));
    }

    #[test]
    fn old_reports_are_ignored_and_log_is_used() {
        let dir = tempfile::tempdir().unwrap();
        let reports = dir.path().join("crash-reports");
        std::fs::create_dir_all(&reports).unwrap();
        std::fs::write(reports.join("crash-old-client.txt"), "Description: old").unwrap();
        let started = SystemTime::now() + Duration::from_secs(60);
        let log = vec![
            "[main/INFO]: Loading Minecraft".to_string(),
            "[main/ERROR]: Incompatible mods found! net.fabricmc.loader.impl.FormattedException"
                .to_string(),
        ];

        let report = analyze(dir.path(), started, &log);
        assert_eq!(report.hint, CrashHint::ModConflict);
        assert!(report.report_file.is_none());
        assert!(
            report
                .summary
                .unwrap_or_default()
                .contains("Incompatible mods found")
        );
    }

    #[test]
    fn jvm_start_errors_are_recognized() {
        assert_eq!(
            classify(
                "Error occurred during initialization of VM\nCould not reserve enough space for object heap"
            ),
            CrashHint::HeapTooLarge
        );
        assert_eq!(
            classify("java.lang.UnsupportedClassVersionError: class file version 65.0"),
            CrashHint::JavaVersion
        );
        assert_eq!(classify("something else"), CrashHint::Unknown);
    }
}
