use std::path::{Path, PathBuf};

use serde_json::Value;

use super::ini::Ini;
use super::{
    ForeignInstance, ForeignLoader, ForeignSettings, ImportSource, Locations, display, folder_name,
    read_json, read_text,
};

const PRISM_CONFIGS: [&str; 3] = ["prismlauncher.cfg", "polymc.cfg", "multimc.cfg"];

fn text(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn number(value: &Value, key: &str) -> Option<u32> {
    let value = value.get(key)?;
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
        .and_then(|n| u32::try_from(n).ok())
        .filter(|n| *n > 0)
}

fn commands(values: &[Option<String>]) -> Vec<String> {
    values.iter().flatten().cloned().collect()
}

pub fn read_instance(path: &Path) -> Option<ForeignInstance> {
    if path.join("instance.cfg").is_file() {
        return read_prism(path);
    }
    if path.join("minecraftinstance.json").is_file() {
        return read_curseforge(path);
    }
    if path.join("instance.json").is_file() {
        let json = read_json(&path.join("instance.json"))?;
        if json.get("launcher").is_some() {
            return read_atlauncher(path, &json);
        }
        if json.get("game_configuration").is_some() {
            return read_carbon(path, &json);
        }
    }
    if path.join("config.json").is_file() {
        let json = read_json(&path.join("config.json"))?;
        if json
            .get("loader")
            .and_then(|loader| loader.get("mcVersion"))
            .is_some()
        {
            return read_gdlauncher(path, &json);
        }
    }
    None
}

fn prism_source(instance: &Path, ini: &Ini) -> ImportSource {
    if ini.get("ConfigVersion").is_some() {
        return ImportSource::Prism;
    }
    for ancestor in instance.ancestors().skip(1).take(3) {
        if ancestor.join("prismlauncher.cfg").is_file() || ancestor.join("polymc.cfg").is_file() {
            return ImportSource::Prism;
        }
        if ancestor.join("multimc.cfg").is_file() {
            return ImportSource::MultiMc;
        }
    }
    ImportSource::Prism
}

pub fn prism_game_dir(instance: &Path) -> PathBuf {
    let plain = instance.join("minecraft");
    let dotted = instance.join(".minecraft");
    if dotted.is_dir() && !plain.is_dir() {
        dotted
    } else {
        plain
    }
}

fn read_prism(path: &Path) -> Option<ForeignInstance> {
    let ini = Ini::parse(&read_text(&path.join("instance.cfg"))?);
    let mut minecraft_version = ini.get("IntendedVersion").map(str::to_string);
    let mut loader = ForeignLoader::Vanilla;
    if let Some(pack) = read_json(&path.join("mmc-pack.json")) {
        let components = pack
            .get("components")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let version = |uid: &str| {
            components
                .iter()
                .find(|c| c.get("uid").and_then(Value::as_str) == Some(uid))
                .and_then(|c| text(c, "version").or_else(|| text(c, "cachedVersion")))
        };
        minecraft_version = version("net.minecraft").or(minecraft_version);
        for (uid, kind) in [
            ("net.neoforged", "neoforge"),
            ("net.minecraftforge", "forge"),
            ("net.fabricmc.fabric-loader", "fabric"),
            ("org.quiltmc.quilt-loader", "quilt"),
            ("com.mumfrey.liteloader", "liteloader"),
        ] {
            if let Some(found) = version(uid) {
                loader = ForeignLoader::from_parts(kind, Some(&found));
                break;
            }
        }
    }
    let java = ini.flag("OverrideJavaLocation") || ini.flag("OverrideJava");
    let args = ini.flag("OverrideJavaArgs") || ini.flag("OverrideJava");
    let memory = ini.flag("OverrideMemory");
    let hooks = ini.flag("OverrideCommands") || ini.flag("OverrideLaunchCmd");
    let hook = |names: [&str; 2]| {
        names
            .iter()
            .find_map(|name| ini.get(name))
            .map(str::to_string)
    };
    let settings = ForeignSettings {
        java_path: java
            .then(|| ini.get("JavaPath").map(str::to_string))
            .flatten(),
        jvm_args: args
            .then(|| ini.get("JvmArgs").map(str::to_string))
            .flatten(),
        min_memory_mb: memory.then(|| ini.number("MinMemAlloc")).flatten(),
        max_memory_mb: memory.then(|| ini.number("MaxMemAlloc")).flatten(),
        commands: if hooks {
            commands(&[
                hook(["PreLaunchCommand", "PreLaunchCmd"]),
                hook(["WrapperCommand", "WrapperCmd"]),
                hook(["PostExitCommand", "PostExitCmd"]),
            ])
        } else {
            Vec::new()
        },
    };
    Some(ForeignInstance {
        source: prism_source(path, &ini),
        path: display(path),
        profile: None,
        name: ini
            .get("name")
            .map(str::to_string)
            .unwrap_or_else(|| folder_name(path)),
        game_dir: display(&prism_game_dir(path)),
        minecraft_version,
        latest: None,
        loader,
        settings,
    })
}

pub fn curseforge_loader(name: &str, game_version: Option<&str>) -> ForeignLoader {
    let Some((kind, rest)) = name.split_once('-') else {
        return ForeignLoader::from_parts(name, None);
    };
    let mut version = rest.to_string();
    if let Some(game) = game_version {
        if let Some(stripped) = version.strip_suffix(&format!("-{game}")) {
            version = stripped.to_string();
        }
        if let Some(stripped) = version.strip_prefix(&format!("{game}-")) {
            version = stripped.to_string();
        }
    }
    ForeignLoader::from_parts(kind, Some(&version))
}

fn read_curseforge(path: &Path) -> Option<ForeignInstance> {
    let json = read_json(&path.join("minecraftinstance.json"))?;
    let minecraft_version = text(&json, "gameVersion");
    let loader = json
        .get("baseModLoader")
        .and_then(|base| text(base, "name"))
        .map(|name| curseforge_loader(&name, minecraft_version.as_deref()))
        .unwrap_or(ForeignLoader::Vanilla);
    let memory = json
        .get("isMemoryOverride")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Some(ForeignInstance {
        source: ImportSource::CurseForge,
        path: display(path),
        profile: None,
        name: text(&json, "name").unwrap_or_else(|| folder_name(path)),
        game_dir: display(path),
        minecraft_version,
        latest: None,
        loader,
        settings: ForeignSettings {
            java_path: None,
            jvm_args: text(&json, "javaArgsOverride"),
            min_memory_mb: None,
            max_memory_mb: memory.then(|| number(&json, "allocatedMemory")).flatten(),
            commands: Vec::new(),
        },
    })
}

fn read_atlauncher(path: &Path, json: &Value) -> Option<ForeignInstance> {
    let launcher = json.get("launcher")?;
    let loader = match launcher.get("loaderVersion") {
        Some(found) if !found.is_null() => ForeignLoader::from_parts(
            &text(found, "type").unwrap_or_default(),
            text(found, "version").as_deref(),
        ),
        _ => ForeignLoader::Vanilla,
    };
    let hooks = launcher
        .get("enableCommands")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Some(ForeignInstance {
        source: ImportSource::AtLauncher,
        path: display(path),
        profile: None,
        name: text(launcher, "name").unwrap_or_else(|| folder_name(path)),
        game_dir: display(path),
        minecraft_version: text(json, "id"),
        latest: None,
        loader,
        settings: ForeignSettings {
            java_path: text(launcher, "javaPath"),
            jvm_args: text(launcher, "javaArguments"),
            min_memory_mb: None,
            max_memory_mb: number(launcher, "maximumMemory"),
            commands: if hooks {
                commands(&[
                    text(launcher, "preLaunchCommand"),
                    text(launcher, "wrapperCommand"),
                    text(launcher, "postExitCommand"),
                ])
            } else {
                Vec::new()
            },
        },
    })
}

fn read_gdlauncher(path: &Path, json: &Value) -> Option<ForeignInstance> {
    let loader_json = json.get("loader")?;
    let loader = ForeignLoader::from_parts(
        &text(loader_json, "loaderType").unwrap_or_default(),
        text(loader_json, "loaderVersion").as_deref(),
    );
    Some(ForeignInstance {
        source: ImportSource::GdLauncher,
        path: display(path),
        profile: None,
        name: folder_name(path),
        game_dir: display(path),
        minecraft_version: text(loader_json, "mcVersion"),
        latest: None,
        loader,
        settings: ForeignSettings {
            java_path: text(json, "customJavaPath"),
            jvm_args: text(json, "javaArgs"),
            min_memory_mb: None,
            max_memory_mb: number(json, "javaMemory"),
            commands: Vec::new(),
        },
    })
}

fn read_carbon(path: &Path, json: &Value) -> Option<ForeignInstance> {
    let game = json.get("game_configuration")?;
    let version = game.get("version");
    let release = version.and_then(|v| text(v, "release"));
    let loader = version
        .and_then(|v| v.get("modloaders"))
        .and_then(Value::as_array)
        .and_then(|loaders| loaders.first())
        .map(|first| {
            ForeignLoader::from_parts(
                &text(first, "type").unwrap_or_default(),
                text(first, "version").as_deref(),
            )
        })
        .unwrap_or(ForeignLoader::Vanilla);
    let memory = game.get("memory");
    Some(ForeignInstance {
        source: ImportSource::GdLauncher,
        path: display(path),
        profile: None,
        name: text(json, "name").unwrap_or_else(|| folder_name(path)),
        game_dir: display(&path.join("instance")),
        minecraft_version: release,
        latest: None,
        loader,
        settings: ForeignSettings {
            java_path: game.get("java_override").and_then(|j| text(j, "Path")),
            jvm_args: text(game, "extra_java_args"),
            min_memory_mb: memory.and_then(|m| number(m, "min_mb")),
            max_memory_mb: memory.and_then(|m| number(m, "max_mb")),
            commands: commands(&[
                text(json, "pre_launch_hook").or_else(|| text(game, "pre_launch_hook")),
                text(json, "wrapper_command").or_else(|| text(game, "wrapper_command")),
                text(json, "post_exit_hook").or_else(|| text(game, "post_exit_hook")),
            ]),
        },
    })
}

pub fn prism_roots(locations: &Locations) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(dir) = std::env::var_os("PRISMLAUNCHER_DATA_DIR") {
        roots.push(PathBuf::from(dir));
    }
    if let Some(data) = &locations.data {
        roots.push(data.join("PrismLauncher"));
        roots.push(data.join("multimc"));
    }
    if let Some(home) = &locations.home {
        roots.push(home.join(".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher"));
    }
    roots
}

pub fn prism_instance_dirs(root: &Path) -> Vec<PathBuf> {
    let configured = PRISM_CONFIGS.iter().find_map(|name| {
        let ini = Ini::parse(&read_text(&root.join(name))?);
        ini.get("InstanceDir").map(PathBuf::from)
    });
    let dir = match configured {
        Some(dir) if dir.is_absolute() => dir,
        Some(dir) => root.join(dir),
        None => root.join("instances"),
    };
    vec![dir]
}

pub fn instance_dirs(locations: &Locations) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = &locations.home {
        dirs.push(home.join("curseforge/minecraft/Instances"));
    }
    if let Some(documents) = &locations.documents {
        dirs.push(documents.join("curseforge/minecraft/Instances"));
    }
    if let Some(roaming) = &locations.roaming {
        dirs.push(roaming.join("ATLauncher/instances"));
        let legacy = roaming.join("gdlauncher_next");
        let legacy = read_text(&legacy.join("override.data"))
            .map(|text| PathBuf::from(text.trim()))
            .filter(|path| path.is_absolute())
            .unwrap_or(legacy);
        dirs.push(legacy.join("instances"));
    }
    if let Some(data) = &locations.data {
        dirs.push(data.join("atlauncher/instances"));
        let carbon = data.join("gdlauncher_carbon");
        let runtime = read_text(&carbon.join("runtime_path_override"))
            .map(|text| PathBuf::from(text.trim()))
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| carbon.join("data"));
        dirs.push(runtime.join("instances"));
    }
    dirs.sort();
    dirs.dedup();
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, relative: &str, text: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn reads_prism_instances() {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "prismlauncher.cfg",
            "[General]\nInstanceDir=instances\n",
        );
        let instance = root.path().join("instances/Fabric Pack");
        write(
            &instance,
            "instance.cfg",
            "[General]\nConfigVersion=1.3\nname=Fabric Pack\nOverrideMemory=true\nMaxMemAlloc=6144\nMinMemAlloc=1024\nOverrideJavaLocation=false\nJavaPath=/usr/bin/java\nOverrideCommands=true\nPreLaunchCommand=echo hi\n",
        );
        write(
            &instance,
            "mmc-pack.json",
            r#"{"formatVersion":1,"components":[{"uid":"org.lwjgl3","version":"3.3.3"},{"uid":"net.minecraft","version":"1.21.1"},{"uid":"net.fabricmc.intermediary","version":"1.21.1"},{"uid":"net.fabricmc.fabric-loader","version":"0.16.9"}]}"#,
        );
        write(&instance, ".minecraft/mods/sodium.jar", "jar");
        let found = read_instance(&instance).unwrap();
        assert_eq!(found.source, ImportSource::Prism);
        assert_eq!(found.minecraft_version.as_deref(), Some("1.21.1"));
        assert_eq!(
            found.loader,
            ForeignLoader::Fabric {
                version: "0.16.9".into()
            }
        );
        assert_eq!(found.settings.max_memory_mb, Some(6144));
        assert_eq!(found.settings.java_path, None);
        assert_eq!(found.settings.commands, vec!["echo hi".to_string()]);
        assert!(found.game_dir.ends_with(".minecraft"));
        assert_eq!(
            prism_instance_dirs(root.path()),
            vec![root.path().join("instances")]
        );
    }

    #[test]
    fn reads_multimc_legacy_instances() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "multimc.cfg", "InstanceDir=instances\n");
        let instance = root.path().join("instances/old");
        write(
            &instance,
            "instance.cfg",
            "name=Old\nIntendedVersion=1.8.9\nOverrideJava=true\nJavaPath=C:\\\\Java\\\\javaw.exe\n",
        );
        let found = read_instance(&instance).unwrap();
        assert_eq!(found.source, ImportSource::MultiMc);
        assert_eq!(found.minecraft_version.as_deref(), Some("1.8.9"));
        assert_eq!(
            found.settings.java_path.as_deref(),
            Some("C:\\Java\\javaw.exe")
        );
        assert!(found.game_dir.ends_with("minecraft"));
    }

    #[test]
    fn reads_curseforge_instances() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "minecraftinstance.json",
            r#"{"name":"All the Mods","gameVersion":"1.20.1","baseModLoader":{"name":"fabric-0.15.11-1.20.1","minecraftVersion":"1.20.1"},"isMemoryOverride":true,"allocatedMemory":8192,"javaArgsOverride":"-XX:+UseG1GC"}"#,
        );
        let found = read_instance(dir.path()).unwrap();
        assert_eq!(found.source, ImportSource::CurseForge);
        assert_eq!(
            found.loader,
            ForeignLoader::Fabric {
                version: "0.15.11".into()
            }
        );
        assert_eq!(found.settings.max_memory_mb, Some(8192));
        assert_eq!(found.settings.jvm_args.as_deref(), Some("-XX:+UseG1GC"));
        assert_eq!(
            curseforge_loader("forge-47.2.0", Some("1.20.1")),
            ForeignLoader::Forge {
                version: "47.2.0".into()
            }
        );
        assert_eq!(
            curseforge_loader("forge-1.20.1-47.2.0", Some("1.20.1")),
            ForeignLoader::Forge {
                version: "47.2.0".into()
            }
        );
    }

    #[test]
    fn reads_atlauncher_instances() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "instance.json",
            r#"{"id":"1.20.4","launcher":{"name":"Quilt World","loaderVersion":{"type":"Quilt","version":"0.26.0"},"javaPath":"C:\\Java","maximumMemory":4096,"enableCommands":true,"preLaunchCommand":"pre","postExitCommand":"post"}}"#,
        );
        let found = read_instance(dir.path()).unwrap();
        assert_eq!(found.source, ImportSource::AtLauncher);
        assert_eq!(found.name, "Quilt World");
        assert_eq!(found.minecraft_version.as_deref(), Some("1.20.4"));
        assert_eq!(
            found.loader,
            ForeignLoader::Quilt {
                version: "0.26.0".into()
            }
        );
        assert_eq!(
            found.settings.commands,
            vec!["pre".to_string(), "post".to_string()]
        );
    }

    #[test]
    fn reads_gdlauncher_instances() {
        let legacy = tempfile::tempdir().unwrap();
        write(
            legacy.path(),
            "config.json",
            r#"{"loader":{"loaderType":"forge","loaderVersion":"1.16.5-36.2.0","mcVersion":"1.16.5"},"javaMemory":3072}"#,
        );
        let found = read_instance(legacy.path()).unwrap();
        assert_eq!(found.source, ImportSource::GdLauncher);
        assert!(matches!(found.loader, ForeignLoader::Forge { .. }));
        assert_eq!(found.settings.max_memory_mb, Some(3072));

        let carbon = tempfile::tempdir().unwrap();
        write(
            carbon.path(),
            "instance.json",
            r#"{"_version":"1","name":"Carbon","icon":null,"game_configuration":{"version":{"release":"1.21.4","modloaders":[{"type":"Fabric","version":"0.16.10"}]},"memory":{"min_mb":1024,"max_mb":4096},"java_override":{"Path":"/opt/java"}}}"#,
        );
        let found = read_instance(carbon.path()).unwrap();
        assert_eq!(found.name, "Carbon");
        assert_eq!(
            found.loader,
            ForeignLoader::Fabric {
                version: "0.16.10".into()
            }
        );
        assert_eq!(found.settings.java_path.as_deref(), Some("/opt/java"));
        assert!(found.game_dir.ends_with("instance"));
    }

    #[test]
    fn ignores_unrelated_folders() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "config.json", r#"{"theme":"dark"}"#);
        assert!(read_instance(dir.path()).is_none());
    }
}
