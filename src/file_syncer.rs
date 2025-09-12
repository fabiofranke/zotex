use std::time::Duration;

use crate::zotero_api::{
    client::ZoteroClient,
    types::{FetchItemsError, FetchItemsParams, FetchItemsResponse},
};
use tokio::fs::OpenOptions;
use tokio::io::AsyncBufReadExt;
use tokio_util::sync::CancellationToken;

pub struct FileSyncer<TClient: ZoteroClient> {
    client: TClient,
    file_path: String,
}

impl<TClient: ZoteroClient> FileSyncer<TClient> {
    pub async fn try_new(client: TClient, file_path: String) -> Result<Self, SyncError> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&file_path)
            .await
            .map_err(|e| SyncError::FileError {
                file_path: file_path.clone(),
                io_error: e,
            })?;
        Ok(Self { client, file_path })
    }

    pub async fn sync(
        &self,
        interval: Option<Duration>,
        cancellation_token: CancellationToken,
    ) -> Result<SyncSuccess, SyncError> {
        match interval {
            Some(duration) if (duration.as_secs() > 0) => {
                log::info!(
                    "Starting periodic sync every {} seconds.",
                    duration.as_secs()
                );
                self.sync_periodically(duration, cancellation_token).await
            }
            _ => {
                log::info!("Starting one-time sync.");
                self.sync_once(cancellation_token).await
            }
        }
    }

    async fn sync_periodically(
        &self,
        duration: Duration,
        cancellation_token: CancellationToken,
    ) -> Result<SyncSuccess, SyncError> {
        let mut interval = tokio::time::interval(duration);
        let mut has_changes = false;
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    log::info!("Starting scheduled sync.");
                    match self.sync_once(cancellation_token.child_token()).await {
                        Ok(SyncSuccess::Changes) => {
                            has_changes = true;
                        }
                        Ok(SyncSuccess::NoChanges) => {
                            // nothing to do
                        }
                        Err(e) => {
                            log::error!("Aborting periodic sync due to error: {}", e);
                            return Err(e);
                        }
                    }
                }
                _ = cancellation_token.cancelled() => {
                    log::info!("Cancellation requested, stopping periodic sync.");
                    break;
                }
            }
        }
        Ok(if has_changes {
            SyncSuccess::Changes
        } else {
            SyncSuccess::NoChanges
        })
    }

    async fn sync_once(
        &self,
        cancellation_token: CancellationToken,
    ) -> Result<SyncSuccess, SyncError> {
        let header = self.try_read_file_headline().await;
        if let Some(h) = &header {
            log::info!(
                "Found existing export with version {}",
                h.last_modified_version
            );
        } else {
            log::info!("No existing export found, performing full fetch.");
        }
        let params = FetchItemsParams {
            last_modified_version: header.map(|h| h.last_modified_version),
        };
        let response = self.client.fetch_items(&params, cancellation_token).await?;
        match response {
            FetchItemsResponse::UpToDate => {
                log::info!(
                    "File '{}' is up to date with the Zotero library.",
                    &self.file_path
                );
                Ok(SyncSuccess::NoChanges)
            }
            FetchItemsResponse::Updated {
                last_modified_version,
                text: items,
            } => {
                let header = FileHeadline {
                    last_modified_version,
                };
                let file_content = format!("{}\n{}", String::from(header), items);
                tokio::fs::write(&self.file_path, file_content)
                    .await
                    .map_err(|e| SyncError::FileError {
                        file_path: self.file_path.clone(),
                        io_error: e,
                    })?;
                log::info!(
                    "Wrote library export with version {} to file '{}'.",
                    last_modified_version,
                    &self.file_path
                );
                Ok(SyncSuccess::Changes)
            }
        }
    }

    async fn try_read_file_headline(&self) -> Option<FileHeadline> {
        let file = OpenOptions::new()
            .read(true)
            .open(&self.file_path)
            .await
            .ok()?;
        let mut reader = tokio::io::BufReader::new(file);
        let mut first_line = String::new();
        reader.read_line(&mut first_line).await.ok()?;
        FileHeadline::try_from(first_line.trim()).ok()
    }
}

pub enum SyncSuccess {
    Changes,
    NoChanges,
}

#[derive(thiserror::Error, Debug)]
pub enum SyncError {
    #[error("Error with file '{file_path}'")]
    FileError {
        file_path: String,
        #[source]
        io_error: std::io::Error,
    },
    #[error("Error in Zotero client")]
    ClientError(#[from] FetchItemsError),
}

struct FileHeadline {
    last_modified_version: u64,
}

impl FileHeadline {
    const PREFIX: &'static str = "% *** THIS FILE WAS AUTO-GENERATED BY ZOTEX - DO NOT EDIT ***";
    const VERSION_PREFIX: &'static str = "Last-Modified-Version: ";
}

impl From<FileHeadline> for String {
    fn from(headline: FileHeadline) -> Self {
        format!(
            "{}{}{}",
            FileHeadline::PREFIX,
            FileHeadline::VERSION_PREFIX,
            headline.last_modified_version
        )
    }
}

impl TryFrom<&str> for FileHeadline {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if !value.starts_with(Self::PREFIX) {
            return Err(());
        }
        let version_part = value.trim_start_matches(Self::PREFIX).trim();
        if !version_part.starts_with(Self::VERSION_PREFIX) {
            return Err(());
        }
        let version_str = version_part.trim_start_matches(Self::VERSION_PREFIX).trim();
        let last_modified_version = version_str.parse::<u64>().map_err(|_| ())?;
        Ok(Self {
            last_modified_version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_headline_string_conversion() {
        let headline = FileHeadline {
            last_modified_version: 12345,
        };
        let headline_str: String = headline.into();

        let parsed_headline = FileHeadline::try_from(headline_str.as_str());
        assert!(parsed_headline.is_ok());
        let parsed_headline = parsed_headline.unwrap();
        assert_eq!(parsed_headline.last_modified_version, 12345);
    }
}
