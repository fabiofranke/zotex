use std::fmt::Display;

use serde::{Deserialize, Serialize};

pub mod api_key;
pub mod builder;
pub mod client;

const API_BASE_URL: &str = "https://api.zotero.org";

mod headers {
    pub const ZOTERO_API_VERSION: &str = "Zotero-API-Version";
    pub const ZOTERO_API_KEY: &str = "Zotero-API-Key";
    pub const LAST_MODIFIED_VERSION: &str = "Last-Modified-Version";
    pub const IF_MODIFIED_SINCE_VERSION: &str = "If-Modified-Since-Version";
}

/// Input for a request to fetch items from the Zotero API.
pub struct FetchItemsParams {
    /// Version of the library at the time of the last export
    pub last_modified_version: Option<u64>,

    /// Format in which the library should be exported
    pub format: ExportFormat,
}

/// Zotero export formats supported by this tool
#[derive(clap::ValueEnum, Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExportFormat {
    #[default]
    Biblatex,
    Bibtex,
}

impl Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_variant::to_variant_name(self).unwrap_or_default()
        )
    }
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
}

#[cfg(test)]
mod tests {
    use crate::zotero_api::ExportFormat;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case(ExportFormat::Biblatex, "biblatex")]
    #[case(ExportFormat::Bibtex, "bibtex")]
    fn export_format_to_str(#[case] format: ExportFormat, #[case] string_representation: &str) {
        assert_eq!(format.to_string(), string_representation);
    }
}
