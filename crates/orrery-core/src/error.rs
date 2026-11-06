use std::path::{Path, PathBuf};

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("file operation failed at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid JSON in {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("{path} uses schema version {found}, this launcher supports up to {supported}")]
    UnsupportedSchema {
        path: PathBuf,
        found: u32,
        supported: u32,
    },

    #[error("request to {url} failed: {source}")]
    Http {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("request to {url} returned HTTP {status}")]
    HttpStatus { url: String, status: u16 },

    #[error("checksum mismatch for {url}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        url: String,
        expected: String,
        actual: String,
    },

    #[error("invalid archive {path}: {message}")]
    Archive { path: PathBuf, message: String },

    #[error("not supported: {0}")]
    Unsupported(String),

    #[error("{message}")]
    Auth {
        reason: crate::auth::AuthFailure,
        message: String,
    },

    #[error("system keyring error: {0}")]
    Secret(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("{stage} command failed with {}: {output}", exit_label(*.code))]
    LaunchHook {
        stage: &'static str,
        code: Option<i32>,
        output: String,
    },

    #[error("{0}")]
    Unreachable(String),

    #[error("operation was cancelled")]
    Cancelled,
}

impl CoreError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Io { .. } => "io",
            Self::Json { .. } => "json",
            Self::UnsupportedSchema { .. } => "unsupported_schema",
            Self::Http { .. } | Self::HttpStatus { .. } => "network",
            Self::ChecksumMismatch { .. } => "checksum",
            Self::Archive { .. } => "archive",
            Self::Unsupported(_) => "unsupported",
            Self::Auth { reason, .. } => reason.code(),
            Self::Secret(_) => "secret_store",
            Self::InvalidInput(_) => "invalid_input",
            Self::NotFound(_) => "not_found",
            Self::LaunchHook { .. } => "launch_hook",
            Self::Unreachable(_) => "unreachable",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Http { source, .. } => {
                source.is_timeout()
                    || source.is_connect()
                    || source.is_request()
                    || source.is_body()
            }
            Self::HttpStatus { status, .. } => *status == 429 || *status >= 500,
            Self::ChecksumMismatch { .. } => true,
            _ => false,
        }
    }
}

fn exit_label(code: Option<i32>) -> String {
    match code {
        Some(code) => format!("exit code {code}"),
        None => "no exit code".into(),
    }
}

pub trait IoResultExt<T> {
    fn at_path(self, path: &Path) -> CoreResult<T>;
}

impl<T> IoResultExt<T> for std::io::Result<T> {
    fn at_path(self, path: &Path) -> CoreResult<T> {
        self.map_err(|source| CoreError::Io {
            path: path.to_path_buf(),
            source,
        })
    }
}
