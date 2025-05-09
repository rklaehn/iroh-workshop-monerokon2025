use anyhow::{Context, Result};
use iroh::Endpoint;
use iroh_base::ticket::NodeTicket;
use std::{env, str::FromStr};
use tracing::info;

mod util;

/// The ALPN protocol identifier for the echo service
const ECHO_ALPN: &[u8] = b"ECHO";

/// Server mode - accepts connections and echoes messages back
async fn accept() -> Result<()> {
    // Get or generate a secret key
    let secret_key = util::get_or_generate_secret_key()?;
    
    // Create an endpoint and print the node ID
    let ep = Endpoint::builder()
        .alpns(vec![ECHO_ALPN.to_vec()])
        .secret_key(secret_key)
        .bind()
        .await?;
    
    let node_id = ep.node_id();
    let addr = ep.node_addr().await?;
    let ticket = NodeTicket::from(addr.clone());
    
    println!("Node ID: {}", node_id);
    println!("Full address: {:?}", addr);
    println!("Ticket: {}", ticket);
    println!("To connect, use: {} <message> {}", env::args().next().unwrap_or_default(), ticket);
    
    // Accept a connection
    info!("Waiting for connections");
    let incoming = ep.accept().await.context("no incoming connection")?;
    let conn = incoming.await?;
    info!("Connection accepted");
    
    // Accept a bi-directional stream
    let (mut send_stream, mut recv_stream) = conn.accept_bi().await?;
    
    // Read the message
    let msg = recv_stream.read_to_end(1024).await?;
    info!("Received message: {}", String::from_utf8_lossy(&msg));
    
    // Echo the message back
    send_stream.write_all(&msg).await?;
    send_stream.finish()?;
    
    // Wait for the client to close the connection
    conn.closed().await;
    info!("Connection closed");
    
    // Close the endpoint
    ep.close().await;
    
    Ok(())
}

/// Client mode - connects to a server and sends a message
async fn connect(message: &str, addr_str: &str) -> Result<()> {
    // Parse the address using NodeTicket
    let ticket = NodeTicket::from_str(addr_str).context("invalid address")?;
    
    info!("Connecting to: {:?}", ticket.node_addr());
    
    // Create an endpoint
    let ep = Endpoint::builder().bind().await?;
    
    // Connect to the node
    let conn = ep.connect(ticket, ECHO_ALPN).await?;
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

    if args.len() == 1 {
        // Server mode
        accept().await
    } else if args.len() >= 3 {
        // Client mode - connect to the provided address and send a message
        let message = args[1..args.len()-1].join(" ");
        let addr_str = &args[args.len()-1];
        connect(&message, addr_str).await
    } else {
        println!("Usage:");
        println!("  Server mode: {}", args[0]);
        println!("  Client mode: {} <message> <address>", args[0]);
        Ok(())
    }
}
