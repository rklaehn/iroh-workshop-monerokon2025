use std::{env, process, str::FromStr};

use anyhow::{Context, Result};
use iroh::{discovery, protocol::Router, Endpoint, NodeAddr};
use iroh_base::ticket::NodeTicket;
use tokio::signal;
use tracing::info;
use util::z32_node_id;

mod echo;
mod util;

/// Server mode - accepts connections and echoes messages back
async fn accept() -> Result<()> {
    // Get or generate a secret key
    let secret_key = util::get_or_generate_secret_key()?;

    // Create an endpoint and print the node ID
    let ep = Endpoint::builder()
        .secret_key(secret_key)
        .discovery_n0()
        .discovery_dht()
        .bind()
        .await?;

    let node_id = ep.node_id();
    let addr = ep.node_addr().await?;
    let ticket = NodeTicket::from(addr.clone());
    let ticket_short = NodeTicket::from(NodeAddr::from(addr.node_id));

    println!("Node ID: {}", node_id);
    println!("Full address: {:?}", addr);
    println!("Ticket: {}", ticket);
    println!("Short ticket: {}", ticket_short);
    println!(
        "To connect, use: {} connect <message> {}",
        env::args().next().unwrap_or_default(),
        ticket
    );
    println!("To see the info published on DNS, run:");
    println!(
        "dig TXT @dns.iroh.link _iroh.{}.{}",
        z32_node_id(&addr.node_id),
        "dns.iroh.link"
    );
    println!("To see the info published on the mainline DHT, open:");
    println!("https://app.pkarr.org/?pk={}", z32_node_id(&addr.node_id));

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
    //
    // only resolve discovery, don't publish
    let ep = Endpoint::builder()
        .add_discovery(|_| Some(discovery::pkarr::PkarrResolver::n0_dns()))
        .add_discovery(|_| discovery::pkarr::dht::DhtDiscovery::builder().build().ok())
        .bind()
        .await?;

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
