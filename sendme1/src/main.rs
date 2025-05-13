use std::{env, path::PathBuf, process, str::FromStr};

use anyhow::{ensure, Context, Result};
use iroh::{protocol::Router, Endpoint};
use iroh_blobs::{net_protocol::Blobs, store::fs::FsStore, ticket::BlobTicket, util::sink};
use tracing::info;
use util::{crate_name, create_recv_dir, create_send_dir};

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
        absolute_path.is_file(),
        "Not a file: {}",
        absolute_path.display()
    );

    // Get or generate a secret key
    let secret_key = util::get_or_generate_secret_key()?;

    // Create a blob store
    let blobs = FsStore::load(create_send_dir()?).await?;

    // Create an endpoint and print the node ID
    let ep = Endpoint::builder()
        .secret_key(secret_key)
        .bind()
        .await?;

    let node_id = ep.node_id();
    let addr = ep.node_addr().await?;

    println!("Node ID: {}", node_id);
    println!("Full address: {:?}", addr);

    let tag = blobs.add_path(&absolute_path).await?;
    let ticket = BlobTicket::new(addr, *tag.hash(), tag.format());
    println!("Sharing file: {}", absolute_path.display());
    println!("Hash: {}", tag.hash());
    println!(
        "To receive, use: {} receive <target> {}",
        env::args().next().unwrap_or_default(),
        ticket
    );
    println!();

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
async fn receive(target: &str, ticket: &str) -> Result<()> {
    let target = PathBuf::from(target);
    // Parse the address using NodeTicket
    let ticket = BlobTicket::from_str(ticket).context("invalid address")?;

    // Convert target path to absolute
    let target = env::current_dir()?.join(target);

    info!("Connecting to: {:?}", ticket.node_addr());

    // Create a blob store
    let store = FsStore::load(create_recv_dir(ticket.hash_and_format())?).await?;

    // Create an endpoint
    let ep = Endpoint::builder().bind().await?;

    // Connect to the node
    info!("Connecting to: {:?}", ticket.node_addr());
    let conn = ep
        .connect(ticket.node_addr().clone(), iroh_blobs::ALPN)
        .await?;
    info!("Getting blob");
    let stats = store
        .remote()
        .fetch(conn, ticket.clone(), sink::Drain)
        .await?;
    info!("Exporting file");
    let size = store.export(ticket.hash(), target.clone()).await?;
    info!("Exported file to {} with size: {}", target.display(), size);
    println!("Transfer stats: {:?}", stats);

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
        "receive" | "recv" if args.len() == 4 => {
            // Client mode - receive a file or directory
            let path = &args[2];
            let ticket = &args[3];
            receive(path, ticket).await
        }
        _ => {
            println!("Usage: {} <command> [args]", crate_name());
            println!("Commands:");
            println!("  share <file_path>             Share a file");
            println!("  receive <file_path> <ticket>  Receive a directory");
            process::exit(1);
        }
    }
}
