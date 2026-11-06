use orrery_core::CoreError;
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct CommandError {
    pub kind: String,
    pub message: String,
}

impl From<CoreError> for CommandError {
    fn from(err: CoreError) -> Self {
        Self {
            kind: err.kind().into(),
            message: err.to_string(),
        }
    }
}

pub type CommandResult<T> = Result<T, CommandError>;
