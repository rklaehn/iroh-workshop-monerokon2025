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
    // create an iroh endpoint
    // create an iroh router
    // configure the router to accept the echo protocol handler
    // spawn the router
    // print a node ticket so the client can connect
    // wait until control-c
    todo!();
}

/// Client mode - connects to a server and sends a message
async fn connect(message: &str, ticket: &str) -> Result<()> {
    // like echo1
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
