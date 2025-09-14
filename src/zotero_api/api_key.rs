/// Structure for what the /keys/current endpoint returns on success.
pub struct ApiKeyInfo {
    pub user_id: String,
    pub username: String,
    pub access: KeyAccessInfo,
}

/// Details about what the API key can access (only the subset that is relevant for this tool)
pub struct KeyAccessInfo {
    pub user: KeyUserAccessInfo,
}

/// Details about what the API key can access of the user items (only the subset that is relevant for this tool)
pub struct KeyUserAccessInfo {
    pub library: bool,
}
