mod file_syncer;
mod zotero_api;

use crate::file_syncer::FileSyncer;
use crate::zotero_api::client::ReqwestZoteroClient;
use anyhow::Context;
use clap::Parser;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Zotero User ID
    #[arg(short, long)]
    user_id: String,

    /// Zotero API Key
    #[arg(short, long)]
    api_key: String,

    /// File that the library will be exported to
    #[arg(short, long)]
    file: String,

    /// Interval (in seconds) for periodic exports. If not provided, the program will exit after exporting once
    #[arg(short, long)]
    interval: Option<u64>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    let client = ReqwestZoteroClient::new(args.user_id, args.api_key);
    let syncer = FileSyncer::try_new(client, args.file.clone())
        .await
        .with_context(|| "Error during file syncer initialization. Please ensure the file path is valid, the directory exists and is accessible.")?;

    let cancellation_token = CancellationToken::new();
    tokio::select! {
        result = syncer.sync(args.interval.map(Duration::from_secs), cancellation_token.child_token()) => {
            if let Err(e) = result {
                return Err(e).with_context(|| "Error during synchronization process.");
            }
        }
        _ = tokio::signal::ctrl_c() => {
            log::info!("Signal received, cancelling...");
            cancellation_token.cancel();
        }
    }

    Ok(())
}
