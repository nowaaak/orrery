use std::path::Path;

use crate::error::IoResultExt;
use crate::{CoreError, CoreResult};

const MIX: u32 = 0x5bd1_e995;
const SEED: u32 = 1;

pub fn murmur2(data: &[u8], seed: u32) -> u32 {
    let mut hash = seed ^ data.len() as u32;
    let (chunks, rest) = data.as_chunks::<4>();
    for chunk in chunks {
        let mut k = u32::from_le_bytes(*chunk);
        k = k.wrapping_mul(MIX);
        k ^= k >> 24;
        k = k.wrapping_mul(MIX);
        hash = hash.wrapping_mul(MIX);
        hash ^= k;
    }
    if rest.len() >= 3 {
        hash ^= u32::from(rest[2]) << 16;
    }
    if rest.len() >= 2 {
        hash ^= u32::from(rest[1]) << 8;
    }
    if let Some(first) = rest.first() {
        hash ^= u32::from(*first);
        hash = hash.wrapping_mul(MIX);
    }
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(MIX);
    hash ^= hash >> 15;
    hash
}

fn is_ignored(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | b'\r' | b' ')
}

pub fn fingerprint(data: &[u8]) -> u32 {
    let kept: Vec<u8> = data.iter().copied().filter(|b| !is_ignored(*b)).collect();
    murmur2(&kept, SEED)
}

pub async fn file_fingerprint(path: &Path) -> CoreResult<u32> {
    let data = tokio::fs::read(path).await.at_path(path)?;
    tokio::task::spawn_blocking(move || fingerprint(&data))
        .await
        .map_err(|err| CoreError::Unsupported(format!("fingerprint panicked: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_the_smhasher_verification() {
        let key: Vec<u8> = (0..=255u8).collect();
        let mut hashes = Vec::with_capacity(256 * 4);
        for length in 0..256usize {
            let seed = 256 - length as u32;
            hashes.extend_from_slice(&murmur2(&key[..length], seed).to_le_bytes());
        }
        assert_eq!(murmur2(&hashes, 0), 0x2786_4C1E);
    }

    #[test]
    fn whitespace_does_not_change_the_fingerprint() {
        assert_eq!(fingerprint(b"a b\r\n\tc"), murmur2(b"abc", SEED));
        assert_eq!(fingerprint(b"abc"), fingerprint(b" abc \n"));
        assert_ne!(fingerprint(b"abc"), fingerprint(b"abd"));
    }

    #[tokio::test]
    async fn fingerprints_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mod.jar");
        std::fs::write(&path, b"some jar\nbytes").unwrap();
        assert_eq!(
            file_fingerprint(&path).await.unwrap(),
            fingerprint(b"some jar\nbytes")
        );
    }
}
