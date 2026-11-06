use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;

use crate::storage::Versioned;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum AccountKind {
    Microsoft,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum Skin {
    Remote { url: String, slim: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Account {
    pub id: String,
    pub kind: AccountKind,
    pub uuid: String,
    pub username: String,
    #[serde(default)]
    pub skin: Option<Skin>,
    pub added_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountsFile {
    pub schema: u32,
    #[serde(default)]
    pub active_id: Option<String>,
    #[serde(default, deserialize_with = "without_offline_accounts")]
    pub accounts: Vec<Account>,
}

fn without_offline_accounts<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Account>, D::Error> {
    let entries = Vec::<serde_json::Value>::deserialize(deserializer)?;
    entries
        .into_iter()
        .filter(|entry| entry.get("kind").and_then(serde_json::Value::as_str) != Some("offline"))
        .map(|entry| serde_json::from_value(entry).map_err(serde::de::Error::custom))
        .collect()
}

impl Versioned for AccountsFile {
    const SCHEMA: u32 = 1;
    fn schema(&self) -> u32 {
        self.schema
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AccountsView {
    pub active_id: Option<String>,
    pub accounts: Vec<Account>,
}
