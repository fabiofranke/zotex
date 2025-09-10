use clap::Parser;

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
    let args = Args::parse();
    let client = reqwest::Client::new();

    let resp = client
        .get(format!(
            "https://api.zotero.org/users/{}/items?format=biblatex",
            args.user_id
        ))
        .header("Zotero-API-Version", "3")
        .header("Zotero-API-Key", args.api_key)
        .send()
        .await?;

    if resp.status() == reqwest::StatusCode::OK {
        let body = resp.text().await?;
        if let Some(file) = args.file {
            std::fs::write(file, body)?;
        } else {
            println!("{}", body);
        }
        println!("Successfully fetched Zotero items.");
    } else {
        eprintln!("Error: Received status code {}", resp.status());
    }
    Ok(())
}
