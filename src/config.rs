

use std::io::ErrorKind;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone, Debug, Default)]
pub struct AgenciesConfig(pub IndexMap<String, Agency>);

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Agency {
    pub id: String,
}


#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "kebab-case")]
pub struct ApiConfig{
    
    pub base_url: String,
    pub api_version: u32,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Config {
    pub api: ApiConfig,
    pub agencies: AgenciesConfig,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Secrets {
    pub access_tokens: IndexMap<String,AccessToken>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct TokensRequest<'a> {
    pub refresh_token: &'a str,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct TokensResponse {
    pub access_token: String,
}

impl Secrets {
    fn get_access_token(&mut self, ApiConfig{base_url, api_version, ..}: &ApiConfig) -> std::io::Result<Option<&str>> {
        let Some(token) = self.access_tokens.get_mut(base_url) else {
            return Ok(None)
        };

        match token.r#type {
            AccessTokenType::Access => Ok(Some(&token.token)),
            AccessTokenType::Refresh => {
                let get_token = format!("https://{base_url}/v1/tokens");

                let mut req = ureq::post(get_token)
                    .content_type("application/json")
                    .send_json(TokensRequest{refresh_token: &token.token})
                    .map_err(|e| e.into_io())?;

                let response: TokensResponse = req.body_mut().read_json().map_err(|e| e.into_io())?;

                *token = AccessToken { token: response.access_token, r#type: AccessTokenType::Access };

                Ok(Some(&token.token))
            }
        }


    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AccessTokenType {
    #[default]
    Access,
    Refresh
}

#[derive(Deserialize, Clone, Default)]
#[serde(rename_all = "kebab-case")]
pub struct AccessToken {
    pub token: String,
    #[serde(default)]
    pub r#type: AccessTokenType,
}

impl core::fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccessToken")
            .field_with("token", |v| {
                v.write_str("\"")?;
                if self.token.len() > 16 {
                    v.write_str(&self.token[..4])?;
                }
                v.write_str("...\"")
            })
            .field("type", &self.r#type)
            .finish()
    }
}

#[derive(Clone, Debug, Default)]
pub struct GlobalConfig {
    pub config: Config,
    pub secrets: Secrets,
}

impl GlobalConfig {
    pub fn get_global_access_token(&mut self) -> std::io::Result<Option<&str>> {
        self.secrets.get_access_token(&self.config.api)
    }
}


pub fn load_config() -> std::io::Result<GlobalConfig> {
    let config_file = std::fs::read_to_string("config.toml")?;
    let secrets_file = std::fs::read_to_string("secrets.toml")
        .map(Some)
        .or_else(|e| match e.kind() {
            ErrorKind::NotFound | ErrorKind::PermissionDenied | ErrorKind::IsADirectory => Ok(None),
            _ => Err(e)
        })?;

    Ok(GlobalConfig {
        config: toml::from_str(&config_file).map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))?,
        secrets: secrets_file.as_ref().map(|v| toml::from_str(v)).transpose().map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))?.unwrap_or_default(),
    })
}