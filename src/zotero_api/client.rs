use crate::zotero_api::types::{FetchItemsError, FetchItemsParams, FetchItemsResponse};
use reqwest::header::{self, HeaderMap};
use tokio_util::sync::CancellationToken;

pub trait ZoteroClient {
    async fn fetch_items(
        &self,
        params: &FetchItemsParams,
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

    async fn fetch_page(
        &self,
        url: &str,
        headers: &HeaderMap,
        cancellation_token: CancellationToken,
    ) -> Result<FetchPageResponse, FetchItemsError> {
        let request = self.client.get(url).headers(headers.clone()).build()?;

        log::trace!("Sending request: {:?}", request);

        tokio::select! {
            _ = cancellation_token.cancelled() => {
                log::info!("Cancellation requested, aborting fetch_page.");
                Err(FetchItemsError::Cancelled)
            }
            request_result = self.client.execute(request) => {
                let response = request_result?;
                log::trace!("Received response: {:?}", response);
                Self::parse_zotero_page_response(response).await
            }
        }
    }

    async fn parse_zotero_page_response(
        response: reqwest::Response,
    ) -> Result<FetchPageResponse, FetchItemsError> {
        match response.status() {
            reqwest::StatusCode::OK => {
                let last_modified_version = response
                    .headers()
                    .get("Last-Modified-Version")
                    .and_then(|hv| hv.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);
                let next_page_url = Self::try_get_next_page_url(response.headers());
                let text = response.text().await?;
                Ok(FetchPageResponse::Updated {
                    last_modified_version,
                    text,
                    next_page_url,
                })
            }
            reqwest::StatusCode::NOT_MODIFIED => Ok(FetchPageResponse::UpToDate),
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

enum FetchPageResponse {
    UpToDate,
    Updated {
        last_modified_version: u64,
        text: String,
        next_page_url: Option<String>,
    },
}

impl ZoteroClient for ReqwestZoteroClient {
    async fn fetch_items(
        &self,
        params: &FetchItemsParams,
        cancellation_token: CancellationToken,
    ) -> Result<FetchItemsResponse, FetchItemsError> {
        let mut next_url = Some(format!(
            "{}{}",
            self.user_url, "/items?format=biblatex&limit=25"
        ));
        let mut headers = HeaderMap::new();
        if let Some(version) = params.last_modified_version {
            headers.insert("If-Modified-Since-Version", version.into());
        }

        let mut result = Ok(FetchItemsResponse::UpToDate);

        while let Some(url) = next_url {
            tokio::select! {
                _ = cancellation_token.cancelled() => {
                    log::info!("Cancellation requested, aborting fetch_items.");
                    return Err(FetchItemsError::Cancelled);
                }
                page_result = self.fetch_page(&url, &headers, cancellation_token.child_token()) => {
                    match page_result {
                        Ok(FetchPageResponse::Updated { last_modified_version, text, next_page_url }) => {
                            if let Ok(FetchItemsResponse::Updated { text: existing_text, .. }) = &mut result {
                                existing_text.push_str(&text);
                            } else {
                                result = Ok(FetchItemsResponse::Updated {
                                    last_modified_version,
                                    text,
                                });
                            }
                            next_url = next_page_url;
                        }
                        Ok(FetchPageResponse::UpToDate) => {
                            result = Ok(FetchItemsResponse::UpToDate);
                            next_url = None;
                        }
                        Err(e) => {
                            result = Err(e);
                            next_url = None;
                        }
                    }
                }
            }
        }
        result
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
