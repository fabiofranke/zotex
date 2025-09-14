/// Input for a request to fetch items from the Zotero API.
pub struct FetchItemsParams {
    /// Version of the library at the time of the last fetch.
    pub last_modified_version: Option<u64>,
}

/// The happy path response when fetching items.
pub enum FetchItemsResponse {
    /// No updates since last fetch.
    UpToDate,
    /// New or updated items are available.
    Updated {
        last_modified_version: u64,
        text: String,
    },
}

/// Errors that can occur when interacting with the Zotero API.
#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("HTTP error")]
    HttpError(#[from] reqwest::Error),

    #[error("Unexpected response status: '{status}' with body: '{body}'")]
    UnexpectedStatus {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("Operation was cancelled")]
    Cancelled,
}
