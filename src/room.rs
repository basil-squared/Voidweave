use dashmap::DashMap;
use rand::Rng;
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::messages::{PlayerInfo, ServerMessage};

pub type Tx = mpsc::UnboundedSender<ServerMessage>;

#[derive(Debug, Clone)]
pub struct ConnectedPlayer {
    pub player_id: String,
    pub username: String,
    pub is_host: bool,
    pub tx: Tx,
}

#[derive(Debug, Default)]
pub struct Room {
    pub players: Vec<ConnectedPlayer>,
}

impl Room {
    pub fn broadcast(&self, message: &ServerMessage, exclude_id: Option<&str>) {
        for player in &self.players {
            if exclude_id.map_or(true, |id| player.player_id != id) {
                let _ = player.tx.send(message.clone());
            }
        }
    }

    pub fn broadcast_all(&self, message: &ServerMessage) {
        self.broadcast(message, None);
    }

    pub fn remove_player(&mut self, player_id: &str) -> Option<ConnectedPlayer> {
        if let Some(pos) = self.players.iter().position(|p| p.player_id == player_id) {
            Some(self.players.remove(pos))
        } else {
            None
        }
    }

    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
    }

    pub fn player_infos(&self) -> Vec<PlayerInfo> {
        self.players
            .iter()
            .map(|p| PlayerInfo {
                player_id: p.player_id.clone(),
                username: p.username.clone(),
                is_host: p.is_host,
            })
            .collect()
    }

    pub fn migrate_host(&mut self) -> Option<String> {
        if let Some(next_host) = self.players.first_mut() {
            next_host.is_host = true;
            Some(next_host.player_id.clone())
        } else {
            None
        }
    }
}

pub struct RoomRegistry {
    rooms: Arc<DashMap<String, Room>>,
    max_rooms: usize,
    max_players: usize,
}

impl RoomRegistry {
    pub fn new(max_rooms: usize, max_players: usize) -> Self {
        Self {
            rooms: Arc::new(DashMap::new()),
            max_rooms,
            max_players,
        }
    }

    pub fn generate_code(&self) -> String {
        let chars = "ABCDEFGHIJKLMNPQRSTUVWXYZ23456789";
        let mut rng = rand::thread_rng();

        loop {
            let code: String = (0..6)
                .map(|_| {
                    let idx = rng.gen_range(0..chars.len());
                    chars.chars().nth(idx).unwrap()
                })
                .collect();

            if !self.rooms.contains_key(&code) {
                return code;
            }
        }
    }

    pub fn create_room(&self, code: String, host: ConnectedPlayer) -> Result<(), &'static str> {
        if self.rooms.len() >= self.max_rooms {
            return Err("server is full");
        }
        let mut room = Room::default();
        room.players.push(host);
        self.rooms.insert(code, room);
        Ok(())
    }

    pub fn join_room(
        &self,
        code: &str,
        player: ConnectedPlayer,
    ) -> Result<Vec<PlayerInfo>, &'static str> {
        let mut room = self
            .rooms
            .get_mut(code)
            .ok_or("room not found")?;
        if room.players.len() >= self.max_players {
            return Err("room is full");
        }
        if room.players.iter().any(|p| p.player_id == player.player_id) {
            return Err("already in room");
        }
        let infos = room.player_infos();
        room.players.push(player);
        Ok(infos)
    }

    pub fn leave_room(&self, code: &str, player_id: &str) -> Option<(bool, Option<String>)> {
        let mut room = self.rooms.get_mut(code)?;
        let was_host = room
            .players
            .iter()
            .find(|p| p.player_id == player_id)
            .map_or(false, |p| p.is_host);

        room.remove_player(player_id);

        if room.is_empty() {
            drop(room);
            self.rooms.remove(code);
            return Some((was_host, None));
        }

        let new_host = if was_host {
            room.migrate_host()
        } else {
            None
        };

        Some((was_host, new_host))
    }

    pub fn broadcast_in_room(
        &self,
        code: &str,
        message: &ServerMessage,
        exclude_id: Option<&str>,
    ) {
        if let Some(room) = self.rooms.get(code) {
            room.broadcast(message, exclude_id);
        }
    }

    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    pub fn get_room_players(&self, code: &str) -> Option<Vec<PlayerInfo>> {
        self.rooms.get(code).map(|room| room.player_infos())
    }
}
