use std::{env, process, str::FromStr};

use anyhow::{Context, Result};
use iroh::Endpoint;
use iroh_base::ticket::NodeTicket;
use tracing::info;

mod util;

/// The ALPN protocol identifier for the echo service
const ECHO_ALPN: &[u8] = b"ECHO";

/// Server mode - accepts connections and echoes messages back
async fn accept() -> Result<()> {
    // create an iroh endpoint, accepting the echo protocol
    // print a node ticket so the client can connect
    // accept a single connection
    // accept a single bidi stream
    // read the message from the stream and echo it back
    // close the connection
    todo!();
}

/// Client mode - connects to a server and sends a message
async fn connect(message: &str, ticket: &str) -> Result<()> {
    // create an iroh endpoint
    // parse the ticket
    // open a connection to the server using the echo protocol
    // open a bidi stream to the server
    // send the message to the server
    // read the response from the server and print it
    // close the connection
    todo!();
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with default configuration
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|x| x.to_lowercase()).unwrap_or_default();
    match cmd.as_str() {
        "accept" if args.len() == 2 => {
            // Server mode - accept connections
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
