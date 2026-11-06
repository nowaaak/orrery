use std::path::{Component, Path, PathBuf};

use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MavenCoordinate {
    pub group: String,
    pub artifact: String,
    pub version: String,
    pub classifier: Option<String>,
    pub extension: String,
}

impl MavenCoordinate {
    pub fn parse(name: &str) -> CoreResult<Self> {
        let (coords, extension) = match name.split_once('@') {
            Some((coords, ext)) => (coords, ext.to_string()),
            None => (name, "jar".to_string()),
        };
        let parts: Vec<&str> = coords.split(':').collect();
        let (group, artifact, version, classifier) = match parts.as_slice() {
            [g, a, v] => (*g, *a, *v, None),
            [g, a, v, c] => (*g, *a, *v, Some((*c).to_string())),
            _ => {
                return Err(CoreError::InvalidInput(format!(
                    "invalid maven coordinate {name:?}"
                )));
            }
        };
        if [group, artifact, version].iter().any(|p| p.is_empty()) {
            return Err(CoreError::InvalidInput(format!(
                "invalid maven coordinate {name:?}"
            )));
        }
        Ok(Self {
            group: group.to_string(),
            artifact: artifact.to_string(),
            version: version.to_string(),
            classifier,
            extension,
        })
    }

    pub fn with_classifier(&self, classifier: &str) -> Self {
        Self {
            classifier: Some(classifier.to_string()),
            ..self.clone()
        }
    }

    pub fn relative_path(&self) -> String {
        let file = match &self.classifier {
            Some(classifier) => format!(
                "{}-{}-{classifier}.{}",
                self.artifact, self.version, self.extension
            ),
            None => format!("{}-{}.{}", self.artifact, self.version, self.extension),
        };
        format!(
            "{}/{}/{}/{file}",
            self.group.replace('.', "/"),
            self.artifact,
            self.version
        )
    }
}

pub fn safe_join(base: &Path, relative: &str) -> CoreResult<PathBuf> {
    let relative_path = Path::new(relative);
    let safe = relative_path
        .components()
        .all(|component| matches!(component, Component::Normal(_)));
    if !safe || relative.is_empty() {
        return Err(CoreError::InvalidInput(format!(
            "unsafe relative path {relative:?}"
        )));
    }
    Ok(base.join(relative_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_coordinates_with_classifier_and_extension() {
        let plain = MavenCoordinate::parse("com.mojang:brigadier:1.1.8").unwrap();
        assert_eq!(
            plain.relative_path(),
            "com/mojang/brigadier/1.1.8/brigadier-1.1.8.jar"
        );

        let native = MavenCoordinate::parse("org.lwjgl:lwjgl:3.3.1:natives-windows").unwrap();
        assert_eq!(
            native.relative_path(),
            "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar"
        );

        let zip = MavenCoordinate::parse("net.example:data:2@zip").unwrap();
        assert_eq!(zip.relative_path(), "net/example/data/2/data-2.zip");

        assert!(MavenCoordinate::parse("broken").is_err());
    }

    #[test]
    fn safe_join_rejects_traversal() {
        let base = Path::new("libs");
        assert!(safe_join(base, "a/b/c.jar").is_ok());
        assert!(safe_join(base, "../c.jar").is_err());
        assert!(safe_join(base, "/etc/passwd").is_err());
        assert!(safe_join(base, "").is_err());
    }
}
