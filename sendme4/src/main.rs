use std::{env, ops::Deref, path::PathBuf, process, str::FromStr, time::Duration};

use anyhow::{ensure, Context, Result};
use futures::StreamExt;
use iroh::{discovery, protocol::Router, Endpoint, NodeId, SecretKey};
use iroh_blobs::{
    api::downloader::{DownloadOptions, SplitStrategy},
    format::collection::Collection,
    net_protocol::Blobs,
    store::fs::FsStore,
    ticket::BlobTicket,
    HashAndFormat,
};
use iroh_mainline_content_discovery::protocol::{
    AbsoluteTime, Announce, AnnounceKind, SignedAnnounce,
};
use tracing::{info, trace, warn};
use util::{create_recv_dir, create_send_dir, TrackerDiscovery};

mod util;

/// node ticket for the tracker
/// local
/// const TRACKER: &str = "b223f67b76e1853c7f76d9a9f8ce4d8dbb04a48ad9631ce52347043388475767";
/// arqu
const TRACKER: &str = "69b2f535d5792b50599b51990963e0cca1041679cd968563a8bc3179a7c42e67";

/// periodically announce the content to the tracker
async fn announce_task(content: HashAndFormat, ep: Endpoint, secret_key: SecretKey) -> Result<()> {
    let tracker: NodeId = TRACKER.parse()?;
    let content = content.to_string();
    loop {
        let announce = Announce {
            host: ep.node_id(),
            kind: AnnounceKind::Complete,
            content: FromStr::from_str(&content).unwrap(),
            timestamp: AbsoluteTime::now(),
        };
        let signed_announce = SignedAnnounce::new(announce, &secret_key)?;
        println!("Connecting to tracker: {}", tracker);
        let Ok(connection) = ep
            .connect(tracker, iroh_mainline_content_discovery::protocol::ALPN)
            .await
        else {
            warn!("Failed to connect to tracker");
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        };
        println!("Announcing: {:?}", signed_announce);
        if let Err(cause) =
            iroh_mainline_content_discovery::announce_iroh(connection, signed_announce).await
        {
            warn!("Failed to send announce {:?}", cause);
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        };
        trace!("Sleeping until next announce");
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

/// Server mode - shares a file or directory
async fn share(path: PathBuf) -> Result<()> {
    // like share3, except we also spawn a task to announce the content
    // to the tracker, using the announce_task function
    // also print the content HashAndFormat, not a ticket
    todo!();
}

/// Client mode - receives a file
async fn receive(content: &str) -> Result<()> {
    // like share3, except that we use the TrackerDiscovery
    // to discover the content, instead of using the tickets
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
        "receive" | "recv" if args.len() == 3 => {
            // Client mode - receive a file or directory
            let content = &args[2];
            receive(content).await
        }
        _ => {
            println!("Usage: sendme4 <command> [args]");
            println!("Commands:");
            println!("  share <dir_path>   Share a directory");
            println!("  receive <hash>     Receive a directory");
            process::exit(1);
        }
    }
}
