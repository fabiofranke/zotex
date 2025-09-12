use crate::zotero_api::types::{FetchItemsError, FetchItemsParams, FetchItemsResponse};
use reqwest::header;
use tokio_util::sync::CancellationToken;

pub trait ZoteroClient {
    async fn fetch_items(
        &self,
        params: FetchItemsParams,
        cancellation_token: CancellationToken,
    ) -> Result<FetchItemsResponse, FetchItemsError>;
}

pub struct ReqwestZoteroClient {
    user_url: String,
    client: reqwest::Client,
}

impl ReqwestZoteroClient {
    pub fn new(user_id: String, api_key: String) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert("Zotero-API-Version", "3".parse().unwrap());
        headers.insert("Zotero-API-Key", api_key.parse().unwrap());
        let user_url = format!("https://api.zotero.org/users/{}", user_id);
        log::trace!(
            "Creating client with user URL: '{}' and default headers: {:?}",
            user_url,
            headers
        );
        Self {
            user_url,
            client: reqwest::Client::builder()
                .default_headers(headers)
                .build()
                .unwrap(),
        }
    }

    async fn response_to_result(
        response: reqwest::Response,
    ) -> Result<FetchItemsResponse, FetchItemsError> {
        match response.status() {
            reqwest::StatusCode::OK => {
                let last_modified_version = response
                    .headers()
                    .get("Last-Modified-Version")
                    .and_then(|hv| hv.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);
                let text = response.text().await?;
                Ok(FetchItemsResponse::Updated {
                    last_modified_version,
                    text,
                })
            }
            reqwest::StatusCode::NOT_MODIFIED => Ok(FetchItemsResponse::UpToDate),
            other_status => {
                let body = response.text().await.unwrap_or_default();
                Err(FetchItemsError::UnexpectedStatus {
                    status: other_status,
                    body,
                })
            }
        }
    }
}

impl ZoteroClient for ReqwestZoteroClient {
    async fn fetch_items(
        &self,
        params: FetchItemsParams,
        cancellation_token: CancellationToken,
    ) -> Result<FetchItemsResponse, FetchItemsError> {
        let mut request_builder = self.client.get(format!(
            "{}{}",
            self.user_url, "/items?format=biblatex&limit=100"
        ));
        if let Some(version) = params.last_modified_version {
            request_builder = request_builder.header("If-Modified-Since-Version", version);
        }
        let request = request_builder.build()?;

        log::trace!("Sending request: {:?}", request);

        tokio::select! {
            _ = cancellation_token.cancelled() => {
                log::info!("Cancellation requested, aborting fetch_items.");
                Err(FetchItemsError::Cancelled)
            }
            result = self.client.execute(request) => {
                let response = result?;
                log::trace!("Received response: {:?}", response);
                Self::response_to_result(response).await
            }
        }
    }
}
