use std::{collections::BTreeSet, env, ops::Deref, path::PathBuf, process, str::FromStr};

use anyhow::{ensure, Context, Result};
use futures::StreamExt;
use iroh::{protocol::Router, Endpoint};
use iroh_blobs::{
    api::downloader::{DownloadOptions, Shuffled, SplitStrategy},
    format::collection::Collection,
    net_protocol::Blobs,
    store::fs::FsStore,
    ticket::BlobTicket,
};
use tracing::info;
use util::{create_recv_dir, create_send_dir};

mod util;

/// Server mode - shares a file or directory
async fn share(path: PathBuf) -> Result<()> {
    // like sendme2, except we use util::dump_provider_events to dump events
    // whenever we get a request
    todo!();
}

/// Client mode - receives a file
async fn receive(tickets: Vec<String>) -> Result<()> {
    // parse multiple tickets
    // check that they are all for the same content
    // add node info from tickets to the endpoint
    // create a blob store
    // create an endpoint
    // create a downloader
    // use the downloader to download the content from
    // all node ids at once.
    todo!();
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with default configuration
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|x| x.to_lowercase()).unwrap_or_default();
    match cmd.as_str() {
        "share" if args.len() == 3 => {
            // Server mode - share a file or directory
            let path = PathBuf::from(&args[2]);
            share(path).await
        }
        "receive" | "recv" if args.len() >= 3 => {
            // Client mode - receive a file or directory
            let tickets = args.iter().skip(2).cloned().collect::<Vec<_>>();
            receive(tickets).await
        }
        _ => {
            println!("Usage: sendme2 <command> [args]");
            println!("Commands:");
            println!("  share <dir_path>   Share a directory");
            println!("  receive <ticket>   Receive a directory");
            process::exit(1);
        }
    }
}
