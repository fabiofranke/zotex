use reqwest::{StatusCode, header::HeaderMap};

use crate::zotero_api::{
    API_BASE_URL,
    api_key::{ApiKey, ApiKeyError, ApiKeyInfo},
    client::ReqwestZoteroClient,
    types::ApiError,
};

pub struct ZoteroClientBuilder {
    http_client: reqwest::Client,
}

impl ZoteroClientBuilder {
    pub fn new(api_key: ApiKey) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Zotero-API-Version", "3".parse().unwrap());
        headers.insert("Zotero-API-Key", api_key.0.parse().unwrap());
        log::debug!("Default http headers: {:?}", headers);
        let http_client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();
        Self { http_client }
    }

    /// Validates the given API key and returns a client instance ready to be used.
    /// Fails if the key is invalid, has insufficient rights, or if something else went wrong with the Zotero API.
    pub async fn build(self) -> Result<ReqwestZoteroClient, ClientBuildError> {
        let response = self
            .http_client
            .get(format!("{}/keys/current", API_BASE_URL))
            .send()
            .await
            .map_err(ApiError::from)?;
        if response.status() != StatusCode::OK {
            return Err(ClientBuildError::ApiError(ApiError::UnexpectedStatus {
                status: response.status(),
                body: response.text().await.unwrap_or_default(),
            }));
        }
        let key_info = response
            .json::<ApiKeyInfo>()
            .await
            .map_err(ApiError::from)?;
        log::info!("Got a valid API key for user {}", key_info.username);
        if key_info.can_access_library() {
            let user_url = format!("{}/users/{}", API_BASE_URL, key_info.user_id);
            log::debug!("User URL: {}", user_url);
            Ok(ReqwestZoteroClient::new(self.http_client, user_url))
        } else {
            log::error!("Key does not have access to library");
            Err(ClientBuildError::ApiKeyError(
                ApiKeyError::InsufficientRights,
            ))
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ClientBuildError {
    #[error("Error from Zotero API")]
    ApiError(#[from] ApiError),
    #[error("Error with API key")]
    ApiKeyError(#[from] ApiKeyError),
}
