pub mod api_key;
pub mod builder;
pub mod client;
pub mod types;

const API_BASE_URL: &str = "https://api.zotero.org";

mod headers {
    pub const ZOTERO_API_VERSION: &str = "Zotero-API-Version";
    pub const ZOTERO_API_KEY: &str = "Zotero-API-Key";
    pub const LAST_MODIFIED_VERSION: &str = "Last-Modified-Version";
    pub const IF_MODIFIED_SINCE_VERSION: &str = "If-Modified-Since-Version";
}
