mod file_syncer;
mod zotero_client;

use clap::Parser;

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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

    let client = ReqwestZoteroClient::new(args.user_id, args.api_key);
    let syncer = FileSyncer::try_new(client, args.file.clone()).await?;

    if let Err(e) = syncer.sync().await {
        log::error!("Error syncing Zotero items: {}", e);
    }

    Ok(())
}
