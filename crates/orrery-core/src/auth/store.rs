use std::path::PathBuf;

use tokio::sync::RwLock;

use super::account::{Account, AccountsFile, AccountsView};
use crate::storage::{self, Versioned};
use crate::{CoreError, CoreResult};

pub struct AccountStore {
    path: PathBuf,
    data: RwLock<AccountsFile>,
}

impl AccountStore {
    pub async fn load(path: PathBuf) -> CoreResult<Self> {
        let mut data = storage::read_versioned::<AccountsFile>(&path)
            .await?
            .unwrap_or(AccountsFile {
                schema: AccountsFile::SCHEMA,
                ..AccountsFile::default()
            });
        let active_exists = data
            .accounts
            .iter()
            .any(|account| Some(&account.id) == data.active_id.as_ref());
        if !active_exists {
            data.active_id = data.accounts.first().map(|account| account.id.clone());
        }
        Ok(Self {
            path,
            data: RwLock::new(data),
        })
    }

    pub async fn view(&self) -> AccountsView {
        let data = self.data.read().await;
        AccountsView {
            active_id: data.active_id.clone(),
            accounts: data.accounts.clone(),
        }
    }

    pub async fn get(&self, id: &str) -> CoreResult<Account> {
        self.data
            .read()
            .await
            .accounts
            .iter()
            .find(|account| account.id == id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(format!("account {id}")))
    }

    pub async fn launch_account(&self, id: Option<&str>) -> CoreResult<Account> {
        let id = match id {
            Some(id) => id.to_string(),
            None => self
                .data
                .read()
                .await
                .active_id
                .clone()
                .ok_or_else(|| CoreError::InvalidInput("no account selected".into()))?,
        };
        self.get(&id).await
    }

    pub async fn upsert(&self, account: Account) -> CoreResult<Account> {
        let mut data = self.data.write().await;
        let mut next = data.clone();
        let stored = match next
            .accounts
            .iter_mut()
            .find(|a| a.kind == account.kind && a.uuid == account.uuid)
        {
            Some(existing) => {
                existing.username = account.username;
                existing.skin = account.skin;
                existing.clone()
            }
            None => {
                next.accounts.push(account.clone());
                account
            }
        };
        next.active_id = Some(stored.id.clone());
        self.persist(&mut data, next).await?;
        Ok(stored)
    }

    pub async fn remove(&self, id: &str) -> CoreResult<Account> {
        let mut data = self.data.write().await;
        let mut next = data.clone();
        let index = next
            .accounts
            .iter()
            .position(|a| a.id == id)
            .ok_or_else(|| CoreError::NotFound(format!("account {id}")))?;
        let removed = next.accounts.remove(index);
        if next.active_id.as_deref() == Some(id) {
            next.active_id = next.accounts.first().map(|a| a.id.clone());
        }
        self.persist(&mut data, next).await?;
        Ok(removed)
    }

    pub async fn set_active(&self, id: &str) -> CoreResult<()> {
        let mut data = self.data.write().await;
        if !data.accounts.iter().any(|a| a.id == id) {
            return Err(CoreError::NotFound(format!("account {id}")));
        }
        let mut next = data.clone();
        next.active_id = Some(id.to_string());
        self.persist(&mut data, next).await
    }

    pub async fn update(&self, id: &str, change: impl FnOnce(&mut Account)) -> CoreResult<Account> {
        let mut data = self.data.write().await;
        let mut next = data.clone();
        let account = next
            .accounts
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| CoreError::NotFound(format!("account {id}")))?;
        change(account);
        let updated = account.clone();
        self.persist(&mut data, next).await?;
        Ok(updated)
    }

    async fn persist(&self, current: &mut AccountsFile, next: AccountsFile) -> CoreResult<()> {
        storage::write_json_atomic(&self.path, &next).await?;
        *current = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::auth::AccountKind;

    async fn store(dir: &Path) -> AccountStore {
        AccountStore::load(dir.join("accounts.json")).await.unwrap()
    }

    fn microsoft(name: &str, uuid: &str) -> Account {
        Account {
            id: uuid::Uuid::new_v4().to_string(),
            kind: AccountKind::Microsoft,
            uuid: uuid.into(),
            username: name.into(),
            skin: None,
            added_at: 0,
        }
    }

    #[tokio::test]
    async fn accounts_can_be_added_switched_and_removed() {
        let dir = tempfile::tempdir().unwrap();
        let accounts = store(dir.path()).await;

        let steve = accounts
            .upsert(microsoft("Steve", "0123456789abcdef0123456789abcdef"))
            .await
            .unwrap();
        let alex = accounts
            .upsert(microsoft("Alex", "fedcba9876543210fedcba9876543210"))
            .await
            .unwrap();
        assert_eq!(
            accounts.view().await.active_id.as_deref(),
            Some(alex.id.as_str())
        );

        accounts.set_active(&steve.id).await.unwrap();
        let reloaded = store(dir.path()).await;
        assert_eq!(
            reloaded.launch_account(None).await.unwrap().username,
            "Steve"
        );

        reloaded.remove(&steve.id).await.unwrap();
        assert_eq!(
            reloaded.view().await.active_id.as_deref(),
            Some(alex.id.as_str())
        );
        assert!(reloaded.remove(&steve.id).await.is_err());
    }

    #[tokio::test]
    async fn stored_offline_accounts_are_dropped_on_load() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("accounts.json");
        std::fs::write(
            &file,
            r#"{
                "schema": 1,
                "activeId": "a1",
                "accounts": [
                    {"id": "a1", "kind": "offline", "uuid": "abc", "username": "Steve",
                     "skin": {"type": "local", "slim": false}, "addedAt": 0},
                    {"id": "a2", "kind": "microsoft", "uuid": "def", "username": "Alex",
                     "skin": null, "addedAt": 0}
                ]
            }"#,
        )
        .unwrap();

        let accounts = store(dir.path()).await;
        let view = accounts.view().await;
        assert_eq!(view.accounts.len(), 1);
        assert_eq!(view.accounts[0].username, "Alex");
        assert_eq!(view.active_id.as_deref(), Some("a2"));
    }

    #[tokio::test]
    async fn malformed_accounts_are_still_reported() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("accounts.json"),
            r#"{"schema": 1, "accounts": [{"id": "a1", "kind": "unknown"}]}"#,
        )
        .unwrap();
        assert!(
            AccountStore::load(dir.path().join("accounts.json"))
                .await
                .is_err()
        );
    }
}
