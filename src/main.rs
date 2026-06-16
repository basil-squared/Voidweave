use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

mod client;
mod config;
mod messages;
mod room;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = config::Config::load()?;
    let registry = Arc::new(room::RoomRegistry::new(
        config.max_rooms,
        config.max_players_per_room,
    ));

    let listener = TcpListener::bind(config.addr()).await?;
    info!("voidmat-relay listening on {}", config.addr());

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("new connection from {}", addr);
                let registry = Arc::clone(&registry);
                tokio::spawn(async move {
                    match accept_async(stream).await {
                        Ok(ws_stream) => {
                            client::handle_connection(ws_stream, registry).await;
                        }
                        Err(e) => {
                            error!("websocket handshake failed: {}", e);
                        }
                    }
                });
            }
            Err(e) => {
                error!("accept error: {}", e);
            }
        }
    }
}
