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
            // same as echo2
            todo!();
        })
    }
}
