mod zotero_client;

use clap::Parser;

use crate::zotero_client::ReqwestZoteroClient;
use crate::zotero_client::ZoteroClient;

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

    /// Output file (if not provided, prints to stdout)
    #[arg(short, long)]
    file: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

    let client = ReqwestZoteroClient::new(args.user_id, args.api_key);

    match client.fetch_items().await {
        Ok(items) => {
            if let Some(file) = args.file {
                std::fs::write(file, items).unwrap();
            } else {
                println!("{}", items);
            }
            log::info!("Successfully fetched Zotero items.");
        }
        Err(e) => {
            log::error!("Error fetching items: {}", e);
        }
    }

    Ok(())
}
