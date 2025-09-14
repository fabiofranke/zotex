use std::time::Duration;

use crate::zotero_api::ExportFormat;
use crate::zotero_api::{ApiError, FetchItemsParams, FetchItemsResponse, client::ZoteroClient};
use serde::{Deserialize, Serialize};
use tokio::fs::OpenOptions;
use tokio::io::AsyncBufReadExt;
use tokio_util::sync::CancellationToken;

pub struct FileExporter<TClient: ZoteroClient> {
    client: TClient,
    file_path: String,
    format: ExportFormat,
}

impl<TClient: ZoteroClient> FileExporter<TClient> {
    pub async fn try_new(
        client: TClient,
        file_path: String,
        format: ExportFormat,
    ) -> Result<Self, ExportError> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&file_path)
            .await
            .map_err(|e| ExportError::FileError {
                file_path: file_path.clone(),
                io_error: e,
            })?;
        Ok(Self {
            client,
            file_path,
            format,
        })
    }

    pub async fn export(
        &self,
        interval: Option<Duration>,
        cancellation_token: CancellationToken,
    ) -> Result<ExportSuccess, ExportError> {
        match interval {
            Some(duration) if (duration.as_secs() > 0) => {
                log::info!(
                    "Starting periodic export every {} seconds.",
                    duration.as_secs()
                );
                self.export_periodically(duration, cancellation_token).await
            }
            _ => {
                log::info!("Starting one-time export.");
                self.export_once().await
            }
        }
    }

    async fn export_periodically(
        &self,
        duration: Duration,
        cancellation_token: CancellationToken,
    ) -> Result<ExportSuccess, ExportError> {
        let mut interval = tokio::time::interval(duration);
        let mut has_changes = false;
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    log::info!("Starting scheduled export.");
                    match self.export_once().await {
                        Ok(ExportSuccess::Changes) => {
                            has_changes = true;
                        }
                        Ok(ExportSuccess::NoChanges) => {
                            // nothing to do
                        }
                        Err(e) => {
                            log::error!("Aborting periodic export due to error: {}", e);
                            return Err(e);
                        }
                    }
                }
                _ = cancellation_token.cancelled() => {
                    log::info!("Cancellation requested, stopping periodic export.");
                    break;
                }
            }
        }
        Ok(if has_changes {
            ExportSuccess::Changes
        } else {
            ExportSuccess::NoChanges
        })
    }

    async fn export_once(&self) -> Result<ExportSuccess, ExportError> {
        let header = self.try_read_file_headline().await;
        if let Some(h) = &header {
            log::info!("Found existing export with version {}", h.library_version);
        } else {
            log::info!("No existing export found, performing full fetch.");
        }
        let params = FetchItemsParams {
            last_modified_version: header.map(|h| h.library_version),
            format: self.format.clone(),
        };
        let response = self.client.fetch_items(&params).await?;
        match response {
            FetchItemsResponse::UpToDate => {
                log::info!(
                    "File '{}' is up to date with the Zotero library.",
                    &self.file_path
                );
                Ok(ExportSuccess::NoChanges)
            }
            FetchItemsResponse::Updated {
                last_modified_version,
                text: items,
            } => {
                let header = FileMetadata {
                    zotex_version: clap::crate_version!().to_owned(),
                    library_version: last_modified_version,
                    format: self.format.clone(),
                };
                let file_content = format!("{}\n{}", String::from(header), items);
                tokio::fs::write(&self.file_path, file_content)
                    .await
                    .map_err(|e| ExportError::FileError {
                        file_path: self.file_path.clone(),
                        io_error: e,
                    })?;
                log::info!(
                    "Wrote library export with version {} to file '{}'.",
                    last_modified_version,
                    &self.file_path
                );
                Ok(ExportSuccess::Changes)
            }
        }
    }

    async fn try_read_file_headline(&self) -> Option<FileMetadata> {
        let file = OpenOptions::new()
            .read(true)
            .open(&self.file_path)
            .await
            .ok()?;
        let mut reader = tokio::io::BufReader::new(file);
        let mut first_line = String::new();
        reader.read_line(&mut first_line).await.ok()?;
        FileMetadata::try_from(first_line.trim()).ok()
    }
}

pub enum ExportSuccess {
    Changes,
    NoChanges,
}

#[derive(thiserror::Error, Debug)]
pub enum ExportError {
    #[error("Error with file '{file_path}'")]
    FileError {
        file_path: String,
        #[source]
        io_error: std::io::Error,
    },
    #[error("Error in Zotero client")]
    ClientError(#[from] ApiError),
}

#[derive(Serialize, Deserialize)]
struct FileMetadata {
    zotex_version: String,
    library_version: u64,
    format: ExportFormat,
}

impl FileMetadata {
    const PREFIX: &'static str = "% *** THIS FILE WAS AUTO-GENERATED BY ZOTEX - DO NOT EDIT ***";
}

impl From<FileMetadata> for String {
    fn from(headline: FileMetadata) -> Self {
        format!(
            "{} {}",
            FileMetadata::PREFIX,
            serde_json::to_string(&headline).unwrap_or_default()
        )
    }
}

impl TryFrom<&str> for FileMetadata {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if !value.starts_with(Self::PREFIX) {
            return Err(());
        }
        let without_prefix = value.trim_start_matches(Self::PREFIX).trim();
        serde_json::from_str(without_prefix).map_err(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_headline_string_conversion() {
        let headline = FileMetadata {
            zotex_version: "0.1.0".to_owned(),
            library_version: 12345,
            format: Default::default(),
        };
        let headline_str: String = headline.into();

        let parsed_headline = FileMetadata::try_from(headline_str.as_str());
        assert!(parsed_headline.is_ok());
        let parsed_headline = parsed_headline.unwrap();
        assert_eq!(parsed_headline.library_version, 12345);
    }
}
