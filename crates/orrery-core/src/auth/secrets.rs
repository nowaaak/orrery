use crate::{CoreError, CoreResult};

const CHUNK_CHARS: usize = 1000;
const MAX_CHUNKS: usize = 16;

pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> CoreResult<Option<String>>;
    fn set(&self, key: &str, value: &str) -> CoreResult<()>;
    fn delete(&self, key: &str) -> CoreResult<()>;
}

pub struct KeyringStore {
    service: String,
}

impl KeyringStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self, user: &str) -> CoreResult<keyring::Entry> {
        keyring::Entry::new(&self.service, user).map_err(secret_error)
    }

    fn read(&self, user: &str) -> CoreResult<Option<String>> {
        match self.entry(user)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(secret_error(err)),
        }
    }

    fn remove(&self, user: &str) -> CoreResult<()> {
        match self.entry(user)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(secret_error(err)),
        }
    }
}

fn secret_error(err: keyring::Error) -> CoreError {
    match err {
        keyring::Error::NoDefaultStore => CoreError::Secret(
            "no system keyring is available (on Linux a Secret Service such as GNOME Keyring or KWallet is required)"
                .into(),
        ),
        other => CoreError::Secret(other.to_string()),
    }
}

fn chunk_key(key: &str, index: usize) -> String {
    format!("{key}#{index}")
}

fn parse_header(header: &str) -> Option<usize> {
    header
        .strip_prefix("chunks:")
        .and_then(|count| count.parse().ok())
        .filter(|count| *count <= MAX_CHUNKS)
}

impl SecretStore for KeyringStore {
    fn get(&self, key: &str) -> CoreResult<Option<String>> {
        let Some(header) = self.read(key)? else {
            return Ok(None);
        };
        let count = parse_header(&header)
            .ok_or_else(|| CoreError::Secret(format!("corrupt keyring entry {key}")))?;
        let mut value = String::new();
        for index in 0..count {
            let part = self
                .read(&chunk_key(key, index))?
                .ok_or_else(|| CoreError::Secret(format!("incomplete keyring entry {key}")))?;
            value.push_str(&part);
        }
        Ok(Some(value))
    }

    fn set(&self, key: &str, value: &str) -> CoreResult<()> {
        let chars: Vec<char> = value.chars().collect();
        let chunks: Vec<String> = chars
            .chunks(CHUNK_CHARS)
            .map(|chunk| chunk.iter().collect())
            .collect();
        if chunks.len() > MAX_CHUNKS {
            return Err(CoreError::Secret("secret is too large".into()));
        }
        let previous = self.read(key)?.and_then(|h| parse_header(&h)).unwrap_or(0);
        for (index, chunk) in chunks.iter().enumerate() {
            self.entry(&chunk_key(key, index))?
                .set_password(chunk)
                .map_err(secret_error)?;
        }
        self.entry(key)?
            .set_password(&format!("chunks:{}", chunks.len()))
            .map_err(secret_error)?;
        for index in chunks.len()..previous {
            self.remove(&chunk_key(key, index))?;
        }
        Ok(())
    }

    fn delete(&self, key: &str) -> CoreResult<()> {
        let count = self.read(key)?.and_then(|h| parse_header(&h)).unwrap_or(0);
        self.remove(key)?;
        for index in 0..count {
            self.remove(&chunk_key(key, index))?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct MemorySecrets {
    values: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

#[cfg(test)]
impl SecretStore for MemorySecrets {
    fn get(&self, key: &str) -> CoreResult<Option<String>> {
        Ok(self.values.lock().unwrap().get(key).cloned())
    }

    fn set(&self, key: &str, value: &str) -> CoreResult<()> {
        self.values
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn delete(&self, key: &str) -> CoreResult<()> {
        self.values.lock().unwrap().remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_are_parsed_strictly() {
        assert_eq!(parse_header("chunks:3"), Some(3));
        assert_eq!(parse_header("chunks:99"), None);
        assert_eq!(parse_header("garbage"), None);
    }

    #[test]
    #[ignore = "writes to the real system keyring"]
    fn keyring_roundtrip_with_long_secret() {
        let store = KeyringStore::new("io.github.nowaaak.orrery.test");
        let secret: String = "M.C5_BAY.0.U.".chars().cycle().take(4500).collect();
        store.set("roundtrip", &secret).unwrap();
        assert_eq!(
            store.get("roundtrip").unwrap().as_deref(),
            Some(secret.as_str())
        );
        store.set("roundtrip", "short").unwrap();
        assert_eq!(store.get("roundtrip").unwrap().as_deref(), Some("short"));
        store.delete("roundtrip").unwrap();
        assert_eq!(store.get("roundtrip").unwrap(), None);
    }
}
