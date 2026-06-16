use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;
use tracing::{debug, info, warn};

use crate::messages::{ClientMessage, ServerMessage};
use crate::room::{ConnectedPlayer, RoomRegistry, Tx};

pub struct ClientHandler {
    connection_id: String,
    current_room: Option<String>,
    player_id: Option<String>,
    username: Option<String>,
    registry: Arc<RoomRegistry>,
    tx: Tx,
}

impl ClientHandler {
    pub fn new(registry: Arc<RoomRegistry>, tx: Tx) -> Self {
        Self {
            connection_id: uuid::Uuid::new_v4().to_string(),
            current_room: None,
            player_id: None,
            username: None,
            registry,
            tx,
        }
    }

    async fn handle_message(&mut self, msg: ClientMessage) -> anyhow::Result<()> {
        match msg {
            ClientMessage::CreateRoom {
                player_id,
                username,
            } => {
                if self.current_room.is_some() {
                    let _ = self
                        .tx
                        .send(ServerMessage::Error {
                            message: "already in a room".to_string(),
                        });
                    return Ok(());
                }

                let code = self.registry.generate_code();
                let player = ConnectedPlayer {
                    player_id: player_id.clone(),
                    username: username.clone(),
                    is_host: true,
                    tx: self.tx.clone(),
                };

                match self.registry.create_room(code.clone(), player) {
                    Ok(()) => {
                        self.current_room = Some(code.clone());
                        self.player_id = Some(player_id.clone());
                        self.username = Some(username.clone());
                        let _ = self.tx.send(ServerMessage::RoomCreated {
                            room_code: code.clone(),
                        });
                        info!("player {} created room {}", username, code);
                    }
                    Err(e) => {
                        let _ = self.tx.send(ServerMessage::Error {
                            message: e.to_string(),
                        });
                    }
                }
            }
            ClientMessage::JoinRoom {
                room_code,
                player_id,
                username,
            } => {
                if self.current_room.is_some() {
                    let _ = self
                        .tx
                        .send(ServerMessage::Error {
                            message: "already in a room".to_string(),
                        });
                    return Ok(());
                }

                let player = ConnectedPlayer {
                    player_id: player_id.clone(),
                    username: username.clone(),
                    is_host: false,
                    tx: self.tx.clone(),
                };

                match self.registry.join_room(&room_code, player.clone()) {
                    Ok(players) => {
                        self.current_room = Some(room_code.clone());
                        self.player_id = Some(player_id.clone());
                        self.username = Some(username.clone());

                        let _ = self.tx.send(ServerMessage::RoomJoined {
                            room_code: room_code.clone(),
                            players,
                        });

                        self.registry.broadcast_in_room(
                            &room_code,
                            &ServerMessage::PlayerJoined(crate::messages::PlayerInfo {
                                player_id: player_id.clone(),
                                username: username.clone(),
                                is_host: false,
                            }),
                            Some(&player_id),
                        );

                        info!("player {} joined room {}", username, room_code);
                    }
                    Err(e) => {
                        let _ = self.tx.send(ServerMessage::Error {
                            message: e.to_string(),
                        });
                    }
                }
            }
            ClientMessage::LeaveRoom => {
                self.handle_leave().await?;
            }
            ClientMessage::GameAction { payload } => {
                if let Some(room_code) = &self.current_room {
                    if let Some(player_id) = &self.player_id {
                        self.registry.broadcast_in_room(
                            room_code,
                            &ServerMessage::GameAction {
                                from: player_id.clone(),
                                payload,
                            },
                            Some(player_id),
                        );
                    }
                } else {
                    let _ = self.tx.send(ServerMessage::Error {
                        message: "not in a room".to_string(),
                    });
                }
            }
            ClientMessage::Ping => {
                let _ = self.tx.send(ServerMessage::Pong);
            }
        }
        Ok(())
    }

    async fn handle_leave(&mut self) -> anyhow::Result<()> {
        if let Some(room_code) = self.current_room.take() {
            let player_id = self.player_id.as_deref().unwrap_or("");
            let username = self.username.as_deref().unwrap_or("");

            if let Some((_was_host, new_host)) = self.registry.leave_room(&room_code, player_id) {
                self.registry.broadcast_in_room(
                    &room_code,
                    &ServerMessage::PlayerLeft {
                        player_id: player_id.to_string(),
                        username: username.to_string(),
                    },
                    None,
                );

                if let Some(new_host_id) = new_host {
                    self.registry.broadcast_in_room(
                        &room_code,
                        &ServerMessage::HostChanged {
                            new_host_id,
                        },
                        None,
                    );
                }

                info!("player {} left room {}", username, room_code);
            }
        }
        Ok(())
    }
}

pub async fn handle_connection<S>(
    stream: WebSocketStream<S>,
    registry: Arc<RoomRegistry>,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (mut ws_sender, mut ws_receiver) = stream.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    let mut handler = ClientHandler::new(registry, tx);

    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap_or_default();
            if ws_sender.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = ws_receiver.next().await {
        match msg {
            Message::Text(text) => {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        debug!("received: {:?}", client_msg);
                        if let Err(e) = handler.handle_message(client_msg).await {
                            warn!("error handling message: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("malformed message: {} — {}", text, e);
                        let _ = handler.tx.send(ServerMessage::Error {
                            message: format!("malformed message: {}", e),
                        });
                    }
                }
            }
            Message::Close(_) => break,
            Message::Ping(_) => {
                debug!("received ping");
            }
            _ => {}
        }
    }

    handler.handle_leave().await.ok();
    write_task.abort();
    info!("connection {} closed", handler.connection_id);
}
