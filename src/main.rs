mod export;
mod zotero_api;

use crate::export::FileExporter;
use crate::zotero_api::ExportFormat;
use crate::zotero_api::api_key::ApiKey;
use crate::zotero_api::builder::ZoteroClientBuilder;
use anyhow::Context;
use clap::Parser;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

const ZOTEXON_VERSION: &str = clap::crate_version!();

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Zotero API Key with read access to your library. Generate a key in your Zotero settings: https://www.zotero.org/settings/keys/new
    #[arg(long)]
    api_key: String,

    /// File that the library will be exported to
    #[arg(long)]
    file: String,

    /// Interval (in seconds) for periodic exports. If not provided, the program will exit after exporting once
    #[arg(long)]
    interval: Option<u64>,

    /// Format to be used for the export
    #[arg(long, default_value_t, value_enum)]
    format: ExportFormat,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    let client = ZoteroClientBuilder::new(ApiKey(args.api_key))
        .build()
        .await
        .with_context(|| "Error during Zotero client initialization.")?;
    let exporter = FileExporter::try_new(client, args.file.clone(), args.format.clone())
        .await
        .with_context(|| "Error during file exporter initialization. Please ensure the file path is valid, the directory exists and is accessible.")?;

    let cancellation_token = CancellationToken::new();
    let child_token = cancellation_token.child_token();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for signal");
        log::info!("Signal received, cancelling...");
        cancellation_token.cancel();
    });

    exporter
        .export(args.interval.map(Duration::from_secs), child_token)
        .await
        .map(|_| ())
        .with_context(|| "Error during export process.")
}
