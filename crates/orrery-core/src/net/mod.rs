mod client;
mod download;

pub use client::{Http, ensure_success, http_error, with_retry};
pub use download::{Checksum, Download, VerifyMode, download_all, file_matches, sha1_hex};
