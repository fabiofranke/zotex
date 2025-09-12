/// The happy path response when fetching items.
pub enum FetchItemsResponse {
    /// No updates since last fetch.
    UpToDate,
    /// New or updated items are available.
    Updated(String),
}

/// Errors that can occur when fetching items from the Zotero API.
#[derive(thiserror::Error, Debug)]
pub enum FetchItemsError {
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Unexpected response status: '{status}' with body: '{body}'")]
    UnexpectedStatus {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("Operation was cancelled")]
    Cancelled,
}
