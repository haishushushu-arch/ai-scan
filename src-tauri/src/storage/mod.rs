use keyring::{release_store, use_native_store};
use keyring_core::{Entry, Error};
use serde::{Deserialize, Serialize};

const SERVICE: &str = "cn.msutools.ai-scan";
const SESSION_USER: &str = "msutools-session";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at_unix_ms: Option<i64>,
}

pub fn save_session(tokens: &SessionTokens) -> anyhow::Result<()> {
    init_store()?;
    let entry = Entry::new(SERVICE, SESSION_USER)?;
    entry.set_password(&serde_json::to_string(tokens)?)?;
    release_store();
    Ok(())
}

pub fn load_session() -> anyhow::Result<Option<SessionTokens>> {
    init_store()?;
    let entry = Entry::new(SERVICE, SESSION_USER)?;
    let result = match entry.get_password() {
        Ok(raw) => Ok(Some(serde_json::from_str::<SessionTokens>(&raw)?)),
        Err(Error::NoEntry) => Ok(None),
        Err(error) => Err(anyhow::Error::new(error)),
    };
    release_store();
    result
}

pub fn clear_session() -> anyhow::Result<()> {
    init_store()?;
    let entry = Entry::new(SERVICE, SESSION_USER)?;
    let result = match entry.delete_credential() {
        Ok(()) | Err(Error::NoEntry) => Ok(()),
        Err(error) => Err(anyhow::Error::new(error)),
    };
    release_store();
    result
}

fn init_store() -> anyhow::Result<()> {
    use_native_store(false).map_err(anyhow::Error::new)
}
