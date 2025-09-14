use crate::zotero_api::{ApiError, FetchItemsParams, FetchItemsResponse, headers};
use reqwest::header::{self, HeaderMap};

pub trait ZoteroClient {
    async fn fetch_items(&self, params: &FetchItemsParams) -> Result<FetchItemsResponse, ApiError>;
}

pub struct ReqwestZoteroClient {
    http_client: reqwest::Client,
    user_url: String,
}

impl ReqwestZoteroClient {
    pub(in crate::zotero_api) fn new(http_client: reqwest::Client, user_url: String) -> Self {
        Self {
            user_url,
            http_client,
        }
    }

    async fn fetch_page(
        &self,
        url: &str,
        headers: &HeaderMap,
    ) -> Result<FetchPageResponse, ApiError> {
        let request = self.http_client.get(url).headers(headers.clone()).build()?;
        Self::log_request(&request);
        let response = self.http_client.execute(request).await?;
        Self::log_response(&response);
        Self::parse_zotero_page_response(response).await
    }

    fn log_request(request: &reqwest::Request) {
        log::trace!(
            "Sending request: {} {}\nHeaders: {:?}",
            request.method(),
            request.url(),
            request.headers()
        );
    }

    fn log_response(response: &reqwest::Response) {
        log::trace!(
            "Received response: {} {}\nHeaders: {:?}",
            response.status(),
            response.url(),
            response.headers()
        );
    }

    async fn parse_zotero_page_response(
        response: reqwest::Response,
    ) -> Result<FetchPageResponse, ApiError> {
        match response.status() {
            reqwest::StatusCode::OK => {
                let last_modified_version = response
                    .headers()
                    .get(headers::LAST_MODIFIED_VERSION)
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
                Err(ApiError::UnexpectedStatus {
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
    async fn fetch_items(&self, params: &FetchItemsParams) -> Result<FetchItemsResponse, ApiError> {
        let mut next_url = Some(format!("{}/items?format={}", self.user_url, params.format));
        let mut headers = HeaderMap::new();
        if let Some(version) = params.last_modified_version {
            headers.insert(headers::IF_MODIFIED_SINCE_VERSION, version.into());
        }
        let mut result = Ok(FetchItemsResponse::UpToDate);

        while let Some(url) = next_url {
            match self.fetch_page(&url, &headers).await {
                Ok(FetchPageResponse::Updated {
                    last_modified_version,
                    text,
                    next_page_url,
                }) => {
                    if let Ok(FetchItemsResponse::Updated {
                        text: existing_text,
                        ..
                    }) = &mut result
                    {
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
