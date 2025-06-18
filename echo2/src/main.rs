use std::{env, process, str::FromStr};

use anyhow::{Context, Result};
use iroh::{protocol::Router, Endpoint};
use iroh_base::ticket::NodeTicket;
use tokio::signal;
use tracing::info;

mod echo;
mod util;

/// Server mode - accepts connections and echoes messages back
async fn accept() -> Result<()> {
    // Get or generate a secret key
    let secret_key = util::get_or_generate_secret_key()?;

    // Create an endpoint and print the node ID
    let ep = Endpoint::builder()
        .alpns(vec![echo::ECHO_ALPN.to_vec()])
        .secret_key(secret_key)
        .bind()
        .await?;

    let node_id = ep.node_id();
    let addr = ep.node_addr().await?;
    let ticket = NodeTicket::from(addr.clone());

    println!("Node ID: {}", node_id);
    println!("Full address: {:?}", addr);
    println!("Ticket: {}", ticket);
    println!(
        "To connect, use: {} connect <message> {}",
        env::args().next().unwrap_or_default(),
        ticket
    );
    println!("inspect ticket at https://ticket.iroh.computer/\n");

    // Create a router with the endpoint
    let router = Router::builder(ep)
        .accept(echo::ECHO_ALPN, echo::EchoProtocol)
        .spawn();

    println!("Server is running. Press Ctrl+C to stop...");

    // Wait for Ctrl-C
    signal::ctrl_c().await?;
    println!("\nReceived Ctrl+C, shutting down...");

    // Gracefully shut down the router
    router.shutdown().await?;

    Ok(())
}

/// Client mode - connects to a server and sends a message
async fn connect(message: &str, ticket: &str) -> Result<()> {
    // Parse the address using NodeTicket
    let ticket = NodeTicket::from_str(ticket).context("invalid address")?;

    info!("Connecting to: {:?}", ticket.node_addr());

    // Create an endpoint
    let ep = Endpoint::builder().bind().await?;

    // Connect to the node
    let conn = ep.connect(ticket, echo::ECHO_ALPN).await?;
    info!("Connected");

    // Open a bi-directional stream
    let (mut send_stream, mut recv_stream) = conn.open_bi().await?;

    // Send the message
    info!("Sending message: {}", message);
    send_stream.write_all(message.as_bytes()).await?;
    send_stream.finish()?;

    // Wait for the response
    let res = recv_stream.read_to_end(1024).await?;
    println!("Received response: {}", String::from_utf8_lossy(&res));

    // Close the connection
    conn.close(0u8.into(), b"done");

    // Wait for the connection to close
    conn.closed().await;
    info!("Connection closed");

    // Close the endpoint
    ep.close().await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with default configuration
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|x| x.to_lowercase()).unwrap_or_default();
    match cmd.as_str() {
        "accept" if args.len() == 2 => {
            // server mode - accept connections
            accept().await
        }
        "connect" if args.len() == 4 => {
            // Client mode - connect to a server and send a message
            let message = &args[2];
            let ticket = &args[3];
            connect(message, ticket).await
        }
        _ => {
            println!("Usage: echo1 <command> [args]");
            println!("Commands:");
            println!("  accept                       Listen for echo requests");
            println!("  connect <message> <ticket>   Connect to an echo server and send a message");
            process::exit(1);
        }
    }
}
