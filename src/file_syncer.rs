use std::time::Duration;

use tokio::fs::OpenOptions;
use tokio_util::sync::CancellationToken;

use crate::zotero_client::ZoteroClient;

pub struct FileSyncer<TClient: ZoteroClient> {
    client: TClient,
    file_path: String,
}

impl<TClient: ZoteroClient> FileSyncer<TClient> {
    pub async fn try_new(
        client: TClient,
        file_path: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(&file_path)
            .await?;
        Ok(Self { client, file_path })
    }

    pub async fn sync(
        &self,
        interval: Option<Duration>,
        cancellation_token: CancellationToken,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut interval = tokio::time::interval(duration);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    log::info!("Starting scheduled sync.");
                    if let Err(e) = self.sync_once(cancellation_token.child_token()).await {
                        log::error!("Error during sync: {}", e);
                    }
                }
                _ = cancellation_token.cancelled() => {
                    log::info!("Cancellation requested, stopping periodic sync.");
                    break;
                }
            }
        }
        Ok(())
    }

    async fn sync_once(
        &self,
        cancellation_token: CancellationToken,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let items = self.client.fetch_items(cancellation_token).await?;
        log::trace!("Fetched items: {}", items);
        tokio::fs::write(&self.file_path, items).await?;
        log::info!("Successfully synced Zotero items to {}", self.file_path);
        Ok(())
    }
}
