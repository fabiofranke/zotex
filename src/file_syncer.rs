use tokio::fs::OpenOptions;

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

    pub async fn sync(&self) -> Result<(), Box<dyn std::error::Error>> {
        let items = self.client.fetch_items().await?;
        log::trace!("Fetched items: {}", items);
        tokio::fs::write(&self.file_path, items).await?;
        log::info!("Successfully synced Zotero items to {}", self.file_path);
        Ok(())
    }
}
