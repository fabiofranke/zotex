use std::time::Duration;

use crate::zotero_api::{
    client::ZoteroClient,
    types::{FetchItemsError, FetchItemsResponse},
};
use tokio::fs::OpenOptions;
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
            .await?;
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
        let response = self.client.fetch_items(cancellation_token).await?;
        match response {
            FetchItemsResponse::UpToDate => {
                log::info!(
                    "File '{}' is up to date with the Zotero library.",
                    &self.file_path
                );
                Ok(SyncSuccess::NoChanges)
            }
            FetchItemsResponse::Updated(items) => {
                tokio::fs::write(&self.file_path, items).await?;
                log::info!("Wrote updated items to '{}'.", &self.file_path);
                Ok(SyncSuccess::Changes)
            }
        }
    }
}

pub enum SyncSuccess {
    Changes,
    NoChanges,
}

#[derive(thiserror::Error, Debug)]
pub enum SyncError {
    #[error("Error with file operation: {0}")]
    FileError(#[from] std::io::Error),
    #[error("Error in Zotero client: {0}")]
    ClientError(#[from] FetchItemsError),
}
