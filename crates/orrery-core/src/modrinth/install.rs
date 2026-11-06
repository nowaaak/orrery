use std::collections::{HashMap, VecDeque};
use std::io::Read;
use std::path::{Path, PathBuf};

use super::client::ModrinthClient;
use super::models::{DependencyType, Version};
use crate::instances::mods::{ContentKind, InstalledMod, ModsDir, validate_content_file_name};
use crate::net::{Checksum, Download, Http, VerifyMode, download_all};
use crate::progress::{Progress, Stage};
use crate::{CoreError, CoreResult};

const MAX_RESOLVED_PROJECTS: usize = 64;
const CDN_PREFIX: &str = "https://cdn.modrinth.com/";
const FABRIC_API_SLUG: &str = "fabric-api";
const MAX_MOD_MANIFEST_BYTES: u64 = 1024 * 1024;

pub struct ModTarget<'a> {
    pub game_version: &'a str,
    pub loaders: Vec<&'a str>,
    pub kind: ContentKind,
}

#[derive(Debug, Clone)]
pub struct PlannedMod {
    pub version: Version,
    pub dependency: bool,
}

pub fn pick_version(versions: Vec<Version>) -> Option<Version> {
    let release = versions.iter().position(|v| v.version_type == "release");
    match release {
        Some(index) => versions.into_iter().nth(index),
        None => versions.into_iter().next(),
    }
}

pub async fn resolve(
    client: &ModrinthClient,
    target: &ModTarget<'_>,
    installed: &[InstalledMod],
    project_id: &str,
    version_id: Option<&str>,
) -> CoreResult<Vec<PlannedMod>> {
    resolve_from(client, target, installed, project_id, version_id, false).await
}

async fn resolve_from(
    client: &ModrinthClient,
    target: &ModTarget<'_>,
    installed: &[InstalledMod],
    project_id: &str,
    version_id: Option<&str>,
    as_dependency: bool,
) -> CoreResult<Vec<PlannedMod>> {
    let mut queue: VecDeque<(Option<String>, Option<String>, bool)> = VecDeque::from([(
        Some(project_id.to_string()),
        version_id.map(str::to_string),
        as_dependency,
    )]);
    let mut planned: Vec<PlannedMod> = Vec::new();

    while let Some((project, version, dependency)) = queue.pop_front() {
        if planned.len() >= MAX_RESOLVED_PROJECTS {
            return Err(CoreError::InvalidInput(
                "too many dependencies to resolve".into(),
            ));
        }
        let version = match (version, project.as_deref()) {
            (Some(version_id), _) => client.version(&version_id).await?,
            (None, Some(project_id)) => {
                if dependency && installed.iter().any(|m| m.project_id == project_id) {
                    continue;
                }
                let versions = client
                    .project_versions(project_id, target.game_version, &target.loaders)
                    .await?;
                pick_version(versions).ok_or_else(|| {
                    CoreError::NotFound(format!(
                        "a version of {project_id} for Minecraft {} with {}",
                        target.game_version,
                        target.loaders.join("/")
                    ))
                })?
            }
            (None, None) => continue,
        };
        if planned
            .iter()
            .any(|p| p.version.project_id == version.project_id)
            || (dependency && installed.iter().any(|m| m.project_id == version.project_id))
        {
            continue;
        }

        for dep in &version.dependencies {
            match dep.dependency_type {
                DependencyType::Required if target.kind == ContentKind::Mod => {
                    queue.push_back((dep.project_id.clone(), dep.version_id.clone(), true));
                }
                DependencyType::Incompatible => {
                    if let Some(conflict) = dep.project_id.as_deref()
                        && let Some(existing) = installed.iter().find(|m| m.project_id == conflict)
                    {
                        return Err(CoreError::InvalidInput(format!(
                            "{} is incompatible with the installed mod {}",
                            version.name, existing.title
                        )));
                    }
                }
                DependencyType::Required | DependencyType::Optional | DependencyType::Embedded => {}
            }
        }
        planned.push(PlannedMod {
            version,
            dependency,
        });
    }
    Ok(planned)
}

pub async fn install(
    http: &Http,
    client: &ModrinthClient,
    mods: &ModsDir,
    target: &ModTarget<'_>,
    project_id: &str,
    version_id: Option<&str>,
    progress: &Progress,
) -> CoreResult<Vec<InstalledMod>> {
    progress.set_stage(Stage::FetchingMetadata);
    let index = mods.load().await?;
    let plan = resolve(client, target, &index.mods, project_id, version_id).await?;
    progress.check_cancelled()?;
    let mut entries = apply_plan(http, client, mods, target, &plan, progress).await?;

    if target.kind == ContentKind::Mod && any_requires_fabric_api(mods, &entries).await? {
        progress.set_stage(Stage::FetchingMetadata);
        let index = mods.load().await?;
        let plan = resolve_from(client, target, &index.mods, FABRIC_API_SLUG, None, true).await?;
        if !plan.is_empty() {
            entries.extend(apply_plan(http, client, mods, target, &plan, progress).await?);
        }
    }
    progress.set_stage(Stage::Finished);
    Ok(entries)
}

async fn apply_plan(
    http: &Http,
    client: &ModrinthClient,
    mods: &ModsDir,
    target: &ModTarget<'_>,
    plan: &[PlannedMod],
    progress: &Progress,
) -> CoreResult<Vec<InstalledMod>> {
    let index = mods.load().await?;
    let mut downloads = Vec::new();
    let mut entries = Vec::new();
    let mut titles: HashMap<String, (String, Option<String>)> = HashMap::new();
    for item in plan {
        let version = &item.version;
        let file = version.primary_file().ok_or_else(|| {
            CoreError::NotFound(format!("downloadable file for {}", version.name))
        })?;
        validate_content_file_name(&file.filename, target.kind)?;
        if !file.url.starts_with(CDN_PREFIX) {
            return Err(CoreError::InvalidInput(format!(
                "unexpected download host for {}",
                file.filename
            )));
        }
        let sha512 = file
            .hashes
            .get("sha512")
            .cloned()
            .ok_or_else(|| CoreError::NotFound(format!("sha512 for {}", file.filename)))?;
        if !titles.contains_key(&version.project_id) {
            let project = client.project(&version.project_id).await?;
            titles.insert(
                version.project_id.clone(),
                (project.title, project.icon_url),
            );
        }
        let (title, icon_url) = titles[&version.project_id].clone();
        downloads.push(Download {
            url: file.url.clone(),
            path: mods.dir(target.kind).join(&file.filename),
            checksum: Some(Checksum::Sha512(sha512.clone())),
            size: Some(file.size),
        });
        let previous = index
            .mods
            .iter()
            .find(|m| m.project_id == version.project_id);
        entries.push(InstalledMod {
            project_id: version.project_id.clone(),
            version_id: version.id.clone(),
            title,
            version_number: version.version_number.clone(),
            file_name: file.filename.clone(),
            sha512,
            enabled: true,
            dependency: item.dependency && previous.is_none_or(|p| p.dependency),
            icon_url,
            kind: target.kind,
            game_versions: version.game_versions.clone(),
            loaders: version.loaders.clone(),
            pack: None,
            pinned: previous.is_some_and(|p| p.pinned),
        });
    }

    progress.set_stage(Stage::DownloadingMods);
    download_all(http, downloads, 4, VerifyMode::Full, progress).await?;

    mods.commit(index, &entries).await?;
    Ok(entries)
}

async fn any_requires_fabric_api(mods: &ModsDir, entries: &[InstalledMod]) -> CoreResult<bool> {
    let paths: Vec<PathBuf> = entries.iter().map(|entry| mods.file_path(entry)).collect();
    tokio::task::spawn_blocking(move || {
        paths.iter().any(|path| match read_mod_manifest(path) {
            Ok(Some(manifest)) => requires_fabric_api(&manifest),
            Ok(None) => false,
            Err(err) => {
                tracing::warn!(path = %path.display(), error = %err, "could not read mod manifest");
                false
            }
        })
    })
    .await
    .map_err(|err| CoreError::Unsupported(format!("mod manifest check panicked: {err}")))
}

fn read_mod_manifest(path: &Path) -> Result<Option<serde_json::Value>, String> {
    let file = std::fs::File::open(path).map_err(|err| err.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|err| err.to_string())?;
    for name in ["fabric.mod.json", "quilt.mod.json"] {
        let Ok(entry) = archive.by_name(name) else {
            continue;
        };
        let mut text = String::new();
        entry
            .take(MAX_MOD_MANIFEST_BYTES)
            .read_to_string(&mut text)
            .map_err(|err| err.to_string())?;
        return serde_json::from_str(&text)
            .map(Some)
            .map_err(|err| format!("{name}: {err}"));
    }
    Ok(None)
}

fn requires_fabric_api(manifest: &serde_json::Value) -> bool {
    let fabric = manifest
        .get("depends")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flat_map(|depends| depends.keys().map(String::as_str));
    let quilt = manifest
        .pointer("/quilt_loader/depends")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|dep| dep.as_str().or_else(|| dep.get("id")?.as_str()));
    fabric.chain(quilt).any(is_fabric_api_module)
}

fn is_fabric_api_module(id: &str) -> bool {
    if matches!(
        id,
        "fabric" | "fabric-api" | "fabric-api-base" | "quilted_fabric_api"
    ) {
        return true;
    }
    id.strip_prefix("fabric-")
        .and_then(|rest| rest.rsplit_once("-v"))
        .is_some_and(|(_, major)| !major.is_empty() && major.chars().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn version_json(
        id: &str,
        project: &str,
        kind: &str,
        deps: serde_json::Value,
    ) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "project_id": project,
            "name": format!("{project} {id}"),
            "version_number": "1.0",
            "version_type": kind,
            "date_published": "2024-01-01T00:00:00Z",
            "files": [{ "hashes": { "sha512": "abc" }, "url": "https://cdn.modrinth.com/x.jar", "filename": format!("{project}.jar"), "primary": true, "size": 3 }],
            "dependencies": deps
        })
    }

    #[test]
    fn detects_fabric_api_requirements() {
        let fabric = serde_json::json!({
            "depends": { "fabricloader": ">=0.15", "fabric-lifecycle-events-v1": "*" }
        });
        assert!(requires_fabric_api(&fabric));
        let quilt = serde_json::json!({
            "quilt_loader": { "depends": ["quilt_loader", { "id": "fabric-api", "versions": "*" }] }
        });
        assert!(requires_fabric_api(&quilt));
        let standalone = serde_json::json!({
            "depends": { "fabricloader": "*", "minecraft": "1.20.1", "fabric-language-kotlin": "*" }
        });
        assert!(!requires_fabric_api(&standalone));
        assert!(!requires_fabric_api(&serde_json::json!({})));
        assert!(!is_fabric_api_module("fabric-v"));
        assert!(is_fabric_api_module("fabric-rendering-v1"));
    }

    #[test]
    fn prefers_release_versions() {
        let versions: Vec<Version> = serde_json::from_value(serde_json::json!([
            version_json("b", "p", "beta", serde_json::json!([])),
            version_json("r", "p", "release", serde_json::json!([])),
        ]))
        .unwrap();
        assert_eq!(pick_version(versions).unwrap().id, "r");
    }

    #[tokio::test]
    async fn resolves_required_dependencies_and_skips_installed_ones() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/project/sodiumextra/version"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                version_json(
                    "v1",
                    "sodiumextra",
                    "release",
                    serde_json::json!([
                        { "project_id": "sodium", "dependency_type": "required" },
                        { "project_id": "fabricapi", "dependency_type": "required" },
                        { "project_id": "optional", "dependency_type": "optional" }
                    ])
                )
            ])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/project/sodium/version"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                version_json("v2", "sodium", "release", serde_json::json!([]))
            ])))
            .mount(&server)
            .await;

        let client = ModrinthClient::with_base(Http::new("test").unwrap(), &server.uri());
        let installed = vec![InstalledMod {
            project_id: "fabricapi".into(),
            version_id: "old".into(),
            title: "Fabric API".into(),
            version_number: "1".into(),
            file_name: "fabric-api.jar".into(),
            sha512: "x".into(),
            enabled: true,
            dependency: true,
            icon_url: None,
            kind: ContentKind::Mod,
            game_versions: Vec::new(),
            loaders: Vec::new(),
            pack: None,
            pinned: false,
        }];
        let target = ModTarget {
            game_version: "1.20.1",
            loaders: vec!["fabric"],
            kind: ContentKind::Mod,
        };
        let plan = resolve(&client, &target, &installed, "sodiumextra", None)
            .await
            .unwrap();
        let ids: Vec<_> = plan
            .iter()
            .map(|p| (p.version.project_id.as_str(), p.dependency))
            .collect();
        assert_eq!(ids, [("sodiumextra", false), ("sodium", true)]);
    }

    #[tokio::test]
    async fn incompatible_installed_mod_is_reported() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/project/optifabric/version"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                version_json(
                    "v1",
                    "optifabric",
                    "release",
                    serde_json::json!([
                        { "project_id": "sodium", "dependency_type": "incompatible" }
                    ])
                )
            ])))
            .mount(&server)
            .await;
        let client = ModrinthClient::with_base(Http::new("test").unwrap(), &server.uri());
        let installed = vec![InstalledMod {
            project_id: "sodium".into(),
            version_id: "v".into(),
            title: "Sodium".into(),
            version_number: "1".into(),
            file_name: "sodium.jar".into(),
            sha512: "x".into(),
            enabled: true,
            dependency: false,
            icon_url: None,
            kind: ContentKind::Mod,
            game_versions: Vec::new(),
            loaders: Vec::new(),
            pack: None,
            pinned: false,
        }];
        let target = ModTarget {
            game_version: "1.20.1",
            loaders: vec!["fabric"],
            kind: ContentKind::Mod,
        };
        let err = resolve(&client, &target, &installed, "optifabric", None)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("incompatible"));
    }
}
