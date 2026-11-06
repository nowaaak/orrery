use std::path::{Path, PathBuf};

use tokio::sync::Mutex;

use super::model::{
    Instance, InstanceUpdate, JavaSettings, LaunchSettings, LoaderSpec, NewInstance,
    ResolutionSetting,
};
use crate::error::IoResultExt;
use crate::game_options::{self, OPTIONS_FILE};
use crate::minecraft::hooks::validate_command;
use crate::minecraft::manifest::validate_version_id;
use crate::servers::ServerAddress;
use crate::servers::list::SERVERS_FILE;
use crate::settings::validate_memory;
use crate::storage::{self, Versioned, unix_now};
use crate::syncpacks::SyncSubscription;
use crate::trash::{Trash, TrashEntry, TrashItem, TrashKind};
use crate::{CoreError, CoreResult};

const INSTANCE_FILE: &str = "instance.json";
const GAME_DIR: &str = "minecraft";
const MAX_NAME_CHARS: usize = 64;
const MAX_SLUG_LEN: usize = 40;
const MAX_ENV_VARS: usize = 64;
const MAX_GROUP_CHARS: usize = 40;
const MAX_ICON_BYTES: usize = 1024 * 1024;
const ICON_PREFIX: &str = "icon-";

pub struct InstanceStore {
    root: PathBuf,
    write_lock: Mutex<()>,
}

pub fn validate_instance_id(id: &str) -> CoreResult<()> {
    let valid = !id.is_empty()
        && id.len() <= MAX_SLUG_LEN + 4
        && !id.starts_with('-')
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if valid {
        Ok(())
    } else {
        Err(CoreError::InvalidInput(format!(
            "invalid instance id {id:?}"
        )))
    }
}

fn slugify(name: &str) -> String {
    let mut slug = String::new();
    for c in name.to_lowercase().chars() {
        let mapped = match c {
            'ä' => "ae".to_string(),
            'ö' => "oe".to_string(),
            'ü' => "ue".to_string(),
            'ß' => "ss".to_string(),
            c if c.is_ascii_alphanumeric() => c.to_string(),
            _ => "-".to_string(),
        };
        for m in mapped.chars() {
            if m == '-' && (slug.is_empty() || slug.ends_with('-')) {
                continue;
            }
            slug.push(m);
        }
    }
    slug.truncate(MAX_SLUG_LEN);
    let slug = slug.trim_end_matches('-').to_string();
    if slug.is_empty() {
        "instanz".to_string()
    } else {
        slug
    }
}

fn validate_name(name: &str) -> CoreResult<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return Err(CoreError::InvalidInput(format!(
            "instance name must have between 1 and {MAX_NAME_CHARS} characters"
        )));
    }
    Ok(name.to_string())
}

fn validate_loader(loader: &LoaderSpec) -> CoreResult<()> {
    match loader {
        LoaderSpec::Vanilla => Ok(()),
        LoaderSpec::Fabric { loader_version }
        | LoaderSpec::LegacyFabric { loader_version }
        | LoaderSpec::Quilt { loader_version }
        | LoaderSpec::Forge { loader_version }
        | LoaderSpec::NeoForge { loader_version } => validate_version_id(loader_version),
    }
}

fn validate_java(java: &JavaSettings) -> CoreResult<()> {
    if let (Some(min), Some(max)) = (java.min_memory_mb, java.max_memory_mb) {
        validate_memory(min, max)?;
    }
    if let Some(path) = &java.java_path
        && !Path::new(path).is_absolute()
    {
        return Err(CoreError::InvalidInput("Java path must be absolute".into()));
    }
    validate_args(&java.extra_jvm_args, "JVM")
}

fn validate_args(args: &[String], what: &str) -> CoreResult<()> {
    if args
        .iter()
        .any(|arg| arg.trim().is_empty() || arg.contains('\n') || arg.contains('\0'))
    {
        return Err(CoreError::InvalidInput(format!("invalid {what} argument")));
    }
    Ok(())
}

fn trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_launch(launch: LaunchSettings) -> CoreResult<LaunchSettings> {
    let launch = LaunchSettings {
        account_id: trimmed(launch.account_id),
        join_server: trimmed(launch.join_server),
        wrapper: trimmed(launch.wrapper),
        pre_launch: trimmed(launch.pre_launch),
        post_exit: trimmed(launch.post_exit),
        ..launch
    };
    validate_args(&launch.extra_game_args, "game")?;
    if let Some(server) = &launch.join_server {
        ServerAddress::parse(server)?;
    }
    for command in [&launch.wrapper, &launch.pre_launch, &launch.post_exit]
        .into_iter()
        .flatten()
    {
        validate_command(command)?;
    }
    if launch.env.len() > MAX_ENV_VARS {
        return Err(CoreError::InvalidInput(format!(
            "at most {MAX_ENV_VARS} environment variables are allowed"
        )));
    }
    for (index, var) in launch.env.iter().enumerate() {
        let valid_key = var
            .key
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && var
                .key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !valid_key || var.value.contains('\0') {
            return Err(CoreError::InvalidInput(format!(
                "invalid environment variable {:?}",
                var.key
            )));
        }
        if launch.env[..index].iter().any(|other| other.key == var.key) {
            return Err(CoreError::InvalidInput(format!(
                "environment variable {} is set twice",
                var.key
            )));
        }
    }
    Ok(launch)
}

fn normalize_group(group: Option<String>) -> CoreResult<Option<String>> {
    let group = trimmed(group);
    if let Some(group) = &group
        && (group.chars().count() > MAX_GROUP_CHARS || group.chars().any(char::is_control))
    {
        return Err(CoreError::InvalidInput(format!(
            "group names must have at most {MAX_GROUP_CHARS} characters"
        )));
    }
    Ok(group)
}

pub fn icon_extension(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => Some("png"),
        [0xFF, 0xD8, 0xFF, ..] => Some("jpg"),
        [b'G', b'I', b'F', b'8', ..] => Some("gif"),
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => Some("webp"),
        _ => None,
    }
}

fn validate_icon_name(name: &str) -> CoreResult<()> {
    let valid = name
        .strip_prefix(ICON_PREFIX)
        .and_then(|rest| rest.split_once('.'))
        .is_some_and(|(stamp, ext)| {
            !stamp.is_empty()
                && stamp.chars().all(|c| c.is_ascii_digit())
                && matches!(ext, "png" | "jpg" | "gif" | "webp")
        });
    if valid {
        Ok(())
    } else {
        Err(CoreError::InvalidInput(format!(
            "invalid icon name {name:?}"
        )))
    }
}

fn validate_resolution(resolution: Option<ResolutionSetting>) -> CoreResult<()> {
    match resolution {
        Some(r) if !(320..=7680).contains(&r.width) || !(240..=4320).contains(&r.height) => {
            Err(CoreError::InvalidInput("resolution is out of range".into()))
        }
        _ => Ok(()),
    }
}

impl InstanceStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            write_lock: Mutex::new(()),
        }
    }

    pub fn instance_dir(&self, id: &str) -> CoreResult<PathBuf> {
        validate_instance_id(id)?;
        Ok(self.root.join(id))
    }

    pub fn game_dir(&self, id: &str) -> CoreResult<PathBuf> {
        Ok(self.instance_dir(id)?.join(GAME_DIR))
    }

    pub async fn list(&self) -> CoreResult<Vec<Instance>> {
        let mut instances = Vec::new();
        let mut entries = match tokio::fs::read_dir(&self.root).await {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(instances),
            Err(err) => return Err(err).at_path(&self.root),
        };
        while let Some(entry) = entries.next_entry().await.at_path(&self.root)? {
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if validate_instance_id(&name).is_err() {
                continue;
            }
            match self.get(&name).await {
                Ok(instance) => instances.push(instance),
                Err(err) => {
                    tracing::warn!(instance = %name, error = %err, "skipping unreadable instance")
                }
            }
        }
        instances.sort_by(|a, b| {
            b.last_played_at
                .cmp(&a.last_played_at)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        Ok(instances)
    }

    pub async fn get(&self, id: &str) -> CoreResult<Instance> {
        let path = self.instance_dir(id)?.join(INSTANCE_FILE);
        let mut instance = storage::read_versioned::<Instance>(&path)
            .await?
            .ok_or_else(|| CoreError::NotFound(format!("instance {id}")))?;
        instance.id = id.to_string();
        Ok(instance)
    }

    pub async fn create(&self, new: NewInstance) -> CoreResult<Instance> {
        let name = validate_name(&new.name)?;
        validate_version_id(&new.minecraft_version)?;
        validate_loader(&new.loader)?;

        let _guard = self.write_lock.lock().await;
        let settings_source = match &new.copy_settings_from {
            Some(source_id) => Some(self.get(source_id).await?),
            None => None,
        };
        let id = self.unique_id(&slugify(&name)).await;
        let instance = Instance {
            schema: Instance::SCHEMA,
            id: id.clone(),
            name,
            minecraft_version: new.minecraft_version,
            loader: new.loader,
            java: JavaSettings::default(),
            resolution: None,
            launch: LaunchSettings::default(),
            group: None,
            pinned: false,
            icon: None,
            created_at: unix_now(),
            last_played_at: None,
            total_play_secs: 0,
            sync_packs: Vec::new(),
        };
        let game_dir = self.game_dir(&id)?;
        tokio::fs::create_dir_all(&game_dir)
            .await
            .at_path(&game_dir)?;
        let result = async {
            if let Some(source) = &settings_source {
                self.copy_settings(source, &instance, &game_dir).await?;
            }
            self.write(&instance).await
        }
        .await;
        if let Err(err) = result {
            let _ = tokio::fs::remove_dir_all(self.instance_dir(&id)?).await;
            return Err(err);
        }
        Ok(instance)
    }

    async fn copy_settings(
        &self,
        source: &Instance,
        target: &Instance,
        game_dir: &Path,
    ) -> CoreResult<()> {
        let source_dir = self.game_dir(&source.id)?;
        if let Some(bytes) = read_optional(&source_dir.join(OPTIONS_FILE)).await? {
            let options = match String::from_utf8(bytes) {
                Ok(text) => game_options::transfer_options(
                    &text,
                    &source.minecraft_version,
                    &target.minecraft_version,
                )
                .into_bytes(),
                Err(err) => {
                    tracing::warn!(instance = %source.id, "options.txt is not UTF-8, copying it unchanged");
                    err.into_bytes()
                }
            };
            storage::write_bytes_atomic(&game_dir.join(OPTIONS_FILE), &options).await?;
        }
        if let Some(servers) = read_optional(&source_dir.join(SERVERS_FILE)).await? {
            storage::write_bytes_atomic(&game_dir.join(SERVERS_FILE), &servers).await?;
        }
        Ok(())
    }

    pub async fn update(&self, id: &str, update: InstanceUpdate) -> CoreResult<Instance> {
        let name = validate_name(&update.name)?;
        validate_version_id(&update.minecraft_version)?;
        validate_loader(&update.loader)?;
        validate_java(&update.java)?;
        validate_resolution(update.resolution)?;
        let launch = normalize_launch(update.launch)?;

        let _guard = self.write_lock.lock().await;
        let mut instance = self.get(id).await?;
        instance.name = name;
        instance.minecraft_version = update.minecraft_version;
        instance.loader = update.loader;
        instance.java = update.java;
        instance.resolution = update.resolution;
        instance.launch = launch;
        self.write(&instance).await?;
        Ok(instance)
    }

    pub async fn set_group(&self, id: &str, group: Option<String>) -> CoreResult<Instance> {
        let group = normalize_group(group)?;
        let _guard = self.write_lock.lock().await;
        let mut instance = self.get(id).await?;
        instance.group = group;
        self.write(&instance).await?;
        Ok(instance)
    }

    pub async fn set_sync_packs(
        &self,
        id: &str,
        sync_packs: Vec<SyncSubscription>,
    ) -> CoreResult<Instance> {
        let _guard = self.write_lock.lock().await;
        let mut instance = self.get(id).await?;
        instance.sync_packs = sync_packs;
        self.write(&instance).await?;
        Ok(instance)
    }

    pub async fn set_pinned(&self, id: &str, pinned: bool) -> CoreResult<Instance> {
        let _guard = self.write_lock.lock().await;
        let mut instance = self.get(id).await?;
        instance.pinned = pinned;
        self.write(&instance).await?;
        Ok(instance)
    }

    pub async fn set_icon(&self, id: &str, image: Option<Vec<u8>>) -> CoreResult<Instance> {
        let new_icon = match &image {
            None => None,
            Some(bytes) if bytes.len() > MAX_ICON_BYTES => {
                return Err(CoreError::InvalidInput(
                    "the icon must be smaller than 1 MB".into(),
                ));
            }
            Some(bytes) => {
                let extension = icon_extension(bytes).ok_or_else(|| {
                    CoreError::InvalidInput(
                        "the icon must be a PNG, JPEG, GIF or WebP image".into(),
                    )
                })?;
                Some(format!("{ICON_PREFIX}{}.{extension}", unix_now_millis()))
            }
        };
        let _guard = self.write_lock.lock().await;
        let mut instance = self.get(id).await?;
        let dir = self.instance_dir(id)?;
        if let (Some(name), Some(bytes)) = (&new_icon, &image) {
            storage::write_bytes_atomic(&dir.join(name), bytes).await?;
        }
        let old = std::mem::replace(&mut instance.icon, new_icon);
        self.write(&instance).await?;
        if let Some(old) = old
            && validate_icon_name(&old).is_ok()
            && let Err(err) = tokio::fs::remove_file(dir.join(&old)).await
        {
            tracing::warn!(instance = %id, error = %err, "failed to remove the old icon");
        }
        Ok(instance)
    }

    pub async fn icon(&self, id: &str) -> CoreResult<Option<Vec<u8>>> {
        let instance = self.get(id).await?;
        let Some(name) = instance.icon else {
            return Ok(None);
        };
        validate_icon_name(&name)?;
        read_optional(&self.instance_dir(id)?.join(name)).await
    }

    pub async fn mark_played(&self, id: &str) -> CoreResult<()> {
        let _guard = self.write_lock.lock().await;
        let mut instance = self.get(id).await?;
        instance.last_played_at = Some(unix_now());
        self.write(&instance).await
    }

    pub async fn add_playtime(&self, id: &str, secs: u64) -> CoreResult<()> {
        let _guard = self.write_lock.lock().await;
        let mut instance = self.get(id).await?;
        instance.total_play_secs = instance.total_play_secs.saturating_add(secs);
        self.write(&instance).await
    }

    pub async fn delete(&self, id: &str) -> CoreResult<()> {
        let _guard = self.write_lock.lock().await;
        let dir = self.instance_dir(id)?;
        if tokio::fs::metadata(dir.join(INSTANCE_FILE)).await.is_err() {
            return Err(CoreError::NotFound(format!("instance {id}")));
        }
        tokio::fs::remove_dir_all(&dir).await.at_path(&dir)
    }

    pub async fn move_to_trash(&self, id: &str, trash: &Trash) -> CoreResult<TrashEntry> {
        let _guard = self.write_lock.lock().await;
        let instance = self.get(id).await?;
        let item = TrashItem {
            kind: TrashKind::Instance,
            name: instance.name,
            instance_id: id.to_string(),
            folder: None,
        };
        trash.put(item, &self.instance_dir(id)?).await
    }

    pub async fn restore_from_trash(&self, entry: &TrashEntry, trash: &Trash) -> CoreResult<()> {
        let _guard = self.write_lock.lock().await;
        let dir = self.instance_dir(&entry.instance_id)?;
        if tokio::fs::metadata(&dir).await.is_ok() {
            return Err(CoreError::InvalidInput(format!(
                "an instance with the id {} already exists",
                entry.instance_id
            )));
        }
        trash.take(&entry.id, &dir).await.map(|_| ())
    }

    pub async fn duplicate(&self, id: &str, name: &str) -> CoreResult<Instance> {
        let name = validate_name(name)?;
        let _guard = self.write_lock.lock().await;
        let source = self.get(id).await?;
        let new_id = self.unique_id(&slugify(&name)).await;
        let from = self.instance_dir(id)?;
        let to = self.instance_dir(&new_id)?;
        let copy_target = to.clone();
        tokio::task::spawn_blocking(move || copy_dir(&from, &copy_target))
            .await
            .map_err(|err| CoreError::Unsupported(format!("copy panicked: {err}")))??;

        let instance = Instance {
            id: new_id,
            name,
            created_at: unix_now(),
            last_played_at: None,
            total_play_secs: 0,
            ..source
        };
        if let Err(err) = self.write(&instance).await {
            let _ = tokio::fs::remove_dir_all(&to).await;
            return Err(err);
        }
        Ok(instance)
    }

    async fn write(&self, instance: &Instance) -> CoreResult<()> {
        let path = self.instance_dir(&instance.id)?.join(INSTANCE_FILE);
        storage::write_json_atomic(&path, instance).await
    }

    async fn unique_id(&self, base: &str) -> String {
        let mut candidate = base.to_string();
        let mut counter = 2;
        while tokio::fs::metadata(self.root.join(&candidate))
            .await
            .is_ok()
        {
            candidate = format!("{base}-{counter}");
            counter += 1;
        }
        candidate
    }
}

fn unix_now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default()
}

async fn read_optional(path: &Path) -> CoreResult<Option<Vec<u8>>> {
    match tokio::fs::read(path).await {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).at_path(path),
    }
}

fn copy_dir(from: &Path, to: &Path) -> CoreResult<()> {
    std::fs::create_dir_all(to).at_path(to)?;
    for entry in std::fs::read_dir(from).at_path(from)? {
        let entry = entry.at_path(from)?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        let file_type = entry.file_type().at_path(&source)?;
        if file_type.is_dir() {
            copy_dir(&source, &target)?;
        } else if file_type.is_file() {
            std::fs::copy(&source, &target).at_path(&target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_instance(name: &str) -> NewInstance {
        NewInstance {
            name: name.into(),
            minecraft_version: "1.20.1".into(),
            loader: LoaderSpec::Vanilla,
            copy_settings_from: None,
        }
    }

    #[tokio::test]
    async fn create_copies_settings_from_another_instance() {
        let dir = tempfile::tempdir().unwrap();
        let store = InstanceStore::new(dir.path().to_path_buf());
        let source = store.create(new_instance("Modern")).await.unwrap();
        let empty = store.create(new_instance("Leer")).await.unwrap();
        let source_dir = store.game_dir(&source.id).unwrap();
        std::fs::write(
            source_dir.join("options.txt"),
            "version:3465\nnarrator:0\nkey_key.jump:key.keyboard.space\n",
        )
        .unwrap();
        std::fs::write(source_dir.join("servers.dat"), [10, 0, 0, 0]).unwrap();

        let legacy = store
            .create(NewInstance {
                minecraft_version: "1.8.9".into(),
                copy_settings_from: Some(source.id.clone()),
                ..new_instance("PvP")
            })
            .await
            .unwrap();
        let legacy_dir = store.game_dir(&legacy.id).unwrap();
        assert_eq!(
            std::fs::read_to_string(legacy_dir.join("options.txt")).unwrap(),
            "narrator:0\nkey_key.jump:57\n"
        );
        assert_eq!(
            std::fs::read(legacy_dir.join("servers.dat")).unwrap(),
            [10, 0, 0, 0]
        );

        let from_empty = store
            .create(NewInstance {
                copy_settings_from: Some(empty.id.clone()),
                ..new_instance("Frisch")
            })
            .await
            .unwrap();
        let fresh_dir = store.game_dir(&from_empty.id).unwrap();
        assert!(fresh_dir.is_dir());
        assert!(!fresh_dir.join("options.txt").exists());
        assert!(!fresh_dir.join("servers.dat").exists());

        let missing = store
            .create(NewInstance {
                copy_settings_from: Some("gibt-es-nicht".into()),
                ..new_instance("Fehler")
            })
            .await;
        assert!(missing.is_err());
        assert!(!dir.path().join("fehler").exists());
        assert_eq!(store.list().await.unwrap().len(), 4);
    }

    #[test]
    fn slugify_handles_umlauts_and_symbols() {
        assert_eq!(slugify("Größte Welt!"), "groesste-welt");
        assert_eq!(slugify("  ---  "), "instanz");
        assert_eq!(slugify("PvP 1.8.9"), "pvp-1-8-9");
    }

    #[tokio::test]
    async fn create_list_update_duplicate_delete() {
        let dir = tempfile::tempdir().unwrap();
        let store = InstanceStore::new(dir.path().to_path_buf());

        let first = store.create(new_instance("Survival")).await.unwrap();
        let second = store.create(new_instance("Survival")).await.unwrap();
        assert_eq!(first.id, "survival");
        assert_eq!(second.id, "survival-2");
        assert!(store.game_dir(&first.id).unwrap().is_dir());

        std::fs::write(
            store.game_dir(&first.id).unwrap().join("options.txt"),
            "fov:90",
        )
        .unwrap();
        let updated = store
            .update(
                &first.id,
                InstanceUpdate {
                    name: "Survival Welt".into(),
                    minecraft_version: "1.21".into(),
                    loader: LoaderSpec::Fabric {
                        loader_version: "0.16.0".into(),
                    },
                    java: JavaSettings {
                        max_memory_mb: Some(6144),
                        min_memory_mb: Some(1024),
                        ..JavaSettings::default()
                    },
                    resolution: None,
                    launch: LaunchSettings::default(),
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.id, "survival");
        assert_eq!(store.get("survival").await.unwrap().name, "Survival Welt");

        let copy = store.duplicate("survival", "Kopie").await.unwrap();
        assert_eq!(copy.minecraft_version, "1.21");
        assert!(
            store
                .game_dir(&copy.id)
                .unwrap()
                .join("options.txt")
                .exists()
        );

        store.mark_played("survival-2").await.unwrap();
        store.add_playtime("survival-2", 90).await.unwrap();
        store.add_playtime("survival-2", 30).await.unwrap();
        assert_eq!(store.get("survival-2").await.unwrap().total_play_secs, 120);
        let listed = store.list().await.unwrap();
        assert_eq!(listed.len(), 3);
        assert_eq!(listed[0].id, "survival-2");

        store.delete("kopie").await.unwrap();
        assert_eq!(store.list().await.unwrap().len(), 2);
        assert!(store.delete("kopie").await.is_err());
    }

    #[test]
    fn launch_settings_are_trimmed_and_validated() {
        use crate::instances::EnvVar;
        let env = |key: &str| EnvVar {
            key: key.into(),
            value: "1".into(),
        };
        let normalized = normalize_launch(LaunchSettings {
            join_server: Some("  localhost:25566 ".into()),
            pre_launch: Some("   ".into()),
            wrapper: Some(" prime-run ".into()),
            env: vec![env("MESA_GL_VERSION_OVERRIDE"), env("_X1")],
            ..LaunchSettings::default()
        })
        .unwrap();
        assert_eq!(normalized.join_server.as_deref(), Some("localhost:25566"));
        assert_eq!(normalized.pre_launch, None);
        assert_eq!(normalized.wrapper.as_deref(), Some("prime-run"));

        for invalid in [
            LaunchSettings {
                join_server: Some("bad host".into()),
                ..LaunchSettings::default()
            },
            LaunchSettings {
                post_exit: Some("a\nb".into()),
                ..LaunchSettings::default()
            },
            LaunchSettings {
                wrapper: Some("\"unclosed".into()),
                ..LaunchSettings::default()
            },
            LaunchSettings {
                env: vec![env("1BAD")],
                ..LaunchSettings::default()
            },
            LaunchSettings {
                env: vec![env("A"), env("A")],
                ..LaunchSettings::default()
            },
            LaunchSettings {
                extra_game_args: vec![" ".into()],
                ..LaunchSettings::default()
            },
        ] {
            assert!(normalize_launch(invalid.clone()).is_err(), "{invalid:?}");
        }
    }

    #[tokio::test]
    async fn trashed_instances_come_back_unless_the_id_is_taken() {
        let dir = tempfile::tempdir().unwrap();
        let store = InstanceStore::new(dir.path().join("instances"));
        let trash = Trash::new(dir.path().join("trash"));
        let created = store.create(new_instance("Survival")).await.unwrap();
        std::fs::write(
            store.game_dir(&created.id).unwrap().join("options.txt"),
            "fov:90",
        )
        .unwrap();

        let entry = store.move_to_trash(&created.id, &trash).await.unwrap();
        assert_eq!(entry.name, "Survival");
        assert!(store.list().await.unwrap().is_empty());

        store.restore_from_trash(&entry, &trash).await.unwrap();
        let restored = store.get(&created.id).await.unwrap();
        assert_eq!(restored.name, "Survival");
        assert_eq!(
            std::fs::read_to_string(store.game_dir(&created.id).unwrap().join("options.txt"))
                .unwrap(),
            "fov:90"
        );

        let again = store.move_to_trash(&created.id, &trash).await.unwrap();
        store.create(new_instance("Survival")).await.unwrap();
        assert!(store.restore_from_trash(&again, &trash).await.is_err());
        assert_eq!(trash.list().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn groups_pins_and_icons_are_stored() {
        let dir = tempfile::tempdir().unwrap();
        let store = InstanceStore::new(dir.path().to_path_buf());
        let instance = store.create(new_instance("Survival")).await.unwrap();

        let grouped = store
            .set_group(&instance.id, Some("  PvP  ".into()))
            .await
            .unwrap();
        assert_eq!(grouped.group.as_deref(), Some("PvP"));
        assert!(
            store
                .set_group(&instance.id, Some("x".repeat(41)))
                .await
                .is_err()
        );
        assert_eq!(
            store
                .set_group(&instance.id, Some(" ".into()))
                .await
                .unwrap()
                .group,
            None
        );
        assert!(store.set_pinned(&instance.id, true).await.unwrap().pinned);

        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3];
        let first = store
            .set_icon(&instance.id, Some(png.to_vec()))
            .await
            .unwrap();
        let first_name = first.icon.clone().unwrap();
        assert!(first_name.starts_with("icon-") && first_name.ends_with(".png"));
        assert_eq!(store.icon(&instance.id).await.unwrap().unwrap(), png);

        let jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0];
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second = store
            .set_icon(&instance.id, Some(jpeg.clone()))
            .await
            .unwrap();
        assert!(second.icon.as_deref().unwrap().ends_with(".jpg"));
        assert!(
            !store
                .instance_dir(&instance.id)
                .unwrap()
                .join(&first_name)
                .exists()
        );

        assert!(
            store
                .set_icon(&instance.id, Some(b"<svg></svg>".to_vec()))
                .await
                .is_err()
        );
        assert!(
            store
                .set_icon(&instance.id, Some(vec![0x89; MAX_ICON_BYTES + 1]))
                .await
                .is_err()
        );

        let copy = store.duplicate(&instance.id, "Kopie").await.unwrap();
        assert!(copy.pinned);
        assert_eq!(store.icon(&copy.id).await.unwrap().unwrap(), jpeg);

        let cleared = store.set_icon(&instance.id, None).await.unwrap();
        assert_eq!(cleared.icon, None);
        assert_eq!(store.icon(&instance.id).await.unwrap(), None);
        assert!(validate_icon_name("../escape.png").is_err());
        assert!(validate_icon_name("icon-12.exe").is_err());
    }

    #[tokio::test]
    async fn rejects_invalid_ids_and_input() {
        let dir = tempfile::tempdir().unwrap();
        let store = InstanceStore::new(dir.path().to_path_buf());
        assert!(store.get("../escape").await.is_err());
        assert!(store.create(new_instance("   ")).await.is_err());
        let mut bad_version = new_instance("x");
        bad_version.minecraft_version = "../1.20".into();
        assert!(store.create(bad_version).await.is_err());
    }
}
