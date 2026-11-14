//! Saved accounts in the Wex data root.
//!
//! `<root>/accounts.json` holds the list and the active account: no secrets.
//! `<root>/accounts/<id>.secret` holds the sealed Microsoft tokens of one account
//! (see `secret.rs`). Both live in the data root (default `%APPDATA%\Wex`), never
//! next to the executable, so reinstalling or updating the launcher keeps them.

use super::secret::{self, SecretCipher};
use crate::auth::offline_uuid;
use crate::error::{Result, WexError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const FILE_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    Offline,
    Microsoft,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// `offline-<uuid>` or `ms-<profile uuid>`; stable across renames of a Microsoft profile.
    pub id: String,
    pub kind: AccountKind,
    /// In-game name.
    pub name: String,
    /// Player UUID without dashes, as Mojang services return it.
    pub uuid: String,
    pub added_at: DateTime<Utc>,
    #[serde(default)]
    pub last_used_at: Option<DateTime<Utc>>,
    /// Microsoft only: the saved sign-in was rejected and the player must sign in again.
    #[serde(default)]
    pub needs_sign_in: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct AccountsFile {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    active_id: Option<String>,
    #[serde(default)]
    accounts: Vec<Account>,
}

/// Microsoft tokens of one account. Never serialised anywhere except sealed.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MsSecrets {
    pub refresh_token: String,
    pub mc_access_token: String,
    pub mc_expires_at: DateTime<Utc>,
    #[serde(default)]
    pub xuid: Option<String>,
}

impl std::fmt::Debug for MsSecrets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MsSecrets")
            .field("refresh_token", &"***")
            .field("mc_access_token", &"***")
            .field("mc_expires_at", &self.mc_expires_at)
            .field("xuid", &self.xuid.as_ref().map(|_| "***"))
            .finish()
    }
}

/// Vanilla rule for offline names: 3–16 characters, Latin letters, digits and `_`.
pub fn is_valid_offline_name(name: &str) -> bool {
    (3..=16).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub struct AccountStore {
    root: PathBuf,
    file: AccountsFile,
    cipher: Box<dyn SecretCipher>,
}

impl AccountStore {
    /// Open the store in a data root with the system cipher.
    pub fn open(root: &Path) -> Result<Self> {
        Self::open_with(root, secret::system_cipher())
    }

    pub fn open_with(root: &Path, cipher: Box<dyn SecretCipher>) -> Result<Self> {
        let path = root.join("accounts.json");
        let file = match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str::<AccountsFile>(&text).unwrap_or_else(|e| {
                // Keep the unreadable file for diagnosis; the player signs in again.
                tracing::warn!(
                    "accounts.json is unreadable ({}), kept as accounts.json.broken",
                    e
                );
                let _ = fs::copy(&path, root.join("accounts.json.broken"));
                AccountsFile::default()
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => AccountsFile::default(),
            Err(e) => return Err(e.into()),
        };
        Ok(Self {
            root: root.to_path_buf(),
            file,
            cipher,
        })
    }

    pub fn cipher_name(&self) -> &'static str {
        self.cipher.name()
    }

    pub fn accounts(&self) -> &[Account] {
        &self.file.accounts
    }

    pub fn active_id(&self) -> Option<&str> {
        self.file.active_id.as_deref()
    }

    pub fn active(&self) -> Option<&Account> {
        let id = self.file.active_id.as_deref()?;
        self.get(id)
    }

    pub fn get(&self, id: &str) -> Option<&Account> {
        self.file.accounts.iter().find(|a| a.id == id)
    }

    fn get_mut(&mut self, id: &str) -> Option<&mut Account> {
        self.file.accounts.iter_mut().find(|a| a.id == id)
    }

    fn secrets_dir(&self) -> PathBuf {
        self.root.join("accounts")
    }

    fn secret_path(&self, id: &str) -> PathBuf {
        // Ids are built by Wex from [a-z0-9-]; keep only those characters in the file name anyway.
        let safe: String = id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        self.secrets_dir().join(format!("{}.secret", safe))
    }

    /// Write accounts.json atomically (temporary file, then rename).
    pub fn save(&mut self) -> Result<()> {
        self.file.version = FILE_VERSION;
        fs::create_dir_all(&self.root)?;
        let json = serde_json::to_string_pretty(&self.file)?;
        write_atomic(&self.root.join("accounts.json"), json.as_bytes())
    }

    /// Add an offline account (or reuse the one with the same name) and make it active.
    pub fn add_offline(&mut self, name: &str) -> Result<Account> {
        let name = name.trim();
        if !is_valid_offline_name(name) {
            return Err(WexError::Auth {
                code: "AUTH_BAD_NAME",
                message: "Ник: от 3 до 16 символов, латинские буквы, цифры и _.".to_string(),
                details: None,
            });
        }
        let uuid = offline_uuid(name).simple().to_string();
        let id = format!("offline-{}", uuid);
        if self.get(&id).is_none() {
            self.file.accounts.push(Account {
                id: id.clone(),
                kind: AccountKind::Offline,
                name: name.to_string(),
                uuid,
                added_at: Utc::now(),
                last_used_at: None,
                needs_sign_in: false,
            });
        }
        self.file.active_id = Some(id.clone());
        self.save()?;
        Ok(self.get(&id).cloned().expect("just added"))
    }

    /// Change the name of an offline account; its UUID follows the name, as in vanilla.
    pub fn rename_offline(&mut self, id: &str, name: &str) -> Result<Account> {
        let name = name.trim().to_string();
        if !is_valid_offline_name(&name) {
            return Err(WexError::Auth {
                code: "AUTH_BAD_NAME",
                message: "Ник: от 3 до 16 символов, латинские буквы, цифры и _.".to_string(),
                details: None,
            });
        }
        let uuid = offline_uuid(&name).simple().to_string();
        let new_id = format!("offline-{}", uuid);
        match self.get(id) {
            Some(a) if a.kind == AccountKind::Offline => {}
            _ => return Err(not_found()),
        }
        if new_id != id && self.get(&new_id).is_some() {
            // Another saved offline account already has that name: switch to it instead.
            self.file.accounts.retain(|a| a.id != id);
        } else {
            let acc = self.get_mut(id).expect("checked");
            acc.id = new_id.clone();
            acc.name = name;
            acc.uuid = uuid;
        }
        if self.file.active_id.as_deref() == Some(id) {
            self.file.active_id = Some(new_id.clone());
        }
        self.save()?;
        Ok(self.get(&new_id).cloned().expect("present"))
    }

    /// Add or update a Microsoft account with fresh tokens and make it active.
    pub fn upsert_microsoft(
        &mut self,
        uuid: &str,
        name: &str,
        secrets: &MsSecrets,
    ) -> Result<Account> {
        let uuid = uuid.replace('-', "").to_ascii_lowercase();
        let id = format!("ms-{}", uuid);
        // Secrets first: an account entry never points at missing tokens.
        self.write_secrets(&id, secrets)?;
        match self.get_mut(&id) {
            Some(acc) => {
                acc.name = name.to_string();
                acc.needs_sign_in = false;
            }
            None => self.file.accounts.push(Account {
                id: id.clone(),
                kind: AccountKind::Microsoft,
                name: name.to_string(),
                uuid,
                added_at: Utc::now(),
                last_used_at: None,
                needs_sign_in: false,
            }),
        }
        self.file.active_id = Some(id.clone());
        self.save()?;
        Ok(self.get(&id).cloned().expect("present"))
    }

    pub fn switch(&mut self, id: &str) -> Result<Account> {
        let acc = self.get(id).cloned().ok_or_else(not_found)?;
        self.file.active_id = Some(acc.id.clone());
        self.save()?;
        Ok(acc)
    }

    /// Remove an account and delete its tokens. The next account becomes active, if any.
    pub fn sign_out(&mut self, id: &str) -> Result<Option<Account>> {
        if self.get(id).is_none() {
            return Err(not_found());
        }
        let path = self.secret_path(id);
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        self.file.accounts.retain(|a| a.id != id);
        if self.file.active_id.as_deref() == Some(id) {
            self.file.active_id = self.file.accounts.first().map(|a| a.id.clone());
        }
        self.save()?;
        Ok(self.active().cloned())
    }

    pub fn mark_used(&mut self, id: &str) -> Result<()> {
        if let Some(acc) = self.get_mut(id) {
            acc.last_used_at = Some(Utc::now());
        }
        self.save()
    }

    pub fn mark_needs_sign_in(&mut self, id: &str) -> Result<()> {
        if let Some(acc) = self.get_mut(id) {
            acc.needs_sign_in = true;
        }
        self.save()
    }

    pub fn update_microsoft_name(&mut self, id: &str, name: &str) -> Result<()> {
        if let Some(acc) = self.get_mut(id) {
            acc.name = name.to_string();
        }
        self.save()
    }

    pub fn write_secrets(&self, id: &str, secrets: &MsSecrets) -> Result<()> {
        fs::create_dir_all(self.secrets_dir())?;
        let plain = serde_json::to_vec(secrets)?;
        let data = secret::encode(self.cipher.as_ref(), &plain)?;
        let path = self.secret_path(id);
        write_atomic(&path, &data)?;
        restrict_to_owner(&path);
        Ok(())
    }

    pub fn read_secrets(&self, id: &str) -> Result<MsSecrets> {
        let data = match fs::read(self.secret_path(id)) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(WexError::Auth {
                    code: "AUTH_SIGN_IN_AGAIN",
                    message: "Вход Microsoft не сохранён. Войдите в аккаунт ещё раз.".to_string(),
                    details: None,
                })
            }
            Err(e) => return Err(e.into()),
        };
        let plain = secret::decode(self.cipher.as_ref(), &data)?;
        serde_json::from_slice(&plain).map_err(|e| secret::corrupt(&e.to_string()))
    }
}

fn not_found() -> WexError {
    WexError::Auth {
        code: "AUTH_ACCOUNT_NOT_FOUND",
        message: "Этого аккаунта больше нет в лаунчере.".to_string(),
        details: None,
    }
}

fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(unix)]
fn restrict_to_owner(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_to_owner(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn secrets(tag: &str) -> MsSecrets {
        MsSecrets {
            refresh_token: format!("refresh-{}-PLAINTEXT-MARKER", tag),
            mc_access_token: format!("access-{}-PLAINTEXT-MARKER", tag),
            mc_expires_at: DateTime::parse_from_rfc3339("2030-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            xuid: Some("2535400000000000".to_string()),
        }
    }

    #[test]
    fn offline_account_uses_vanilla_uuid_and_validates_name() {
        let tmp = tempdir().unwrap();
        let mut store = AccountStore::open(tmp.path()).unwrap();
        let acc = store.add_offline("Player").unwrap();
        assert_eq!(acc.uuid, "a01e3843e5213998958af459800e4d11");
        assert_eq!(acc.kind, AccountKind::Offline);
        assert_eq!(store.active_id(), Some(acc.id.as_str()));
        assert!(store.add_offline("no").is_err());
        assert!(store.add_offline("bad name!").is_err());
        assert!(store.add_offline("ThisNameIsWayTooLong").is_err());
        // Same name twice: one account.
        store.add_offline("Player").unwrap();
        assert_eq!(store.accounts().len(), 1);
    }

    #[test]
    fn secrets_are_sealed_on_disk() {
        let tmp = tempdir().unwrap();
        let mut store = AccountStore::open(tmp.path()).unwrap();
        let acc = store
            .upsert_microsoft(
                "0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0",
                "Steve",
                &secrets("a"),
            )
            .unwrap();
        let file = tmp
            .path()
            .join("accounts")
            .join(format!("{}.secret", acc.id));
        let raw = fs::read(&file).unwrap();
        #[cfg(windows)]
        assert!(
            !raw.windows(9).any(|w| w == b"PLAINTEXT"),
            "tokens must not be stored as plain text"
        );
        assert!(raw.starts_with(b"WEXS"));
        let listing = fs::read_to_string(tmp.path().join("accounts.json")).unwrap();
        assert!(
            !listing.contains("PLAINTEXT"),
            "accounts.json must not contain tokens"
        );
        assert_eq!(store.read_secrets(&acc.id).unwrap(), secrets("a"));
        assert!(!format!("{:?}", secrets("a")).contains("PLAINTEXT"));
    }

    #[test]
    fn accounts_survive_reinstall() {
        // The data root is separate from the install folder: removing and
        // reinstalling the launcher binaries leaves both files in place.
        let data_root = tempdir().unwrap();
        let install_dir = tempdir().unwrap();
        fs::write(install_dir.path().join("wex.exe"), b"old build").unwrap();

        let ms_id = {
            let mut store = AccountStore::open(data_root.path()).unwrap();
            store.add_offline("FriendOne").unwrap();
            store
                .upsert_microsoft("00112233445566778899aabbccddeeff", "Alex", &secrets("b"))
                .unwrap()
                .id
        };

        // "Uninstall" and "reinstall".
        fs::remove_dir_all(install_dir.path()).unwrap();
        fs::create_dir_all(install_dir.path()).unwrap();
        fs::write(install_dir.path().join("wex.exe"), b"new build").unwrap();

        let store = AccountStore::open(data_root.path()).unwrap();
        assert_eq!(store.accounts().len(), 2);
        assert_eq!(store.active_id(), Some(ms_id.as_str()));
        assert_eq!(store.read_secrets(&ms_id).unwrap(), secrets("b"));
    }

    #[test]
    fn switch_and_sign_out_delete_tokens() {
        let tmp = tempdir().unwrap();
        let mut store = AccountStore::open(tmp.path()).unwrap();
        let off = store.add_offline("Builder").unwrap();
        let ms = store
            .upsert_microsoft("aa".repeat(16).as_str(), "Notch", &secrets("c"))
            .unwrap();
        assert_eq!(store.switch(&off.id).unwrap().id, off.id);
        assert_eq!(store.active_id(), Some(off.id.as_str()));
        assert!(store.switch("ms-missing").is_err());

        store.switch(&ms.id).unwrap();
        let next = store.sign_out(&ms.id).unwrap();
        assert_eq!(next.map(|a| a.id), Some(off.id.clone()));
        assert!(!tmp
            .path()
            .join("accounts")
            .join(format!("{}.secret", ms.id))
            .exists());
        assert!(store.read_secrets(&ms.id).is_err());

        store.sign_out(&off.id).unwrap();
        assert!(store.active().is_none());
        assert!(store.accounts().is_empty());
    }

    #[test]
    fn rename_offline_follows_vanilla_uuid() {
        let tmp = tempdir().unwrap();
        let mut store = AccountStore::open(tmp.path()).unwrap();
        let a = store.add_offline("Player").unwrap();
        let b = store.rename_offline(&a.id, "AuraUser").unwrap();
        assert_eq!(b.uuid, offline_uuid("AuraUser").simple().to_string());
        assert_eq!(store.active_id(), Some(b.id.as_str()));
        assert_eq!(store.accounts().len(), 1);
    }

    #[test]
    fn broken_listing_is_kept_aside() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("accounts.json"), b"{not json").unwrap();
        let store = AccountStore::open(tmp.path()).unwrap();
        assert!(store.accounts().is_empty());
        assert!(tmp.path().join("accounts.json.broken").exists());
    }
}
