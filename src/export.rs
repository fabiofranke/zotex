use std::time::Duration;

use crate::zotero_api::{
    client::ZoteroClient,
    types::{ApiError, FetchItemsParams, FetchItemsResponse},
};
use tokio::fs::OpenOptions;
use tokio::io::AsyncBufReadExt;
use tokio_util::sync::CancellationToken;

pub struct FileExporter<TClient: ZoteroClient> {
    client: TClient,
    file_path: String,
}

impl<TClient: ZoteroClient> FileExporter<TClient> {
    pub async fn try_new(client: TClient, file_path: String) -> Result<Self, ExportError> {
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
        Ok(Self { client, file_path })
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
                self.export_once(cancellation_token).await
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
                    match self.export_once(cancellation_token.child_token()).await {
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

    async fn export_once(
        &self,
        cancellation_token: CancellationToken,
    ) -> Result<ExportSuccess, ExportError> {
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
                Ok(ExportSuccess::NoChanges)
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
