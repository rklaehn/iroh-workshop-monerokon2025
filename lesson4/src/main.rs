use anyhow::{Context, Result};
use iroh::{Endpoint, protocol::{ProtocolHandler, Router}};
use iroh_base::ticket::NodeTicket;
use iroh_blobs::{protocol::GetRequest, ticket::BlobTicket};
use iroh_blobs::store::fs::FsStore;
use iroh_blobs::net_protocol::Blobs;
use std::{env, str::FromStr, path::PathBuf, sync::Arc};
use tracing::info;

mod util;

/// Server mode - shares a file
async fn share(path: PathBuf) -> Result<()> {
    // Always convert to absolute path
    let absolute_path = env::current_dir()?.join(path);
    
    // Get or generate a secret key
    let secret_key = util::get_or_generate_secret_key()?;
    
    // Create a blob store
    let blobs = FsStore::load("send.db").await?;
    
    // Create an endpoint and print the node ID
    let ep = Endpoint::builder()
        .alpns(vec![iroh_blobs::ALPN.to_vec()])
        .secret_key(secret_key)
        .bind()
        .await?;
    
    let node_id = ep.node_id();
    let addr = ep.node_addr().await?;
    let ticket = NodeTicket::from(addr.clone());
    
    println!("Node ID: {}", node_id);
    println!("Full address: {:?}", addr);
    println!("Ticket: {}", ticket);
    println!("To receive, use: {} <target> {}", env::args().next().unwrap_or_default(), ticket);
    println!("Sharing file: {}", absolute_path.display());
    
    let tag = blobs.add_path(absolute_path).await?;
    let ticket = BlobTicket::new(addr, *tag.hash(), tag.format());
    println!("Hash: {}", tag.hash());
    println!("Ticket: {}", ticket);
    
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
async fn receive(target: PathBuf, addr_str: &str) -> Result<()> {
    // Parse the address using NodeTicket
    let ticket = BlobTicket::from_str(addr_str).context("invalid address")?;
    
    // Convert target path to absolute
    let target = env::current_dir()?.join(target);
    
    info!("Connecting to: {:?}", ticket.node_addr());
    
    // Create a blob store
    let blobs = FsStore::load("recv.db").await?;
    
    // Create an endpoint
    let ep = Endpoint::builder()
        .alpns(vec![iroh_blobs::ALPN.to_vec()])
        .bind()
        .await?;
    
    // Connect to the node
    info!("Connecting to: {:?}", ticket.node_addr());
    let conn = ep.connect(ticket.node_addr().clone(), iroh_blobs::ALPN).await?;
    info!("Getting blob");
    let stats = blobs.remote().fetch(conn, ticket.clone(), None).await?;
    info!("Exporting file");
    let size = blobs.export(ticket.hash(), target.clone()).await?;
    info!("Exported file to {} with size: {}", target.display(), size);
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with default configuration
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();

    if args.len() == 2 {
        // Server mode - share a file
        let path = PathBuf::from(&args[1]);
        share(path).await
    } else if args.len() >= 3 {
        // Client mode - receive a file
        let target = PathBuf::from(&args[1]);
        let addr_str = &args[2];
        receive(target, addr_str).await
    } else {
        println!("Usage:");
        println!("  Share mode: {} <file_path>", args[0]);
        println!("  Receive mode: {} <target> <address>", args[0]);
        Ok(())
    }
} 