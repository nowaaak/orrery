use std::io::Read;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::IoResultExt;
use crate::instances::Instance;
use crate::instances::mods::{ContentKind, InstalledMod, validate_content_file_name};
use crate::loaders::ModLoader;
use crate::modpacks::sha512_file;
use crate::optifine;
use crate::services::Services;
use crate::{CoreError, CoreResult};

const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 512 * 1024;
const LOCAL_PREFIX: &str = "local-";
const PARTIAL_SUFFIX: &str = ".orrery-part";

#[derive(Debug, Default, PartialEq, Eq)]
struct Manifest {
    name: Option<String>,
    version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModFamily {
    Fabric,
    Quilt,
    Forge,
    NeoForge,
    OptiFine,
}

impl ModFamily {
    fn name(self) -> &'static str {
        match self {
            Self::Fabric => "Fabric",
            Self::Quilt => "Quilt",
            Self::Forge => "Forge",
            Self::NeoForge => "NeoForge",
            Self::OptiFine => "OptiFine",
        }
    }

    fn runs_on(self, loader: ModLoader) -> bool {
        match self {
            Self::Fabric => loader.runs_fabric_mods(),
            Self::Quilt => loader == ModLoader::Quilt,
            Self::Forge => loader == ModLoader::Forge,
            Self::NeoForge => loader == ModLoader::NeoForge,
            Self::OptiFine => loader == ModLoader::Forge,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Detected {
    kind: ContentKind,
    manifest: Manifest,
    family: Option<ModFamily>,
    minecraft_version: Option<String>,
}

fn read_entry(archive: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<String> {
    let entry = archive.by_name(name).ok()?;
    let mut text = String::new();
    entry
        .take(MAX_MANIFEST_BYTES)
        .read_to_string(&mut text)
        .ok()?;
    Some(text)
}

fn fabric_manifest(text: &str) -> Manifest {
    let json: Value = serde_json::from_str(text).unwrap_or_default();
    Manifest {
        name: json["name"].as_str().map(str::to_string),
        version: json["version"].as_str().map(str::to_string),
    }
}

fn quilt_manifest(text: &str) -> Manifest {
    let json: Value = serde_json::from_str(text).unwrap_or_default();
    let loader = &json["quilt_loader"];
    Manifest {
        name: loader["metadata"]["name"].as_str().map(str::to_string),
        version: loader["version"].as_str().map(str::to_string),
    }
}

fn toml_string(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        if name.trim() != key {
            return None;
        }
        let value = value.trim();
        let value = value.strip_prefix('"')?;
        let value = &value[..value.find('"')?];
        (!value.is_empty() && !value.starts_with("${")).then(|| value.to_string())
    })
}

fn forge_manifest(text: &str) -> Manifest {
    Manifest {
        name: toml_string(text, "displayName"),
        version: toml_string(text, "version"),
    }
}

fn depends_on_neoforge(text: &str) -> bool {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    compact.contains("modId=\"neoforge\"")
}

fn mcmod_manifest(text: &str) -> Manifest {
    let json: Value = serde_json::from_str(text).unwrap_or_default();
    let first = json
        .get("modList")
        .and_then(|list| list.get(0))
        .or_else(|| json.get(0))
        .cloned()
        .unwrap_or_default();
    Manifest {
        name: first["name"].as_str().map(str::to_string),
        version: first["version"].as_str().map(str::to_string),
    }
}

fn detect(path: &Path) -> CoreResult<Detected> {
    let invalid = |message: &str| CoreError::Archive {
        path: path.to_path_buf(),
        message: message.into(),
    };
    let file = std::fs::File::open(path).at_path(path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|_| invalid("the file is not a zip archive"))?;
    let names: Vec<String> = archive
        .file_names()
        .filter_map(|name| name.ok().map(|name| name.into_owned()))
        .collect();
    let has = |name: &str| names.iter().any(|n| n == name);
    let has_prefix = |prefix: &str| names.iter().any(|n| n.starts_with(prefix));
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase);

    match extension.as_deref() {
        Some("jar") => {
            if let Some(jar) = optifine::read(&mut archive, path)? {
                return Ok(Detected {
                    kind: ContentKind::Mod,
                    manifest: Manifest {
                        name: Some("OptiFine".into()),
                        version: Some(jar.label()),
                    },
                    family: Some(ModFamily::OptiFine),
                    minecraft_version: Some(jar.minecraft_version),
                });
            }
            let detected = |manifest, family| Detected {
                kind: ContentKind::Mod,
                manifest,
                family: Some(family),
                minecraft_version: None,
            };
            if let Some(text) = read_entry(&mut archive, "fabric.mod.json") {
                return Ok(detected(fabric_manifest(&text), ModFamily::Fabric));
            }
            if let Some(text) = read_entry(&mut archive, "quilt.mod.json") {
                return Ok(detected(quilt_manifest(&text), ModFamily::Quilt));
            }
            if let Some(text) = read_entry(&mut archive, "META-INF/neoforge.mods.toml") {
                return Ok(detected(forge_manifest(&text), ModFamily::NeoForge));
            }
            if let Some(text) = read_entry(&mut archive, "META-INF/mods.toml") {
                let family = if depends_on_neoforge(&text) {
                    ModFamily::NeoForge
                } else {
                    ModFamily::Forge
                };
                return Ok(detected(forge_manifest(&text), family));
            }
            if let Some(text) = read_entry(&mut archive, "mcmod.info") {
                return Ok(detected(mcmod_manifest(&text), ModFamily::Forge));
            }
            Err(invalid(
                "the jar is not a Fabric, Quilt, Forge or NeoForge mod",
            ))
        }
        Some("zip") => {
            if has_prefix("shaders/") {
                return Ok(Detected {
                    kind: ContentKind::Shader,
                    manifest: Manifest::default(),
                    family: None,
                    minecraft_version: None,
                });
            }
            if has("pack.mcmeta") && has_prefix("assets/") {
                return Ok(Detected {
                    kind: ContentKind::ResourcePack,
                    manifest: Manifest::default(),
                    family: None,
                    minecraft_version: None,
                });
            }
            if has("pack.mcmeta") && has_prefix("data/") {
                return Err(CoreError::Unsupported(
                    "this is a data pack, add it to a world in the worlds tab".into(),
                ));
            }
            Err(invalid(
                "the zip is neither a resource pack nor a shader pack",
            ))
        }
        _ => Err(CoreError::InvalidInput(
            "drop .jar mods, .zip packs or .mrpack modpacks".into(),
        )),
    }
}

fn check_loader(instance: &Instance, detected: &Detected) -> CoreResult<()> {
    if let Some(minecraft) = &detected.minecraft_version
        && minecraft != &instance.minecraft_version
    {
        return Err(CoreError::Unsupported(format!(
            "this file is made for Minecraft {minecraft}, the instance runs {}",
            instance.minecraft_version
        )));
    }
    match (
        detected.kind,
        instance.loader.mod_loader().map(|(loader, _)| loader),
    ) {
        (ContentKind::ResourcePack, _) => Ok(()),
        (_, None) if detected.family == Some(ModFamily::OptiFine) => Err(CoreError::Unsupported(
            "OptiFine runs as a Forge mod, use a Forge instance or the OptiFine replacement pack"
                .into(),
        )),
        (_, None) => Err(CoreError::Unsupported(format!(
            "{} require an instance with a mod loader",
            detected.kind.project_type()
        ))),
        (ContentKind::Mod, Some(loader)) => match detected.family {
            Some(ModFamily::OptiFine) if loader != ModLoader::Forge => Err(CoreError::Unsupported(
                "OptiFine runs as a Forge mod, use a Forge instance or the OptiFine replacement pack"
                    .into(),
            )),
            Some(family) if !family.runs_on(loader) => Err(CoreError::Unsupported(format!(
                "this is a {} mod, the instance runs {}",
                family.name(),
                loader.display_name()
            ))),
            _ => Ok(()),
        },
        _ => Ok(()),
    }
}

fn copy_into(source: &Path, dir: &Path, file_name: &str) -> CoreResult<PathBuf> {
    std::fs::create_dir_all(dir).at_path(dir)?;
    let target = dir.join(file_name);
    if target.exists() || dir.join(format!("{file_name}.disabled")).exists() {
        return Err(CoreError::InvalidInput(format!(
            "{file_name} is already installed"
        )));
    }
    let partial = dir.join(format!("{file_name}{PARTIAL_SUFFIX}"));
    std::fs::copy(source, &partial).at_path(&partial)?;
    if let Err(err) = std::fs::rename(&partial, &target) {
        let _ = std::fs::remove_file(&partial);
        return Err(err).at_path(&target);
    }
    Ok(target)
}

async fn blocking<T: Send + 'static>(
    job: impl FnOnce() -> CoreResult<T> + Send + 'static,
) -> CoreResult<T> {
    tokio::task::spawn_blocking(job)
        .await
        .map_err(|err| CoreError::Unsupported(format!("file import panicked: {err}")))?
}

impl Services {
    pub async fn install_local_file(
        &self,
        instance_id: &str,
        source: PathBuf,
    ) -> CoreResult<InstalledMod> {
        let instance = self.instances.get(instance_id).await?;
        self.ensure_stopped(&instance)?;
        let metadata = tokio::fs::metadata(&source).await.at_path(&source)?;
        if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
            return Err(CoreError::InvalidInput(
                "only files smaller than 512 MB can be added".into(),
            ));
        }
        let probe = source.clone();
        let detected = blocking(move || detect(&probe)).await?;
        check_loader(&instance, &detected)?;
        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| CoreError::InvalidInput("the file name is not valid UTF-8".into()))?
            .to_string();
        validate_content_file_name(&file_name, detected.kind)?;

        let _guard = self.mods_lock.lock().await;
        let mods = self.mods_dir(&instance)?;
        let dir = mods.dir(detected.kind);
        let copy_source = source.clone();
        let copy_name = file_name.clone();
        let target = blocking(move || copy_into(&copy_source, &dir, &copy_name)).await?;
        let hash_target = target.clone();
        let sha512 = match blocking(move || sha512_file(&hash_target)).await {
            Ok(hash) => hash,
            Err(err) => {
                let _ = tokio::fs::remove_file(&target).await;
                return Err(err);
            }
        };

        let known = match self
            .modrinth
            .versions_for_hashes(std::slice::from_ref(&sha512))
            .await
        {
            Ok(mut versions) => versions.remove(&sha512),
            Err(err) => {
                tracing::info!(error = %err, "Modrinth lookup for a dropped file failed");
                None
            }
        };
        let project = match &known {
            Some(version) => self
                .modrinth
                .projects(std::slice::from_ref(&version.project_id))
                .await
                .ok()
                .and_then(|projects| projects.into_iter().next()),
            None => None,
        };
        let fallback_title = Path::new(&file_name)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(&file_name)
            .to_string();
        let entry = InstalledMod {
            project_id: known
                .as_ref()
                .map(|v| v.project_id.clone())
                .unwrap_or_else(|| format!("{LOCAL_PREFIX}{}", &sha512[..16])),
            version_id: known.as_ref().map(|v| v.id.clone()).unwrap_or_default(),
            title: project
                .as_ref()
                .map(|p| p.title.clone())
                .or(detected.manifest.name.clone())
                .unwrap_or(fallback_title),
            version_number: known
                .as_ref()
                .map(|v| v.version_number.clone())
                .or(detected.manifest.version.clone())
                .unwrap_or_default(),
            file_name,
            sha512,
            enabled: true,
            dependency: false,
            icon_url: project.and_then(|p| p.icon_url),
            kind: detected.kind,
            game_versions: known
                .as_ref()
                .map(|v| v.game_versions.clone())
                .unwrap_or_default(),
            loaders: known
                .as_ref()
                .map(|v| v.loaders.clone())
                .unwrap_or_default(),
            pack: None,
            pinned: false,
        };

        let mut index = mods.load().await?;
        if index.mods.iter().any(|m| m.project_id == entry.project_id) {
            let _ = tokio::fs::remove_file(&target).await;
            return Err(CoreError::InvalidInput(format!(
                "{} is already installed, use the content tab to update it",
                entry.title
            )));
        }
        index.mods.push(entry.clone());
        index.mods.sort_by_key(|m| m.title.to_lowercase());
        mods.save(&index).await?;
        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_zip(path: &Path, files: &[(&str, &str)]) {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, content) in files {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn detects_content_types() {
        let dir = tempfile::tempdir().unwrap();
        let case = |name: &str, files: &[(&str, &str)]| {
            let path = dir.path().join(name);
            write_zip(&path, files);
            detect(&path)
        };
        let fabric = case(
            "sodium.jar",
            &[("fabric.mod.json", r#"{"name":"Sodium","version":"0.6.0"}"#)],
        )
        .unwrap();
        assert_eq!(fabric.kind, ContentKind::Mod);
        assert_eq!(fabric.manifest.name.as_deref(), Some("Sodium"));
        assert_eq!(fabric.family, Some(ModFamily::Fabric));

        let quilt = case(
            "q.jar",
            &[(
                "quilt.mod.json",
                r#"{"quilt_loader":{"version":"1.2","metadata":{"name":"Q"}}}"#,
            )],
        )
        .unwrap();
        assert_eq!(quilt.family, Some(ModFamily::Quilt));
        assert_eq!(quilt.manifest.version.as_deref(), Some("1.2"));

        assert_eq!(
            case("shader.zip", &[("shaders/final.fsh", "x")])
                .unwrap()
                .kind,
            ContentKind::Shader
        );
        assert_eq!(
            case(
                "pack.zip",
                &[("pack.mcmeta", "{}"), ("assets/minecraft/x", "")]
            )
            .unwrap()
            .kind,
            ContentKind::ResourcePack
        );
        assert!(matches!(
            case("data.zip", &[("pack.mcmeta", "{}"), ("data/x/y", "")]),
            Err(CoreError::Unsupported(_))
        ));
        let forge = case(
            "jei.jar",
            &[(
                "META-INF/mods.toml",
                "modLoader=\"javafml\"\n[[mods]]\nmodId=\"jei\"\nversion=\"${file.jarVersion}\"\ndisplayName=\"Just Enough Items\"\n[[dependencies.jei]]\n    modId = \"forge\"\n",
            )],
        )
        .unwrap();
        assert_eq!(forge.family, Some(ModFamily::Forge));
        assert_eq!(forge.manifest.name.as_deref(), Some("Just Enough Items"));
        assert_eq!(forge.manifest.version, None);
        let early_neoforge = case(
            "early.jar",
            &[(
                "META-INF/mods.toml",
                "[[mods]]\nmodId=\"x\"\n[[dependencies.x]]\n    modId = \"neoforge\"\n",
            )],
        )
        .unwrap();
        assert_eq!(early_neoforge.family, Some(ModFamily::NeoForge));
        let neoforge = case(
            "neo.jar",
            &[("META-INF/neoforge.mods.toml", "version=\"2.1\"\n")],
        )
        .unwrap();
        assert_eq!(neoforge.family, Some(ModFamily::NeoForge));
        assert_eq!(neoforge.manifest.version.as_deref(), Some("2.1"));
        let legacy = case(
            "old.jar",
            &[(
                "mcmod.info",
                r#"[{"modid":"old","name":"Old Mod","version":"1.0"}]"#,
            )],
        )
        .unwrap();
        assert_eq!(legacy.family, Some(ModFamily::Forge));
        assert_eq!(legacy.manifest.name.as_deref(), Some("Old Mod"));
        let optifine = case(
            "OptiFine_1.20.1_HD_U_I6.jar",
            &[
                ("net/optifine/Config.class", "..OptiFine_1.20.1_HD_U_I6.."),
                ("META-INF/mods.toml", "modId=\"optifine\"\n"),
            ],
        )
        .unwrap();
        assert_eq!(optifine.family, Some(ModFamily::OptiFine));
        assert_eq!(optifine.minecraft_version.as_deref(), Some("1.20.1"));
        assert_eq!(optifine.manifest.version.as_deref(), Some("HD U I6"));
        assert!(case("plain.jar", &[("a.class", "")]).is_err());
        assert!(case("random.zip", &[("readme.txt", "")]).is_err());
        std::fs::write(dir.path().join("broken.jar"), b"not a zip").unwrap();
        assert!(detect(&dir.path().join("broken.jar")).is_err());
    }

    #[test]
    fn mods_only_go_into_instances_with_their_loader() {
        let instance = |loader: serde_json::Value| -> Instance {
            serde_json::from_value(serde_json::json!({
                "schema": 1, "id": "i", "name": "I", "minecraftVersion": "1.20.1",
                "loader": loader, "createdAt": 0
            }))
            .unwrap()
        };
        let mod_of = |family| Detected {
            kind: ContentKind::Mod,
            manifest: Manifest::default(),
            family: Some(family),
            minecraft_version: None,
        };
        let fabric = instance(serde_json::json!({ "type": "fabric", "loaderVersion": "0.16" }));
        let legacy =
            instance(serde_json::json!({ "type": "legacy_fabric", "loaderVersion": "0.20" }));
        let quilt = instance(serde_json::json!({ "type": "quilt", "loaderVersion": "0.27" }));
        let forge = instance(serde_json::json!({ "type": "forge", "loaderVersion": "47.4.28" }));
        let neoforge =
            instance(serde_json::json!({ "type": "neo_forge", "loaderVersion": "21.1.1" }));
        let vanilla = instance(serde_json::json!({ "type": "vanilla" }));

        assert!(check_loader(&fabric, &mod_of(ModFamily::Fabric)).is_ok());
        assert!(check_loader(&legacy, &mod_of(ModFamily::Fabric)).is_ok());
        assert!(check_loader(&quilt, &mod_of(ModFamily::Fabric)).is_ok());
        assert!(check_loader(&quilt, &mod_of(ModFamily::Quilt)).is_ok());
        assert!(check_loader(&fabric, &mod_of(ModFamily::Quilt)).is_err());
        assert!(check_loader(&forge, &mod_of(ModFamily::Forge)).is_ok());
        assert!(check_loader(&forge, &mod_of(ModFamily::NeoForge)).is_err());
        assert!(check_loader(&neoforge, &mod_of(ModFamily::NeoForge)).is_ok());
        assert!(check_loader(&neoforge, &mod_of(ModFamily::Fabric)).is_err());
        assert!(check_loader(&fabric, &mod_of(ModFamily::Forge)).is_err());
        assert!(check_loader(&vanilla, &mod_of(ModFamily::Fabric)).is_err());
        let pack = Detected {
            kind: ContentKind::ResourcePack,
            manifest: Manifest::default(),
            family: None,
            minecraft_version: None,
        };
        assert!(check_loader(&vanilla, &pack).is_ok());

        let optifine = |minecraft: &str| Detected {
            kind: ContentKind::Mod,
            manifest: Manifest::default(),
            family: Some(ModFamily::OptiFine),
            minecraft_version: Some(minecraft.into()),
        };
        assert!(check_loader(&forge, &optifine("1.20.1")).is_ok());
        assert!(check_loader(&forge, &optifine("1.21.1")).is_err());
        assert!(check_loader(&fabric, &optifine("1.20.1")).is_err());
        assert!(check_loader(&vanilla, &optifine("1.20.1")).is_err());
    }

    #[test]
    fn copies_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("a.jar");
        std::fs::write(&source, b"x").unwrap();
        let mods = dir.path().join("mods");
        copy_into(&source, &mods, "a.jar").unwrap();
        assert!(copy_into(&source, &mods, "a.jar").is_err());
        std::fs::write(mods.join("b.jar.disabled"), b"x").unwrap();
        assert!(copy_into(&source, &mods, "b.jar").is_err());
        assert!(!mods.join("a.jar.orrery-part").exists());
    }
}
