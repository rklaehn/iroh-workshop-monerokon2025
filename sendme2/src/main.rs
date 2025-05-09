use std::{env, ops::Deref, path::PathBuf, process, str::FromStr};

use anyhow::{ensure, Context, Result};
use iroh::{protocol::Router, Endpoint};
use iroh_blobs::{format::collection::Collection, net_protocol::Blobs, store::fs::FsStore, ticket::BlobTicket};
use tracing::info;
use util::create_send_dir;

mod util;

/// Server mode - shares a file
async fn share(path: PathBuf) -> Result<()> {
    // Always convert to absolute path
    let absolute_path = env::current_dir()?.join(path);

    ensure!(
        absolute_path.exists(),
        "File does not exist: {}",
        absolute_path.display()
    );
    ensure!(
        absolute_path.is_dir(),
        "Not a file: {}",
        absolute_path.display()
    );

    // Get or generate a secret key
    let secret_key = util::get_or_generate_secret_key()?;

    // Create a blob store
    let blobs = FsStore::load(create_send_dir(".sendme2-send")?).await?;

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
    println!("Hash: {}", tag.hash());
    println!(
        "To receive, use: {} receive {}",
        env::args().next().unwrap_or_default(),
        ticket
    );
    println!("Sharing file: {}", absolute_path.display());

    // Create a router with the endpoint
    let router = Router::builder(ep.clone())
        .accept(iroh_blobs::ALPN, Blobs::new(&blobs, ep.clone(), None))
        .spawn()
        .await?;

    println!("Server is running. Press Ctrl+C to stop...");

    // Wait for Ctrl-C
    tokio::signal::ctrl_c().await?;
    println!("\nReceived Ctrl+C, shutting down...");

    // Gracefully shut down the router
    router.shutdown().await?;

    Ok(())
}

/// Client mode - receives a file
async fn receive(addr_str: &str) -> Result<()> {
    // Parse the address using NodeTicket
    let ticket = BlobTicket::from_str(addr_str).context("invalid address")?;

    info!("Connecting to: {:?}", ticket.node_addr());

    // Create a blob store
    let store = FsStore::load(format!(".sendme2-recv-{}", ticket.hash())).await?;

    // Create an endpoint
    let ep = Endpoint::builder()
        .alpns(vec![iroh_blobs::ALPN.to_vec()])
        .bind()
        .await?;

    // Connect to the node
    info!("Connecting to: {:?}", ticket.node_addr());
    let conn = ep
        .connect(ticket.node_addr().clone(), iroh_blobs::ALPN)
        .await?;
    info!("Getting blob");
    let stats = store.remote().fetch(conn, ticket.clone(), iroh_blobs::util::sink::Drain).await?;
    println!("Transfer stats: {:?}", stats);
    info!("Exporting file");
    let collection = Collection::load(ticket.hash(), store.deref()).await?;
    util::export(&store, collection).await?;

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
            // Server mode - share a file
            let path = PathBuf::from(&args[2]);
            share(path).await
        }
        "receive" | "recv" if args.len() == 3 => {
            // Client mode - receive a file
            let addr_str = &args[2];
            receive(addr_str).await
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

