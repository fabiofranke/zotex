use reqwest::header;
use tokio_util::sync::CancellationToken;

pub trait ZoteroClient {
    async fn fetch_items(
        &self,
        cancellation_token: CancellationToken,
    ) -> Result<String, Box<dyn std::error::Error>>;
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
}

impl ZoteroClient for ReqwestZoteroClient {
    async fn fetch_items(
        &self,
        cancellation_token: CancellationToken,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let request = self
            .client
            .get(format!("{}{}", self.user_url, "/items?format=biblatex"))
            .build()?;

        log::trace!("Sending request: {:?}", request);

        tokio::select! {
            _ = cancellation_token.cancelled() => {
                log::info!("Cancellation requested, aborting fetch_items.");
                Err("Operation cancelled".into())
            }
            result = self.client.execute(request) => {
                let response = result?;
                log::trace!("Received response: {:?}", response);

                match response.status() {
                    reqwest::StatusCode::OK => {
                        let text = response.text().await?;
                        Ok(text)
                    }
                    status => {
                        let err_msg = format!("Failed to fetch items: HTTP {}", status);
                        Err(err_msg.into())
                    }
                }
            }
        }
    }
}
