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

            Ok(())
        })
    }
}
