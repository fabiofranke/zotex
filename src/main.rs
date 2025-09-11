mod file_syncer;
mod zotero_client;

use std::time::Duration;

use clap::Parser;
use tokio_util::sync::CancellationToken;

use crate::file_syncer::FileSyncer;
use crate::zotero_client::ReqwestZoteroClient;

/// Simple program to fetch Zotero items in BibLaTeX format.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Zotero User ID
    #[arg(short, long)]
    user_id: String,

    /// Zotero API Key
    #[arg(short, long)]
    api_key: String,

    /// File that the library will be written to.
    #[arg(short, long)]
    file: String,

    /// Interval (in seconds) between syncs. If not provided, the program will run once and exit.
    #[arg(short, long)]
    interval: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

    let client = ReqwestZoteroClient::new(args.user_id, args.api_key);
    let syncer = FileSyncer::try_new(client, args.file.clone()).await?;

    let cancellation_token = CancellationToken::new();
    tokio::select! {
        result = syncer.sync(args.interval.map(Duration::from_secs), cancellation_token.child_token()) => {
            if let Err(e) = result {
                log::error!("Error during sync: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            log::info!("Signal received, cancelling...");
            cancellation_token.cancel();
        }
    }

    Ok(())
}
