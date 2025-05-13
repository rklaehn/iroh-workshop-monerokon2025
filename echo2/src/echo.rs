use std::pin::Pin;

use anyhow::Result;
use iroh::{endpoint::Connection, protocol::ProtocolHandler};
use tracing::info;

/// The ALPN protocol identifier for the echo service
pub const ECHO_ALPN: &[u8] = b"ECHO";

/// Echo protocol handler
#[derive(Debug)]
pub struct EchoProtocol;

impl ProtocolHandler for EchoProtocol {
    fn accept(
        &self,
        conn: Connection,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'static>> {
        Box::pin(async move {
            // accept a single bidi stream
            // read the message from the stream and echo it back
            // close the connection
            todo!();
        })
    }
}
