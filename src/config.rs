use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct RelayConfig {
    pub port: Option<u16>,
    pub host: Option<String>,
    pub max_rooms: Option<usize>,
    pub max_players_per_room: Option<usize>,
    pub ping_interval_secs: Option<u64>,
    pub ping_timeout_secs: Option<u64>,
    pub max_message_size_bytes: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TomlConfig {
    pub relay: Option<RelayConfig>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub host: String,
    pub max_rooms: usize,
    pub max_players_per_room: usize,
    pub ping_interval_secs: u64,
    pub ping_timeout_secs: u64,
    pub max_message_size_bytes: usize,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let mut config = Self::defaults();

        if let Ok(content) = std::fs::read_to_string("relay.toml") {
            if let Ok(toml_config) = toml::from_str::<TomlConfig>(&content) {
                if let Some(relay) = toml_config.relay {
                    if let Some(port) = relay.port {
                        config.port = port;
                    }
                    if let Some(host) = relay.host {
                        config.host = host;
                    }
                    if let Some(max_rooms) = relay.max_rooms {
                        config.max_rooms = max_rooms;
                    }
                    if let Some(max_players) = relay.max_players_per_room {
                        config.max_players_per_room = max_players;
                    }
                    if let Some(ping_interval) = relay.ping_interval_secs {
                        config.ping_interval_secs = ping_interval;
                    }
                    if let Some(ping_timeout) = relay.ping_timeout_secs {
                        config.ping_timeout_secs = ping_timeout;
                    }
                    if let Some(max_size) = relay.max_message_size_bytes {
                        config.max_message_size_bytes = max_size;
                    }
                }
            }
        }

        if let Ok(port) = env::var("RELAY_PORT") {
            if let Ok(p) = port.parse() {
                config.port = p;
            }
        }
        if let Ok(host) = env::var("RELAY_HOST") {
            config.host = host;
        }
        if let Ok(max_rooms) = env::var("RELAY_MAX_ROOMS") {
            if let Ok(m) = max_rooms.parse() {
                config.max_rooms = m;
            }
        }
        if let Ok(max_players) = env::var("RELAY_MAX_PLAYERS") {
            if let Ok(m) = max_players.parse() {
                config.max_players_per_room = m;
            }
        }

        Ok(config)
    }

    fn defaults() -> Self {
        Self {
            port: 7777,
            host: "0.0.0.0".to_string(),
            max_rooms: 1000,
            max_players_per_room: 8,
            ping_interval_secs: 30,
            ping_timeout_secs: 10,
            max_message_size_bytes: 65536,
        }
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
