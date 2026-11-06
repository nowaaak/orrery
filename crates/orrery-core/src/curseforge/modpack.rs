use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::client::CurseForgeClient;
use super::fingerprint::file_fingerprint;
use super::install::{
    CfTarget, Clients, FileSource, content_hash, find_sources, minecraft_versions, sha512_of,
};
use super::models::{CfFile, CfMod, project_key};
use crate::error::IoResultExt;
use crate::importers::{curseforge_loader, loader_spec};
use crate::instances::mods::{ContentKind, InstalledMod, ModsDir, validate_content_file_name};
use crate::instances::{Instance, LoaderSpec, NewInstance};
use crate::modpacks::{
    ModpackExport, archive_error, extract_prefixed, index_content, scan_content_async, write_pack,
};
use crate::net::{Checksum, Download, VerifyMode, download_all, sha1_hex};
use crate::progress::{Progress, Stage};
use crate::services::Services;
use crate::storage::{self, Versioned};
use crate::tasks::{TaskId, TaskKind, TaskSpec};
use crate::{CoreError, CoreResult};

pub const MANIFEST_FILE: &str = "manifest.json";
const MODRINTH_INDEX: &str = "modrinth.index.json";
const PENDING_FILE: &str = "pending-downloads.json";
const MAX_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PACK_FILES: usize = 2000;
const MAX_NAME_CHARS: usize = 64;
const DOWNLOAD_CONCURRENCY: usize = 8;
const CLASS_MOD: u32 = 6;
const CLASS_RESOURCE_PACK: u32 = 12;
const CLASS_SHADER: u32 = 6552;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub minecraft: ManifestMinecraft,
    pub manifest_type: String,
    pub manifest_version: u32,
    pub name: String,
    #[serde(default)]
    pub files: Vec<ManifestFile>,
    #[serde(default = "default_overrides")]
    pub overrides: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestMinecraft {
    pub version: String,
    #[serde(default)]
    pub mod_loaders: Vec<ManifestLoader>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestLoader {
    pub id: String,
    #[serde(default)]
    pub primary: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestFile {
    #[serde(rename = "projectID")]
    pub project_id: u32,
    #[serde(rename = "fileID")]
    pub file_id: u32,
    #[serde(default = "required_by_default")]
    pub required: bool,
}

fn default_overrides() -> String {
    "overrides".into()
}

fn required_by_default() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PendingDownload {
    pub file_id: u32,
    pub mod_id: u32,
    pub title: String,
    pub file_name: String,
    pub kind: ContentKind,
    pub sha1: Option<String>,
    pub size: u64,
    pub page_url: String,
    pub icon_url: Option<String>,
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PendingFile {
    schema: u32,
    #[serde(default)]
    files: Vec<PendingDownload>,
}

impl Versioned for PendingFile {
    const SCHEMA: u32 = 1;
    fn schema(&self) -> u32 {
        self.schema
    }
}

#[derive(Debug, Clone)]
pub struct PackPlan {
    pub name: String,
    pub minecraft_version: String,
    pub loader: LoaderSpec,
    pub overrides: String,
    pub files: Vec<ManifestFile>,
}

pub fn is_curseforge_pack(archive: &Path) -> bool {
    let Ok(file) = File::open(archive) else {
        return false;
    };
    let Ok(mut zip) = zip::ZipArchive::new(file) else {
        return false;
    };
    let has_manifest = zip.by_name(MANIFEST_FILE).is_ok();
    has_manifest && zip.by_name(MODRINTH_INDEX).is_err()
}

pub fn read_manifest(archive: &Path) -> CoreResult<Manifest> {
    let file = File::open(archive).at_path(archive)?;
    let mut zip = zip::ZipArchive::new(file).map_err(archive_error(archive))?;
    let entry = zip.by_name(MANIFEST_FILE).map_err(|_| {
        CoreError::InvalidInput("not a CurseForge modpack: manifest.json is missing".into())
    })?;
    let mut text = String::new();
    entry
        .take(MAX_MANIFEST_BYTES)
        .read_to_string(&mut text)
        .at_path(archive)?;
    serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|err| CoreError::InvalidInput(format!("invalid manifest.json: {err}")))
}

fn clean_name(name: &str) -> String {
    let name: String = name
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_NAME_CHARS)
        .collect();
    let name = name.trim();
    if name.is_empty() {
        "Modpack".into()
    } else {
        name.to_string()
    }
}

fn valid_overrides(folder: &str) -> bool {
    !folder.is_empty()
        && folder.len() <= 64
        && folder != "."
        && folder != ".."
        && folder
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

pub fn plan(manifest: Manifest) -> CoreResult<PackPlan> {
    if manifest.manifest_type != "minecraftModpack" || manifest.manifest_version != 1 {
        return Err(CoreError::Unsupported(format!(
            "CurseForge manifest {} version {}",
            manifest.manifest_type, manifest.manifest_version
        )));
    }
    if !valid_overrides(&manifest.overrides) {
        return Err(CoreError::InvalidInput(format!(
            "invalid overrides folder {:?}",
            manifest.overrides
        )));
    }
    let minecraft_version = manifest.minecraft.version.trim().to_string();
    if minecraft_version.is_empty() {
        return Err(CoreError::InvalidInput(
            "modpack does not name a Minecraft version".into(),
        ));
    }
    let loader = match manifest
        .minecraft
        .mod_loaders
        .iter()
        .max_by_key(|loader| loader.primary)
    {
        None => LoaderSpec::Vanilla,
        Some(entry) => {
            let foreign = curseforge_loader(&entry.id, Some(&minecraft_version));
            loader_spec(&foreign, Some(&minecraft_version)).ok_or_else(|| {
                CoreError::Unsupported(format!(
                    "the mod loader {} for Minecraft {minecraft_version}",
                    entry.id
                ))
            })?
        }
    };
    let files: Vec<ManifestFile> = manifest.files.into_iter().filter(|f| f.required).collect();
    if files.len() > MAX_PACK_FILES {
        return Err(CoreError::InvalidInput(
            "modpack lists too many files".into(),
        ));
    }
    Ok(PackPlan {
        name: clean_name(&manifest.name),
        minecraft_version,
        loader,
        overrides: manifest.overrides,
        files,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportManifest {
    minecraft: ExportMinecraft,
    manifest_type: &'static str,
    manifest_version: u32,
    name: String,
    version: &'static str,
    author: &'static str,
    files: Vec<ExportFile>,
    overrides: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportMinecraft {
    version: String,
    mod_loaders: Vec<ExportLoader>,
}

#[derive(Debug, Clone, Serialize)]
struct ExportLoader {
    id: String,
    primary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct ExportFile {
    #[serde(rename = "projectID")]
    project_id: u32,
    #[serde(rename = "fileID")]
    file_id: u32,
    required: bool,
}

fn export_loader(loader: &LoaderSpec) -> CoreResult<Option<String>> {
    Ok(match loader {
        LoaderSpec::Vanilla => None,
        LoaderSpec::Fabric { loader_version } => Some(format!("fabric-{loader_version}")),
        LoaderSpec::Quilt { loader_version } => Some(format!("quilt-{loader_version}")),
        LoaderSpec::Forge { loader_version } => Some(format!("forge-{loader_version}")),
        LoaderSpec::NeoForge { loader_version } => Some(format!("neoforge-{loader_version}")),
        LoaderSpec::LegacyFabric { .. } => {
            return Err(CoreError::Unsupported(
                "CurseForge modpacks with Legacy Fabric".into(),
            ));
        }
    })
}

fn export_manifest(instance: &Instance, files: Vec<ExportFile>) -> CoreResult<ExportManifest> {
    let mod_loaders = export_loader(&instance.loader)?
        .map(|id| ExportLoader { id, primary: true })
        .into_iter()
        .collect();
    Ok(ExportManifest {
        minecraft: ExportMinecraft {
            version: instance.minecraft_version.clone(),
            mod_loaders,
        },
        manifest_type: "minecraftModpack",
        manifest_version: 1,
        name: instance.name.clone(),
        version: "1.0.0",
        author: "",
        files,
        overrides: "overrides",
    })
}

pub async fn export(
    client: &CurseForgeClient,
    instance: &Instance,
    game_dir: &Path,
    target: &Path,
) -> CoreResult<ModpackExport> {
    if !target.is_absolute() || target.extension().and_then(|e| e.to_str()) != Some("zip") {
        return Err(CoreError::InvalidInput(
            "export target must be an absolute .zip path".into(),
        ));
    }
    export_loader(&instance.loader)?;
    let local = scan_content_async(game_dir).await?;
    let mut fingerprints = Vec::with_capacity(local.len());
    for content in &local {
        let path = game_dir
            .join(content.kind.folder())
            .join(&content.file_name);
        fingerprints.push(file_fingerprint(&path).await?);
    }
    let matches = client.files_for_fingerprints(&fingerprints).await?;
    let mut files = Vec::new();
    let mut skipped = Vec::new();
    for (content, fingerprint) in local.iter().zip(&fingerprints) {
        match matches.get(fingerprint) {
            Some(file) => files.push(ExportFile {
                project_id: file.mod_id,
                file_id: file.id,
                required: true,
            }),
            None => skipped.push(content.file_name.clone()),
        }
    }
    files.sort();
    files.dedup();
    skipped.sort();
    let linked = u32::try_from(files.len()).unwrap_or(u32::MAX);
    let manifest = serde_json::to_vec_pretty(&export_manifest(instance, files)?)
        .map_err(|err| CoreError::InvalidInput(format!("modpack manifest: {err}")))?;
    let game_dir = game_dir.to_path_buf();
    let target = target.to_path_buf();
    let override_files = tokio::task::spawn_blocking(move || {
        write_pack(&target, MANIFEST_FILE, &manifest, &game_dir)
    })
    .await
    .map_err(|err| CoreError::Unsupported(format!("modpack export panicked: {err}")))??;
    Ok(ModpackExport {
        linked,
        override_files,
        skipped,
    })
}

fn kind_for(project: Option<&CfMod>, file: &CfFile) -> Option<ContentKind> {
    match project.and_then(|p| p.class_id) {
        Some(CLASS_MOD) => Some(ContentKind::Mod),
        Some(CLASS_RESOURCE_PACK) => Some(ContentKind::ResourcePack),
        Some(CLASS_SHADER) => Some(ContentKind::Shader),
        Some(_) => None,
        None => file.file_name.ends_with(".jar").then_some(ContentKind::Mod),
    }
}

fn page_url(project: Option<&CfMod>, file: &CfFile) -> String {
    match project.and_then(CfMod::website_url) {
        Some(url) => format!("{}/download/{}", url.trim_end_matches('/'), file.id),
        None => format!("https://www.curseforge.com/projects/{}", file.mod_id),
    }
}

#[derive(Debug, Clone)]
struct PackEntry {
    file: CfFile,
    project: Option<CfMod>,
    kind: ContentKind,
}

impl PackEntry {
    fn title(&self) -> String {
        self.project
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| self.file.display_name.clone())
    }

    fn installed(&self, target: &CfTarget<'_>, sha512: String) -> InstalledMod {
        InstalledMod {
            project_id: project_key(self.file.mod_id),
            version_id: self.file.id.to_string(),
            title: self.title(),
            version_number: self.file.display_name.clone(),
            file_name: self.file.file_name.clone(),
            sha512,
            enabled: true,
            dependency: false,
            icon_url: self.project.as_ref().and_then(CfMod::icon_url),
            kind: self.kind,
            game_versions: minecraft_versions(&self.file),
            loaders: target_for(target, self.kind).recorded_loaders(&self.file),
            pack: None,
            pinned: false,
        }
    }

    fn pending(&self, target: &CfTarget<'_>) -> PendingDownload {
        PendingDownload {
            file_id: self.file.id,
            mod_id: self.file.mod_id,
            title: self.title(),
            file_name: self.file.file_name.clone(),
            kind: self.kind,
            sha1: self.file.sha1(),
            size: self.file.file_length,
            page_url: page_url(self.project.as_ref(), &self.file),
            icon_url: self.project.as_ref().and_then(CfMod::icon_url),
            version_number: self.file.display_name.clone(),
            game_versions: minecraft_versions(&self.file),
            loaders: target_for(target, self.kind).recorded_loaders(&self.file),
        }
    }
}

fn target_for<'a>(target: &CfTarget<'a>, kind: ContentKind) -> CfTarget<'a> {
    CfTarget {
        game_version: target.game_version,
        loader: target.loader,
        kind,
    }
}

impl PendingDownload {
    fn installed(&self, sha512: String) -> InstalledMod {
        InstalledMod {
            project_id: project_key(self.mod_id),
            version_id: self.file_id.to_string(),
            title: self.title.clone(),
            version_number: self.version_number.clone(),
            file_name: self.file_name.clone(),
            sha512,
            enabled: true,
            dependency: false,
            icon_url: self.icon_url.clone(),
            kind: self.kind,
            game_versions: self.game_versions.clone(),
            loaders: self.loaders.clone(),
            pack: None,
            pinned: false,
        }
    }

    fn matches(&self, path: &Path, size: u64) -> bool {
        let same_name = path.file_name().and_then(|n| n.to_str()) == Some(self.file_name.as_str());
        let size_known = self.size > 0;
        if (size_known && size != self.size) || (!size_known && !same_name) {
            return false;
        }
        match &self.sha1 {
            Some(expected) => std::fs::read(path).is_ok_and(|data| sha1_hex(&data) == *expected),
            None => same_name,
        }
    }
}

fn find_in_folder(folder: &Path, pending: &[PendingDownload]) -> HashMap<u32, PathBuf> {
    let mut found = HashMap::new();
    let Ok(entries) = std::fs::read_dir(folder) else {
        return found;
    };
    for entry in entries.flatten() {
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let path = entry.path();
        if let Some(item) = pending
            .iter()
            .filter(|item| !found.contains_key(&item.file_id))
            .find(|item| item.matches(&path, metadata.len()))
        {
            found.insert(item.file_id, path);
        }
    }
    found
}

fn pending_path(instance_dir: &Path) -> PathBuf {
    instance_dir.join(PENDING_FILE)
}

async fn load_pending(instance_dir: &Path) -> CoreResult<Vec<PendingDownload>> {
    Ok(
        storage::read_versioned::<PendingFile>(&pending_path(instance_dir))
            .await?
            .map(|file| file.files)
            .unwrap_or_default(),
    )
}

async fn save_pending(instance_dir: &Path, files: Vec<PendingDownload>) -> CoreResult<()> {
    let path = pending_path(instance_dir);
    if files.is_empty() {
        return match tokio::fs::remove_file(&path).await {
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err).at_path(&path),
            _ => Ok(()),
        };
    }
    storage::write_json_atomic(
        &path,
        &PendingFile {
            schema: PendingFile::SCHEMA,
            files,
        },
    )
    .await
}

impl Services {
    pub async fn search_curseforge_modpacks(
        &self,
        query: &str,
        offset: u32,
    ) -> CoreResult<crate::modrinth::models::ModSearchPage> {
        self.curseforge
            .search(super::MODPACK_CLASS, query, None, None, offset)
            .await
    }

    pub async fn install_curseforge_modpack(&self, mod_id: u32, title: &str) -> CoreResult<TaskId> {
        let files = self.curseforge.project_files(mod_id, None).await?;
        let file = files
            .into_iter()
            .filter(|f| f.is_installable() && f.file_name.ends_with(".zip"))
            .max_by(|a, b| {
                (a.is_release(), &a.file_date, a.id).cmp(&(b.is_release(), &b.file_date, b.id))
            })
            .ok_or_else(|| CoreError::NotFound(format!("a downloadable version of {title}")))?;
        let url = file
            .download_url
            .clone()
            .filter(|url| super::is_allowed_download(url))
            .ok_or_else(|| {
                CoreError::Unsupported(format!(
                    "{title} can only be downloaded on the CurseForge website"
                ))
            })?;
        let sha1 = file
            .sha1()
            .ok_or_else(|| CoreError::NotFound(format!("a checksum for {}", file.file_name)))?;
        let services = self.clone();
        self.tasks.spawn(
            TaskSpec {
                kind: TaskKind::InstallModpack,
                subject: title.to_string(),
                instance_id: None,
            },
            move |progress| async move {
                progress.set_stage(Stage::DownloadingMods);
                let archive = services
                    .paths
                    .cache
                    .join("modpacks")
                    .join(format!("curseforge-{}.zip", file.id));
                download_all(
                    services.curseforge.downloads(),
                    vec![Download {
                        url,
                        path: archive.clone(),
                        checksum: Some(Checksum::Sha1(sha1)),
                        size: Some(file.file_length).filter(|size| *size > 0),
                    }],
                    1,
                    VerifyMode::Full,
                    &progress,
                )
                .await?;
                let result = services
                    .import_curseforge_archive(&archive, &progress)
                    .await;
                if let Err(err) = tokio::fs::remove_file(&archive).await {
                    tracing::warn!(error = %err, "failed to remove cached modpack");
                }
                result.map(|_| ())
            },
        )
    }

    pub(crate) async fn import_curseforge_archive(
        &self,
        archive: &Path,
        progress: &Progress,
    ) -> CoreResult<Instance> {
        progress.set_stage(Stage::FetchingMetadata);
        let source = archive.to_path_buf();
        let manifest = tokio::task::spawn_blocking(move || read_manifest(&source))
            .await
            .map_err(|err| CoreError::Unsupported(format!("modpack read panicked: {err}")))??;
        let plan = plan(manifest)?;
        let entries = self.pack_entries(&plan).await?;
        progress.check_cancelled()?;
        let instance = self
            .instances
            .create(NewInstance {
                name: plan.name.clone(),
                minecraft_version: plan.minecraft_version.clone(),
                loader: plan.loader.clone(),
                copy_settings_from: None,
            })
            .await?;
        match self
            .fill_curseforge_instance(&instance, &plan, &entries, archive, progress)
            .await
        {
            Ok(()) => Ok(instance),
            Err(err) => {
                if let Err(cleanup) = self.instances.delete(&instance.id).await {
                    tracing::warn!(error = %cleanup, "failed to remove incomplete modpack instance");
                }
                Err(err)
            }
        }
    }

    async fn pack_entries(&self, plan: &PackPlan) -> CoreResult<Vec<PackEntry>> {
        let file_ids: Vec<u32> = plan.files.iter().map(|f| f.file_id).collect();
        let mut files: HashMap<u32, CfFile> = self
            .curseforge
            .files(&file_ids)
            .await?
            .into_iter()
            .map(|file| (file.id, file))
            .collect();
        let mut mod_ids: Vec<u32> = plan.files.iter().map(|f| f.project_id).collect();
        mod_ids.sort_unstable();
        mod_ids.dedup();
        let projects: HashMap<u32, CfMod> = self
            .curseforge
            .projects(&mod_ids)
            .await?
            .into_iter()
            .map(|project| (project.id, project))
            .collect();
        let mut entries = Vec::with_capacity(plan.files.len());
        for listed in &plan.files {
            let file = files
                .remove(&listed.file_id)
                .filter(|file| file.mod_id == listed.project_id)
                .ok_or_else(|| {
                    CoreError::NotFound(format!(
                        "CurseForge file {} of project {}",
                        listed.file_id, listed.project_id
                    ))
                })?;
            let project = projects.get(&listed.project_id).cloned();
            let Some(kind) = kind_for(project.as_ref(), &file) else {
                tracing::warn!(file = %file.file_name, "skipping modpack file of an unsupported type");
                continue;
            };
            validate_content_file_name(&file.file_name, kind)?;
            entries.push(PackEntry {
                file,
                project,
                kind,
            });
        }
        Ok(entries)
    }

    async fn fill_curseforge_instance(
        &self,
        instance: &Instance,
        plan: &PackPlan,
        entries: &[PackEntry],
        archive: &Path,
        progress: &Progress,
    ) -> CoreResult<()> {
        let game_dir = self.instances.game_dir(&instance.id)?;
        let instance_dir = self.instances.instance_dir(&instance.id)?;
        let mods = ModsDir::new(instance_dir.clone(), &game_dir);
        let target = CfTarget {
            game_version: &plan.minecraft_version,
            loader: plan.loader.mod_loader().map(|(loader, _)| loader),
            kind: ContentKind::Mod,
        };
        let files: Vec<CfFile> = entries.iter().map(|e| e.file.clone()).collect();
        let sources = find_sources(&self.modrinth, &files).await?;

        progress.set_stage(Stage::DownloadingMods);
        let located: Vec<(&PackEntry, &FileSource)> = entries
            .iter()
            .filter_map(|entry| sources.found.get(&entry.file.id).map(|s| (entry, s)))
            .collect();
        let downloads = located
            .iter()
            .map(|(entry, source)| Download {
                url: source.url.clone(),
                path: mods.dir(entry.kind).join(&entry.file.file_name),
                checksum: Some(source.checksum.clone()),
                size: Some(entry.file.file_length).filter(|size| *size > 0),
            })
            .collect();
        Clients {
            http: &self.http,
            curseforge: &self.curseforge,
            modrinth: &self.modrinth,
        }
        .download(downloads, DOWNLOAD_CONCURRENCY, progress)
        .await?;
        progress.check_cancelled()?;

        progress.set_stage(Stage::ExtractingFiles);
        let source = archive.to_path_buf();
        let target_dir = game_dir.clone();
        let prefix = format!("{}/", plan.overrides);
        tokio::task::spawn_blocking(move || extract_prefixed(&source, &target_dir, &[&prefix]))
            .await
            .map_err(|err| {
                CoreError::Unsupported(format!("override extraction panicked: {err}"))
            })??;
        progress.check_cancelled()?;

        progress.set_stage(Stage::FetchingMetadata);
        let mut installed = Vec::with_capacity(located.len());
        for (entry, source) in &located {
            let path = mods.dir(entry.kind).join(&entry.file.file_name);
            let sha512 = content_hash(path, &source.checksum).await?;
            installed.push(entry.installed(&target, sha512));
        }
        let index = mods.load().await?;
        mods.commit(index, &installed).await?;
        let pending: Vec<PendingDownload> = entries
            .iter()
            .filter(|entry| !sources.found.contains_key(&entry.file.id))
            .map(|entry| entry.pending(&target))
            .collect();
        if !pending.is_empty() {
            tracing::info!(
                count = pending.len(),
                "modpack files need a manual download"
            );
        }
        save_pending(&instance_dir, pending).await?;
        if let Err(err) = index_content(&self.modrinth, &mods, &game_dir).await {
            tracing::warn!(error = %err, "could not identify modpack overrides");
        }
        progress.set_stage(Stage::Finished);
        Ok(())
    }

    pub async fn pending_downloads(&self, instance_id: &str) -> CoreResult<Vec<PendingDownload>> {
        let instance = self.instances.get(instance_id).await?;
        load_pending(&self.instances.instance_dir(&instance.id)?).await
    }

    pub async fn collect_pending_downloads(
        &self,
        instance_id: &str,
        folder: &Path,
    ) -> CoreResult<Vec<PendingDownload>> {
        let instance = self.instances.get(instance_id).await?;
        let instance_dir = self.instances.instance_dir(&instance.id)?;
        let pending = load_pending(&instance_dir).await?;
        if pending.is_empty() || self.games.is_running(&instance.id) {
            return Ok(pending);
        }
        let folder = folder.to_path_buf();
        let candidates = pending.clone();
        let found = tokio::task::spawn_blocking(move || find_in_folder(&folder, &candidates))
            .await
            .map_err(|err| CoreError::Unsupported(format!("download scan panicked: {err}")))?;
        if found.is_empty() {
            return Ok(pending);
        }
        self.adopt_pending(&instance, found).await
    }

    pub async fn provide_pending_download(
        &self,
        instance_id: &str,
        file_id: u32,
        path: &Path,
    ) -> CoreResult<Vec<PendingDownload>> {
        let instance = self.instances.get(instance_id).await?;
        self.ensure_stopped(&instance)?;
        let instance_dir = self.instances.instance_dir(&instance.id)?;
        let pending = load_pending(&instance_dir).await?;
        let item = pending
            .iter()
            .find(|item| item.file_id == file_id)
            .ok_or_else(|| CoreError::NotFound(format!("pending download {file_id}")))?
            .clone();
        let candidate = path.to_path_buf();
        let size = tokio::fs::metadata(path).await.at_path(path)?.len();
        let matches = tokio::task::spawn_blocking(move || item.matches(&candidate, size))
            .await
            .map_err(|err| CoreError::Unsupported(format!("file check panicked: {err}")))?;
        if !matches {
            return Err(CoreError::InvalidInput(
                "this is not the file the modpack expects".into(),
            ));
        }
        self.adopt_pending(&instance, HashMap::from([(file_id, path.to_path_buf())]))
            .await
    }

    pub async fn skip_pending_download(
        &self,
        instance_id: &str,
        file_id: u32,
    ) -> CoreResult<Vec<PendingDownload>> {
        let instance = self.instances.get(instance_id).await?;
        let instance_dir = self.instances.instance_dir(&instance.id)?;
        let _guard = self.mods_lock.lock().await;
        let mut pending = load_pending(&instance_dir).await?;
        pending.retain(|item| item.file_id != file_id);
        save_pending(&instance_dir, pending.clone()).await?;
        Ok(pending)
    }

    async fn adopt_pending(
        &self,
        instance: &Instance,
        found: HashMap<u32, PathBuf>,
    ) -> CoreResult<Vec<PendingDownload>> {
        let _guard = self.mods_lock.lock().await;
        let instance_dir = self.instances.instance_dir(&instance.id)?;
        let pending = load_pending(&instance_dir).await?;
        let mods = self.mods_dir(instance)?;
        let mut installed = Vec::new();
        let mut remaining = Vec::new();
        for item in pending {
            let Some(source) = found.get(&item.file_id) else {
                remaining.push(item);
                continue;
            };
            validate_content_file_name(&item.file_name, item.kind)?;
            let dir = mods.dir(item.kind);
            tokio::fs::create_dir_all(&dir).await.at_path(&dir)?;
            let target = dir.join(&item.file_name);
            tokio::fs::copy(source, &target).await.at_path(&target)?;
            let sha512 = sha512_of(target).await?;
            installed.push(item.installed(sha512));
        }
        let index = mods.load().await?;
        mods.commit(index, &installed).await?;
        save_pending(&instance_dir, remaining.clone()).await?;
        Ok(remaining)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn manifest(loader: &str, files: serde_json::Value) -> Manifest {
        serde_json::from_value(serde_json::json!({
            "minecraft": {
                "version": "1.20.1",
                "modLoaders": [{ "id": loader, "primary": true }]
            },
            "manifestType": "minecraftModpack",
            "manifestVersion": 1,
            "name": "  Test\u{7}Pack ",
            "version": "1.0",
            "author": "someone",
            "files": files,
            "overrides": "overrides"
        }))
        .unwrap()
    }

    #[test]
    fn plans_forge_packs_and_drops_optional_files() {
        let plan = plan(manifest(
            "forge-47.2.0",
            serde_json::json!([
                { "projectID": 1, "fileID": 10, "required": true },
                { "projectID": 2, "fileID": 20, "required": false },
                { "projectID": 3, "fileID": 30 }
            ]),
        ))
        .unwrap();
        assert_eq!(plan.name, "TestPack");
        assert_eq!(
            plan.loader,
            LoaderSpec::Forge {
                loader_version: "47.2.0".into()
            }
        );
        let ids: Vec<u32> = plan.files.iter().map(|f| f.file_id).collect();
        assert_eq!(ids, vec![10, 30]);
    }

    #[test]
    fn plans_fabric_packs() {
        let plan = plan(manifest("fabric-0.15.11", serde_json::json!([]))).unwrap();
        assert_eq!(
            plan.loader,
            LoaderSpec::Fabric {
                loader_version: "0.15.11".into()
            }
        );
    }

    #[test]
    fn rejects_unsafe_overrides_and_other_manifests() {
        let mut bad = manifest("forge-47.2.0", serde_json::json!([]));
        bad.overrides = "../evil".into();
        assert!(plan(bad).is_err());
        let mut other = manifest("forge-47.2.0", serde_json::json!([]));
        other.manifest_type = "minecraftWorld".into();
        assert!(plan(other).is_err());
    }

    fn file(id: u32, name: &str) -> CfFile {
        serde_json::from_value(serde_json::json!({
            "id": id, "modId": 5, "displayName": name, "fileName": name,
            "releaseType": 1, "gameVersions": ["1.20.1", "Forge"]
        }))
        .unwrap()
    }

    fn project(class_id: u32) -> CfMod {
        serde_json::from_value(serde_json::json!({
            "id": 5, "name": "P", "classId": class_id,
            "links": { "websiteUrl": "https://www.curseforge.com/minecraft/mc-mods/p" }
        }))
        .unwrap()
    }

    #[test]
    fn maps_classes_to_folders() {
        assert_eq!(
            kind_for(Some(&project(6)), &file(1, "a.jar")),
            Some(ContentKind::Mod)
        );
        assert_eq!(
            kind_for(Some(&project(12)), &file(1, "a.zip")),
            Some(ContentKind::ResourcePack)
        );
        assert_eq!(
            kind_for(Some(&project(6552)), &file(1, "a.zip")),
            Some(ContentKind::Shader)
        );
        assert_eq!(kind_for(Some(&project(17)), &file(1, "a.zip")), None);
        assert_eq!(kind_for(None, &file(1, "a.jar")), Some(ContentKind::Mod));
        assert_eq!(
            page_url(Some(&project(6)), &file(42, "a.jar")),
            "https://www.curseforge.com/minecraft/mc-mods/p/download/42"
        );
    }

    fn instance(loader: LoaderSpec) -> Instance {
        serde_json::from_value(serde_json::json!({
            "schema": 1,
            "id": "x",
            "name": "My Pack",
            "minecraftVersion": "1.20.1",
            "loader": loader,
            "createdAt": 0
        }))
        .unwrap()
    }

    #[test]
    fn exported_manifests_read_back_as_the_same_pack() {
        let instance = instance(LoaderSpec::Forge {
            loader_version: "47.2.0".into(),
        });
        let files = vec![ExportFile {
            project_id: 5,
            file_id: 50,
            required: true,
        }];
        let json = serde_json::to_value(export_manifest(&instance, files).unwrap()).unwrap();
        assert_eq!(json["files"][0]["projectID"], 5);
        let read: Manifest = serde_json::from_value(json).unwrap();
        let plan = plan(read).unwrap();
        assert_eq!(plan.name, "My Pack");
        assert_eq!(plan.files[0].file_id, 50);
        assert_eq!(plan.minecraft_version, "1.20.1");
        assert_eq!(plan.loader, instance.loader);
    }

    #[test]
    fn legacy_fabric_cannot_be_exported() {
        let legacy = instance(LoaderSpec::LegacyFabric {
            loader_version: "0.15.0".into(),
        });
        assert!(export_manifest(&legacy, Vec::new()).is_err());
        assert!(
            export_manifest(&instance(LoaderSpec::Vanilla), Vec::new())
                .unwrap()
                .minecraft
                .mod_loaders
                .is_empty()
        );
    }

    #[test]
    fn detects_curseforge_archives() {
        let dir = tempfile::tempdir().unwrap();
        let write = |name: &str, entries: &[&str]| {
            let path = dir.path().join(name);
            let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
            for entry in entries {
                zip.start_file(*entry, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(b"{}").unwrap();
            }
            zip.finish().unwrap();
            path
        };
        assert!(is_curseforge_pack(&write(
            "cf.zip",
            &["manifest.json", "overrides/a.txt"]
        )));
        assert!(!is_curseforge_pack(&write(
            "mr.zip",
            &["manifest.json", MODRINTH_INDEX]
        )));
        assert!(!is_curseforge_pack(&write("rp.zip", &["pack.mcmeta"])));
    }

    fn pending(file_id: u32, data: &[u8], name: &str) -> PendingDownload {
        PendingDownload {
            file_id,
            mod_id: 5,
            title: "P".into(),
            file_name: name.into(),
            kind: ContentKind::Mod,
            sha1: Some(sha1_hex(data)),
            size: data.len() as u64,
            page_url: String::new(),
            icon_url: None,
            version_number: "1".into(),
            game_versions: Vec::new(),
            loaders: Vec::new(),
        }
    }

    #[test]
    fn finds_renamed_downloads_by_hash() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("mod (1).jar"), b"right bytes").unwrap();
        std::fs::write(dir.path().join("mod.jar"), b"wrong bytes").unwrap();
        std::fs::create_dir(dir.path().join("folder")).unwrap();
        let items = vec![
            pending(1, b"right bytes", "mod.jar"),
            pending(2, b"missing", "x.jar"),
        ];
        let found = find_in_folder(dir.path(), &items);
        assert_eq!(found.len(), 1);
        assert_eq!(found[&1], dir.path().join("mod (1).jar"));
    }

    #[tokio::test]
    async fn pending_list_is_removed_when_empty() {
        let dir = tempfile::tempdir().unwrap();
        save_pending(dir.path(), vec![pending(1, b"x", "a.jar")])
            .await
            .unwrap();
        assert_eq!(load_pending(dir.path()).await.unwrap().len(), 1);
        save_pending(dir.path(), Vec::new()).await.unwrap();
        assert!(!pending_path(dir.path()).exists());
        assert!(load_pending(dir.path()).await.unwrap().is_empty());
    }
}
