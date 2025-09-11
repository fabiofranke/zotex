use reqwest::header;

pub trait ZoteroClient {
    async fn fetch_items(&self) -> Result<String, Box<dyn std::error::Error>>;
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
        Self {
            user_url: format!("https://api.zotero.org/users/{}", user_id),
            client: reqwest::Client::builder()
                .default_headers(headers)
                .build()
                .unwrap(),
        }
    }
}

impl ZoteroClient for ReqwestZoteroClient {
    async fn fetch_items(&self) -> Result<String, Box<dyn std::error::Error>> {
        self.client
            .get(format!("{}{}", self.user_url, "/items?format=biblatex"))
            .send()
            .await?
            .text()
            .await
            .map_err(|e| e.into())
    }
}
