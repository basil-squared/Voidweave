use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    CreateRoom {
        #[serde(rename = "playerId")]
        player_id: String,
        username: String,
    },
    JoinRoom {
        #[serde(rename = "roomCode")]
        room_code: String,
        #[serde(rename = "playerId")]
        player_id: String,
        username: String,
    },
    LeaveRoom,
    GameAction {
        payload: serde_json::Value,
    },
    Ping,
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum ServerMessage {
    RoomCreated {
        #[serde(rename = "roomCode")]
        room_code: String,
    },
    RoomJoined {
        #[serde(rename = "roomCode")]
        room_code: String,
        players: Vec<PlayerInfo>,
    },
    PlayerJoined(PlayerInfo),
    PlayerLeft {
        #[serde(rename = "playerId")]
        player_id: String,
        username: String,
    },
    GameAction {
        from: String,
        payload: serde_json::Value,
    },
    HostChanged {
        #[serde(rename = "newHostId")]
        new_host_id: String,
    },
    Error {
        message: String,
    },
    Pong,
}

#[derive(Debug, Serialize, Clone)]
pub struct PlayerInfo {
    #[serde(rename = "playerId")]
    pub player_id: String,
    pub username: String,
    #[serde(rename = "isHost")]
    pub is_host: bool,
}
