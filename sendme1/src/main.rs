use std::{env, path::PathBuf, process, str::FromStr};

use anyhow::{ensure, Context, Result};
use iroh::{protocol::Router, Endpoint};
use iroh_blobs::{net_protocol::Blobs, store::fs::FsStore, ticket::BlobTicket, util::sink};
use tracing::info;
use util::{crate_name, create_recv_dir, create_send_dir};

mod util;

/// Server mode - shares a file
async fn share(path: PathBuf) -> Result<()> {
    // create a blob store
    // add a file
    // create a blob ticket
    // create an endpoint and router
    // add blobs proto to the router
    // spawn the router
    // print blob ticket
    // wait until control-c
    todo!();
}

/// Client mode - receives a file
async fn receive(target: &str, ticket: &str) -> Result<()> {
    // create a blob store
    // create an endpoint
    // parse the blob ticket
    // fetch the blob
    // export the blob to the target
    todo!();
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
