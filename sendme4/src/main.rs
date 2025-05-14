use std::{env, ops::Deref, path::PathBuf, process, str::FromStr, time::Duration};

use anyhow::{ensure, Context, Result};
use futures::StreamExt;
use iroh::{discovery, protocol::Router, Endpoint, NodeId, SecretKey};
use iroh_blobs::{
    api::{
        blobs,
        downloader::{DownloadOptions, SplitStrategy},
    },
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
    // Always convert to absolute path
    let absolute_path = env::current_dir()?.join(path);

    ensure!(
        absolute_path.exists(),
        "File does not exist: {}",
        absolute_path.display()
    );
    ensure!(
        absolute_path.is_dir() || absolute_path.is_file(),
        "Not a file directory: {}",
        absolute_path.display()
    );

    // Get or generate a secret key
    let secret_key = util::get_or_generate_secret_key()?;

    // Create a blob store
    let blobs_path = create_send_dir()?;
    let blobs = FsStore::load(&blobs_path).await?;

    // Create an endpoint and print the node ID
    let ep = Endpoint::builder()
        .discovery_n0()
        .discovery_dht()
        .secret_key(secret_key.clone())
        .bind()
        .await?;

    let node_id = ep.node_id();
    let addr = ep.node_addr().await?;

    println!("Node ID: {}", node_id);
    println!("Full address: {:?}", addr);

    let tag = util::import(absolute_path.clone(), &blobs).await?;
    let announce_task = tokio::spawn(announce_task(
        *tag.hash_and_format(),
        ep.clone(),
        secret_key,
    ));
    let ticket = BlobTicket::new(addr, *tag.hash(), tag.format());
    println!("Sharing {}", absolute_path.display());
    println!("Hash: {}", tag.hash());
    println!(
        "To receive, use: {} receive {}",
        env::args().next().unwrap_or_default(),
        ticket.hash_and_format(),
    );
    println!();

    let (dump_task, dump_sender) = util::dump_provider_events();

    // Create a router with the endpoint
    let router = Router::builder(ep.clone())
        .accept(
            iroh_blobs::ALPN,
            Blobs::new(&blobs, ep.clone(), Some(dump_sender)),
        )
        .spawn()
        .await?;

    println!("Server is running. Press Ctrl+C to stop...");

    // Wait for Ctrl-C
    tokio::signal::ctrl_c().await?;
    println!("\nReceived Ctrl+C, shutting down...");

    // Gracefully shut down the router
    router.shutdown().await?;

    // Abort the dump task
    dump_task.abort();
    announce_task.abort();

    // Remove the blobs directory
    tokio::fs::remove_dir_all(blobs_path).await?;

    Ok(())
}

/// Client mode - receives a file
async fn receive(content: &str) -> Result<()> {
    let content = HashAndFormat::from_str(content).context("invalid content")?;

    // Create a blob store
    let blobs_path = create_recv_dir(content)?;
    let store = FsStore::load(&blobs_path).await?;

    // Create an endpoint
    let ep = Endpoint::builder()
        .add_discovery(|_| Some(discovery::pkarr::PkarrResolver::n0_dns()))
        .add_discovery(|_| discovery::pkarr::dht::DhtDiscovery::builder().build().ok())
        .bind()
        .await?;

    // Connect to the node
    let downloader = store.downloader(&ep);
    info!("Getting hash sequence");
    let options = DownloadOptions::new(
        content,
        TrackerDiscovery::new(ep.clone(), TRACKER.parse()?),
        SplitStrategy::None,
    );
    // let mut stream = downloader.download(content, nodes).stream().await?;
    let mut stream = downloader.download_with_opts(options).stream().await?;
    while let Some(item) = stream.next().await {
        println!("Received: {:?}", item);
    }
    info!("Exporting file");
    let collection = Collection::load(content.hash, store.deref()).await?;
    util::export(&store, collection).await?;

    // close the endpoint, just to be nice
    ep.close().await;
    // shutdown the store to sync to disk
    store.shutdown().await?;
    // Remove the blobs directory
    tokio::fs::remove_dir_all(blobs_path).await?;
    Ok(())
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
