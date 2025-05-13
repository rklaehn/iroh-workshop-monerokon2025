use std::{env, ops::Deref, path::PathBuf, process, str::FromStr};

use anyhow::{ensure, Context, Result};
use iroh::{protocol::Router, Endpoint};
use iroh_blobs::{
    format::collection::Collection, net_protocol::Blobs, store::fs::FsStore, ticket::BlobTicket,
    util::sink,
};
use tracing::info;
use util::{crate_name, create_recv_dir, create_send_dir};

mod util;

/// Server mode - shares a file or directory
async fn share(path: PathBuf) -> Result<()> {
    // create a blob store
    // add a directory
    // create a blob ticket
    // create an endpoint and router
    // add blobs proto to the router
    // spawn the router
    // print blob ticket
    // wait until control-c

    // Note: you can use util::import
    todo!();
}

/// Client mode - receives a file
async fn receive(ticket: &str) -> Result<()> {
    // create a blob store
    // create an endpoint
    // parse the blob ticket
    // fetch the hash sequence
    // export the directory

    // Note: you can use util::export
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
        "receive" | "recv" if args.len() == 3 => {
            // Client mode - receive a file or directory
            let ticket = &args[2];
            receive(ticket).await
        }
        _ => {
            println!("Usage: {} <command> [args]", crate_name());
            println!("Commands:");
            println!("  share <dir_path>   Share a directory");
            println!("  receive <ticket>   Receive a directory");
            process::exit(1);
        }
    }
}
