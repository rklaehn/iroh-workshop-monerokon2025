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
    let blobs = FsStore::load(create_send_dir()?).await?;

    // Create an endpoint and print the node ID
    let ep = Endpoint::builder()
        .alpns(vec![iroh_blobs::ALPN.to_vec()])
        .secret_key(secret_key)
        .bind()
        .await?;

    let node_id = ep.node_id();
    let addr = ep.node_addr().await?;

    println!("Node ID: {}", node_id);
    println!("Full address: {:?}", addr);

    let tag = util::import(absolute_path.clone(), &blobs).await?;
    let ticket = BlobTicket::new(addr, *tag.hash(), tag.format());
    println!("Sharing {}", absolute_path.display());
    println!("Hash: {}", tag.hash());
    println!(
        "To receive, use: {} receive {}",
        env::args().next().unwrap_or_default(),
        ticket
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

    Ok(())
}

/// Client mode - receives a file
async fn receive(tickets: Vec<String>) -> Result<()> {
    // Parse the addresses using NodeTicket
    let tickets = tickets
        .iter()
        .map(|ticket| BlobTicket::from_str(ticket).context("invalid address"))
        .collect::<Result<Vec<_>>>()?;

    ensure!(!tickets.is_empty(), "No tickets provided");

    // get the content of the tickets. It must be the same for all tickets.
    let content = tickets
        .iter()
        .map(|ticket| ticket.hash_and_format())
        .collect::<BTreeSet<_>>();
    ensure!(
        content.len() == 1,
        "All tickets must be for the same content"
    );
    let content = content.into_iter().next().unwrap();

    // get the node addresses.
    let nodes = tickets
        .iter()
        .map(|ticket| ticket.node_addr().node_id)
        .collect::<BTreeSet<_>>();

    // Create a blob store
    let store = FsStore::load(create_recv_dir(content)?).await?;

    // Create an endpoint
    let ep = Endpoint::builder()
        .alpns(vec![iroh_blobs::ALPN.to_vec()])
        .bind()
        .await?;

    // add the connection information contained in the tickets to the endpoint
    for ticket in tickets {
        ep.add_node_addr(ticket.node_addr().clone())?;
    }

    // Connect to the node
    info!("Trying to get content from: {:?}", nodes);
    let downloader = store.downloader(&ep);
    info!("Getting hash sequence");
    let options = DownloadOptions::new(
        content,
        Shuffled::new(nodes.into_iter().collect()),
        SplitStrategy::Split,
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
