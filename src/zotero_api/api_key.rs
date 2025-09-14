/// Wrapper for the API key string.
pub struct ApiKey(pub String);

/// Structure for what the GET /keys/current endpoint returns on success.
#[derive(Debug, serde::Deserialize)]
pub struct ApiKeyInfo {
    #[serde(rename = "userID")]
    pub user_id: u64,
    pub username: String,
    access: KeyAccessInfo,
}

impl ApiKeyInfo {
    pub fn can_access_library(&self) -> bool {
        self.access.user.library
    }
}

/// Details about what the API key can access (only the subset that is relevant for this tool)
#[derive(Debug, serde::Deserialize)]
struct KeyAccessInfo {
    user: KeyUserAccessInfo,
}

/// Details about what the API key can access of the user items (only the subset that is relevant for this tool)
#[derive(Debug, serde::Deserialize)]
struct KeyUserAccessInfo {
    library: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum ApiKeyError {
    #[error("Insufficient access rights for API key. Needs at least read access to user library.")]
    InsufficientRights,
}
