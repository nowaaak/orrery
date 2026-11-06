use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use super::{
    ForeignInstance, ForeignLoader, ForeignSettings, ImportSource, Locations, display, folder_name,
    read_json,
};

const BUSY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

struct Row {
    path: String,
    name: String,
    game_version: Option<String>,
    loader: Option<String>,
    loader_version: Option<String>,
    java_path: Option<String>,
    jvm_args: Option<String>,
    max_memory_mb: Option<u32>,
    commands: Vec<String>,
}

fn collect_rows<T>(rows: impl Iterator<Item = rusqlite::Result<T>>) -> Vec<T> {
    rows.filter_map(|row| {
        row.map_err(
            |err| tracing::warn!(error = %err, "skipping an unreadable Modrinth App instance"),
        )
        .ok()
    })
    .collect()
}

fn open(database: &Path) -> Option<Connection> {
    let connection = Connection::open_with_flags(
        database,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()?;
    connection.busy_timeout(BUSY_TIMEOUT).ok()?;
    Some(connection)
}

fn has_table(connection: &Connection, table: &str) -> bool {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |_| Ok(()),
        )
        .is_ok()
}

fn columns(connection: &Connection, table: &str) -> Vec<String> {
    let Ok(mut statement) = connection.prepare(&format!("PRAGMA table_info({table})")) else {
        return Vec::new();
    };
    statement
        .query_map([], |row| row.get::<_, String>(1))
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
}

fn args_text(value: &Value) -> Option<String> {
    match value {
        Value::Array(items) => {
            let joined = items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" ");
            (!joined.is_empty()).then_some(joined)
        }
        Value::String(text) => serde_json::from_str::<Value>(text)
            .ok()
            .and_then(|parsed| args_text(&parsed))
            .or_else(|| (!text.trim().is_empty()).then(|| text.clone())),
        _ => None,
    }
}

fn hooks(value: Option<&Value>) -> Vec<String> {
    let Some(hooks) = value else {
        return Vec::new();
    };
    ["pre_launch", "wrapper", "post_exit"]
        .iter()
        .filter_map(|key| hooks.get(*key).and_then(Value::as_str))
        .filter(|text| !text.trim().is_empty())
        .map(str::to_string)
        .collect()
}

fn memory(value: Option<&Value>) -> Option<u32> {
    value
        .and_then(|memory| memory.get("maximum"))
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .filter(|n| *n > 0)
}

fn current_rows(connection: &Connection) -> Vec<Row> {
    let launch = has_table(connection, "instance_launch_overrides");
    let sql = format!(
        "SELECT i.path, i.name, c.game_version, c.loader, c.loader_version, {} \
         FROM instances i LEFT JOIN instance_content_sets c ON c.id = i.applied_content_set_id {}",
        if launch { "json(o.overrides)" } else { "NULL" },
        if launch {
            "LEFT JOIN instance_launch_overrides o ON o.instance_id = i.id"
        } else {
            ""
        },
    );
    let Ok(mut statement) = connection.prepare(&sql) else {
        return Vec::new();
    };
    statement
        .query_map([], |row| {
            let overrides: Option<String> = row.get(5)?;
            let overrides: Value = overrides
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or(Value::Null);
            Ok(Row {
                path: row.get(0)?,
                name: row.get(1)?,
                game_version: row.get(2)?,
                loader: row.get(3)?,
                loader_version: row.get(4)?,
                java_path: overrides
                    .get("java_path")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                jvm_args: overrides.get("extra_launch_args").and_then(args_text),
                max_memory_mb: memory(overrides.get("memory")),
                commands: hooks(overrides.get("hooks")),
            })
        })
        .map(|rows| collect_rows(rows))
        .unwrap_or_default()
}

fn legacy_rows(connection: &Connection) -> Vec<Row> {
    let available = columns(connection, "profiles");
    let column = |name: &str| {
        if available.iter().any(|c| c == name) {
            name.to_string()
        } else {
            "NULL".to_string()
        }
    };
    let sql = format!(
        "SELECT path, name, {}, {}, {}, {}, {}, {}, {}, {}, {} FROM profiles",
        column("game_version"),
        column("mod_loader"),
        column("mod_loader_version"),
        column("override_java_path"),
        column("override_extra_launch_args"),
        column("override_mc_memory_max"),
        column("override_hook_pre_launch"),
        column("override_hook_wrapper"),
        column("override_hook_post_exit"),
    );
    let Ok(mut statement) = connection.prepare(&sql) else {
        return Vec::new();
    };
    statement
        .query_map([], |row| {
            let args: Option<String> = row.get(6)?;
            let memory: Option<i64> = row.get(7)?;
            let hooks: Vec<String> = [row.get::<_, Option<String>>(8)?, row.get(9)?, row.get(10)?]
                .into_iter()
                .flatten()
                .filter(|text| !text.trim().is_empty())
                .collect();
            Ok(Row {
                path: row.get(0)?,
                name: row.get(1)?,
                game_version: row.get(2)?,
                loader: row.get(3)?,
                loader_version: row.get(4)?,
                java_path: row.get(5)?,
                jvm_args: args.and_then(|text| args_text(&Value::String(text))),
                max_memory_mb: memory
                    .and_then(|n| u32::try_from(n).ok())
                    .filter(|n| *n > 0),
                commands: hooks,
            })
        })
        .map(|rows| collect_rows(rows))
        .unwrap_or_default()
}

fn profiles_dir(connection: &Connection, config_dir: &Path) -> PathBuf {
    let custom = if columns(connection, "settings")
        .iter()
        .any(|c| c == "custom_dir")
    {
        connection
            .query_row("SELECT custom_dir FROM settings LIMIT 1", [], |row| {
                row.get::<_, Option<String>>(0)
            })
            .ok()
            .flatten()
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    } else {
        None
    };
    custom
        .unwrap_or_else(|| config_dir.to_path_buf())
        .join("profiles")
}

fn to_instance(row: Row, profiles: &Path) -> ForeignInstance {
    let folder = profiles.join(&row.path);
    ForeignInstance {
        source: ImportSource::ModrinthApp,
        path: display(&folder),
        profile: None,
        name: if row.name.trim().is_empty() {
            folder_name(&folder)
        } else {
            row.name
        },
        game_dir: display(&folder),
        minecraft_version: row.game_version.filter(|v| !v.is_empty()),
        latest: None,
        loader: ForeignLoader::from_parts(
            row.loader.as_deref().unwrap_or("vanilla"),
            row.loader_version.as_deref(),
        ),
        settings: ForeignSettings {
            java_path: row.java_path.filter(|p| !p.trim().is_empty()),
            jvm_args: row.jvm_args,
            min_memory_mb: None,
            max_memory_mb: row.max_memory_mb,
            commands: row.commands,
        },
    }
}

pub fn read_database(config_dir: &Path) -> Vec<ForeignInstance> {
    let Some(connection) = open(&config_dir.join("app.db")) else {
        return Vec::new();
    };
    let rows =
        if has_table(&connection, "instances") && has_table(&connection, "instance_content_sets") {
            current_rows(&connection)
        } else if has_table(&connection, "profiles") {
            legacy_rows(&connection)
        } else {
            Vec::new()
        };
    let profiles = profiles_dir(&connection, config_dir);
    rows.into_iter()
        .filter(|row| !row.path.contains(['/', '\\']) && row.path != ".." && !row.path.is_empty())
        .map(|row| to_instance(row, &profiles))
        .filter(|instance| Path::new(&instance.path).is_dir())
        .collect()
}

fn read_theseus(folder: &Path) -> Option<ForeignInstance> {
    let json = read_json(&folder.join("profile.json"))?;
    let metadata = json.get("metadata")?;
    let text =
        |value: &Value, key: &str| value.get(key).and_then(Value::as_str).map(str::to_string);
    let loader_version = metadata
        .get("loader_version")
        .and_then(|v| v.get("id").and_then(Value::as_str).or_else(|| v.as_str()))
        .map(str::to_string);
    let java = json.get("java");
    Some(ForeignInstance {
        source: ImportSource::ModrinthApp,
        path: display(folder),
        profile: None,
        name: text(metadata, "name").unwrap_or_else(|| folder_name(folder)),
        game_dir: display(folder),
        minecraft_version: text(metadata, "game_version"),
        latest: None,
        loader: ForeignLoader::from_parts(
            &text(metadata, "loader").unwrap_or_else(|| "vanilla".into()),
            loader_version.as_deref(),
        ),
        settings: ForeignSettings {
            java_path: java
                .and_then(|j| j.get("override_version"))
                .and_then(|v| text(v, "path")),
            jvm_args: java
                .and_then(|j| j.get("extra_arguments"))
                .and_then(args_text),
            min_memory_mb: None,
            max_memory_mb: memory(json.get("memory")),
            commands: hooks(json.get("hooks")),
        },
    })
}

pub fn read_profile_folder(folder: &Path, locations: &Locations) -> Option<ForeignInstance> {
    if let Some(found) = read_theseus(folder) {
        return Some(found);
    }
    let candidates = folder
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .into_iter()
        .chain(locations.data.as_ref().map(|data| data.join("ModrinthApp")));
    for config_dir in candidates {
        if let Some(found) = read_database(&config_dir)
            .into_iter()
            .find(|instance| Path::new(&instance.path) == folder)
        {
            return Some(found);
        }
    }
    None
}

pub fn scan(locations: &Locations) -> Vec<ForeignInstance> {
    let mut found = Vec::new();
    if let Some(dir) = std::env::var_os("THESEUS_CONFIG_DIR") {
        found.extend(read_database(Path::new(&dir)));
    }
    if let Some(data) = &locations.data {
        found.extend(read_database(&data.join("ModrinthApp")));
    }
    if let Some(config) = &locations.config {
        let profiles = config.join("com.modrinth.theseus/profiles");
        if let Ok(entries) = std::fs::read_dir(&profiles) {
            found.extend(
                entries
                    .flatten()
                    .filter_map(|entry| read_theseus(&entry.path())),
            );
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database(dir: &Path, schema: &str) {
        let connection = Connection::open(dir.join("app.db")).unwrap();
        connection.execute_batch(schema).unwrap();
    }

    #[test]
    fn reads_the_current_schema() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("profiles/Fabulously Optimized/mods")).unwrap();
        database(
            dir.path(),
            r#"
            CREATE TABLE settings (id INTEGER PRIMARY KEY, custom_dir TEXT);
            INSERT INTO settings VALUES (0, NULL);
            CREATE TABLE instances (id TEXT PRIMARY KEY, path TEXT, name TEXT, icon_path TEXT, applied_content_set_id TEXT);
            CREATE TABLE instance_content_sets (id TEXT PRIMARY KEY, game_version TEXT, loader TEXT, loader_version TEXT);
            CREATE TABLE instance_launch_overrides (instance_id TEXT, overrides BLOB);
            INSERT INTO instances VALUES ('a', 'Fabulously Optimized', 'Fabulously Optimized', NULL, 'c1');
            INSERT INTO instances VALUES ('b', '../escape', 'Evil', NULL, 'c1');
            INSERT INTO instance_content_sets VALUES ('c1', '1.21.4', 'fabric', '0.16.10');
            INSERT INTO instance_launch_overrides VALUES ('a', jsonb('{"java_path":"/opt/java","extra_launch_args":["-Xss2M"],"memory":{"maximum":6144},"hooks":{"pre_launch":"echo pre","wrapper":null,"post_exit":""}}'));
            "#,
        );
        let found = read_database(dir.path());
        assert_eq!(found.len(), 1);
        let instance = &found[0];
        assert_eq!(instance.minecraft_version.as_deref(), Some("1.21.4"));
        assert_eq!(
            instance.loader,
            ForeignLoader::Fabric {
                version: "0.16.10".into()
            }
        );
        assert_eq!(instance.settings.java_path.as_deref(), Some("/opt/java"));
        assert_eq!(instance.settings.jvm_args.as_deref(), Some("-Xss2M"));
        assert_eq!(instance.settings.max_memory_mb, Some(6144));
        assert_eq!(instance.settings.commands, vec!["echo pre".to_string()]);
        let folder = dir.path().join("profiles/Fabulously Optimized");
        assert_eq!(
            read_profile_folder(&folder, &Locations::default())
                .unwrap()
                .name,
            "Fabulously Optimized"
        );
    }

    #[test]
    fn reads_the_legacy_schema_and_theseus() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("profiles/Old")).unwrap();
        database(
            dir.path(),
            r#"
            CREATE TABLE profiles (path TEXT PRIMARY KEY, name TEXT, icon_path TEXT, game_version TEXT, mod_loader TEXT, mod_loader_version TEXT, override_extra_launch_args TEXT, override_mc_memory_max INTEGER);
            INSERT INTO profiles VALUES ('Old', 'Old Profile', NULL, '1.20.1', 'quilt', '0.20.0', '["-Dfoo=bar"]', 2048);
            "#,
        );
        let found = read_database(dir.path());
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].loader,
            ForeignLoader::Quilt {
                version: "0.20.0".into()
            }
        );
        assert_eq!(found[0].settings.jvm_args.as_deref(), Some("-Dfoo=bar"));
        assert_eq!(found[0].settings.max_memory_mb, Some(2048));

        let theseus = tempfile::tempdir().unwrap();
        std::fs::write(
            theseus.path().join("profile.json"),
            r#"{"path":"x","metadata":{"name":"Theseus","game_version":"1.19.4","loader":"fabric","loader_version":{"id":"0.14.21"}},"memory":{"maximum":3000},"hooks":{"post_exit":"bye"}}"#,
        )
        .unwrap();
        let old = read_profile_folder(theseus.path(), &Locations::default()).unwrap();
        assert_eq!(old.name, "Theseus");
        assert_eq!(
            old.loader,
            ForeignLoader::Fabric {
                version: "0.14.21".into()
            }
        );
        assert_eq!(old.settings.commands, vec!["bye".to_string()]);
    }
}
