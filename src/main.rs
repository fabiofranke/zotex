mod export;
mod zotero_api;

use crate::export::{ExportTrigger, FileExporter};
use crate::zotero_api::ExportFormat;
use crate::zotero_api::api_key::ApiKey;
use crate::zotero_api::builder::ZoteroClientBuilder;
use crate::zotero_api::client::ZoteroClient;
use anyhow::Context;
use clap::Parser;
use tokio_util::sync::CancellationToken;

const ZOTEXON_VERSION: &str = clap::crate_version!();

#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
struct Args {
    /// Zotero API Key with read access to your library. Generate a key in your Zotero settings: https://www.zotero.org/settings/keys/new
    #[arg(long)]
    api_key: String,

    /// File that the library will be exported to
    #[arg(long)]
    file: String,

    /// Format to be used for the export
    #[arg(long, default_value_t, value_enum)]
    format: ExportFormat,

    /// Let the program listen for changes in the Zotero library and automatically export on every change. Program will run until interrupted (e.g. with Ctrl+C).
    #[arg(long)]
    sync: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    let api_key = ApiKey(args.api_key);
    let client = ZoteroClientBuilder::new(api_key.clone())
        .build()
        .await
        .with_context(|| "Error during Zotero client initialization.")?;
    let cancellation_token = CancellationToken::new();
    let trigger = if args.sync {
        ExportTrigger::websocket(api_key, client.user_id(), cancellation_token.child_token())
            .await
            .with_context(|| "Error during WebSocket trigger initialization.")?
    } else {
        ExportTrigger::none()
    };
    let exporter = FileExporter::try_new(client, args.file.clone(), args.format.clone(), trigger)
        .await
        .with_context(|| "Error during file exporter initialization. Please ensure the file path is valid, the directory exists and is accessible.")?;

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for signal");
        log::info!("Signal received, cancelling...");
        cancellation_token.cancel();
    });

    exporter
        .run()
        .await
        .map(|_| ())
        .with_context(|| "Error during export process.")
}
