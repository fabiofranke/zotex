use crate::zotero_api::types::{FetchItemsError, FetchItemsParams, FetchItemsResponse};
use reqwest::header::{self, HeaderMap};
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
        let mut headers = HeaderMap::new();
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

    fn try_get_next_page_url(headers: &HeaderMap) -> Option<String> {
        headers.get(header::LINK).and_then(|link_header| {
            let link_str = link_header.to_str().ok()?;
            for part in link_str.split(',') {
                let sections: Vec<&str> = part.split(';').map(|s| s.trim()).collect();
                if sections.len() == 2 && sections[1] == r#"rel="next""# {
                    let url = sections[0].trim_start_matches('<').trim_end_matches('>');
                    return Some(url.to_string());
                }
            }
            None
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[tokio::test]
    #[case(
        r#"<https://api.zotero.org/users/13622011/items?format=biblatex&start=25>; rel="next", <https://api.zotero.org/users/13622011/items?format=biblatex&start=50>; rel="last", <https://www.zotero.org/users/13622011/items>; rel="alternate""#,
        "https://api.zotero.org/users/13622011/items?format=biblatex&start=25"
    )]
    #[case(
        r#"<https://api.zotero.org/users/13622011/items?format=biblatex>; rel="first", <https://api.zotero.org/users/13622011/items?format=biblatex>; rel="prev", <https://api.zotero.org/users/13622011/items?format=biblatex&start=45>; rel="next", <https://api.zotero.org/users/13622011/items?format=biblatex&start=50>; rel="last", <https://www.zotero.org/users/13622011/items>; rel="alternate""#,
        "https://api.zotero.org/users/13622011/items?format=biblatex&start=45"
    )]
    #[case(
        r#"<https://api.zotero.org/users/13622011/items?format=xyz&start=100>; rel="next""#,
        "https://api.zotero.org/users/13622011/items?format=xyz&start=100"
    )]
    async fn next_page_url_some(#[case] link_header: &str, #[case] expected_url: &str) {
        let mut headers = HeaderMap::new();
        headers.insert(header::LINK, link_header.parse().unwrap());
        let next_page_url = ReqwestZoteroClient::try_get_next_page_url(&headers);
        assert_eq!(next_page_url, Some(expected_url.into()));
    }

    #[rstest]
    #[tokio::test]
    #[case(None)]
    #[case(Some(r#""#))]
    #[case(Some(r#"<https://api.zotero.org/users/13622011/items?format=biblatex>; rel="first", <https://api.zotero.org/users/13622011/items?format=biblatex&start=25>; rel="prev", <https://www.zotero.org/users/13622011/items>; rel="alternate""#))]
    async fn next_page_url_none(#[case] link_header: Option<&str>) {
        let mut headers = HeaderMap::new();
        if let Some(link_header) = link_header {
            headers.insert(header::LINK, link_header.parse().unwrap());
        }
        let next_page_url = ReqwestZoteroClient::try_get_next_page_url(&headers);
        assert_eq!(next_page_url, None);
    }
}
