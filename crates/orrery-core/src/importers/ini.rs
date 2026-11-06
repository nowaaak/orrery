use std::collections::HashMap;

pub struct Ini {
    values: HashMap<String, String>,
}

fn unescape(raw: &str) -> String {
    let raw = raw.trim();
    let quoted = raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"');
    let body = if quoted { &raw[1..raw.len() - 1] } else { raw };
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

impl Ini {
    pub fn parse(text: &str) -> Self {
        let mut values = HashMap::new();
        let mut section = String::new();
        for line in text.lines() {
            let line = line.trim_start_matches('\u{feff}').trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                let name = &line[1..line.len() - 1];
                section = if name == "General" {
                    String::new()
                } else {
                    name.trim_start_matches('%').to_string()
                };
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let key = if section.is_empty() {
                key.to_string()
            } else {
                format!("{section}/{key}")
            };
            values.insert(key, unescape(value));
        }
        Self { values }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values
            .get(key)
            .map(String::as_str)
            .filter(|value| !value.is_empty() && !value.starts_with("@Invalid"))
    }

    pub fn flag(&self, key: &str) -> bool {
        self.get(key)
            .is_some_and(|value| value.eq_ignore_ascii_case("true"))
    }

    pub fn number(&self, key: &str) -> Option<u32> {
        self.get(key)?.trim().parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_qt_and_legacy_files() {
        let qt = Ini::parse(
            "[General]\nConfigVersion=1.3\nname=\"Fabric \\\"Pack\\\"\"\nJavaPath=C:\\\\Program Files\\\\Java\\\\bin\\\\javaw.exe\nOverrideJavaLocation=true\nMaxMemAlloc=4096\n[Other]\nkey=value\n",
        );
        assert_eq!(qt.get("name"), Some("Fabric \"Pack\""));
        assert_eq!(
            qt.get("JavaPath"),
            Some("C:\\Program Files\\Java\\bin\\javaw.exe")
        );
        assert!(qt.flag("OverrideJavaLocation"));
        assert_eq!(qt.number("MaxMemAlloc"), Some(4096));
        assert_eq!(qt.get("Other/key"), Some("value"));
        assert_eq!(qt.get("missing"), None);

        let legacy = Ini::parse(
            "# comment\nname=Old Pack\nnotes=line one\\nline two\nIntendedVersion=1.7.10\n",
        );
        assert_eq!(legacy.get("name"), Some("Old Pack"));
        assert_eq!(legacy.get("notes"), Some("line one\nline two"));
        assert!(!legacy.flag("OverrideMemory"));
    }
}
