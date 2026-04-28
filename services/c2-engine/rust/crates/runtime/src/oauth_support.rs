use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::{default_config_home, OAuthConfig};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthTokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>,
    #[serde(default)]
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthTokenExchangeRequest {
    pub client_id: String,
    pub code: String,
    pub redirect_uri: Option<String>,
    pub code_verifier: Option<String>,
    pub scopes: Vec<String>,
}

impl OAuthTokenExchangeRequest {
    #[must_use]
    pub fn form_params(&self) -> Vec<(String, String)> {
        let mut params = vec![
            ("grant_type".to_string(), "authorization_code".to_string()),
            ("client_id".to_string(), self.client_id.clone()),
            ("code".to_string(), self.code.clone()),
        ];
        if let Some(redirect_uri) = &self.redirect_uri {
            params.push(("redirect_uri".to_string(), redirect_uri.clone()));
        }
        if let Some(code_verifier) = &self.code_verifier {
            params.push(("code_verifier".to_string(), code_verifier.clone()));
        }
        if !self.scopes.is_empty() {
            params.push(("scope".to_string(), self.scopes.join(" ")));
        }
        params
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthRefreshRequest {
    pub client_id: String,
    pub refresh_token: String,
    pub scopes: Vec<String>,
}

impl OAuthRefreshRequest {
    #[must_use]
    pub fn from_config(
        config: &OAuthConfig,
        refresh_token: String,
        scopes: Option<Vec<String>>,
    ) -> Self {
        Self {
            client_id: config.client_id.clone(),
            refresh_token,
            scopes: scopes.unwrap_or_else(|| config.scopes.clone()),
        }
    }

    #[must_use]
    pub fn form_params(&self) -> Vec<(String, String)> {
        let mut params = vec![
            ("grant_type".to_string(), "refresh_token".to_string()),
            ("client_id".to_string(), self.client_id.clone()),
            ("refresh_token".to_string(), self.refresh_token.clone()),
        ];
        if !self.scopes.is_empty() {
            params.push(("scope".to_string(), self.scopes.join(" ")));
        }
        params
    }
}

pub fn load_oauth_credentials() -> std::io::Result<Option<OAuthTokenSet>> {
    let path = credentials_path();
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let token_set = serde_json::from_str::<OAuthTokenSet>(&contents).map_err(|error| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, format!("invalid oauth credentials: {error}"))
    })?;
    Ok(Some(token_set))
}

pub fn save_oauth_credentials(token_set: &OAuthTokenSet) -> std::io::Result<()> {
    let path = credentials_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string(token_set).map_err(invalid_data_error)?)
}

pub fn clear_oauth_credentials() -> std::io::Result<()> {
    match std::fs::remove_file(credentials_path()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn credentials_path() -> PathBuf {
    default_config_home().join("credentials.json")
}

fn invalid_data_error(error: serde_json::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string())
}
