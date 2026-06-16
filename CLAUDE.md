# voidmat-relay

standalone websocket relay server for voidmat.
dumb message broker — knows nothing about magic rules or game state.
written in rust using tokio and tungstenite.

## what this server does
- manages game rooms identified by 6-character room codes
- accepts websocket connections from voidmat clients
- receives messages from any connected client
- broadcasts messages to all other clients in the same room
- handles player join/leave/disconnect gracefully
- nothing else. no game logic. no card data. no rules enforcement.

## what this server does NOT do
- validate game actions (clients trust each other, honor system)
- store any game state (ephemeral, lives in client memory only)
- authenticate users (player_id and username are client-provided)
- know anything about magic: the gathering
- persist anything to disk or database

## architecture

### components
- main.rs — entry point, starts TCP listener, spawns connection tasks
- room.rs — RoomRegistry: thread-safe map of room codes to rooms
- client.rs — per-connection async task, reads/writes websocket frames
- messages.rs — ClientMessage and ServerMessage enums, serde serialization
- config.rs — Config struct, loaded from relay.toml or environment variables

### connection lifecycle
1. client connects via websocket
2. server assigns a temporary connection_id (uuid)
3. client sends CreateRoom or JoinRoom message
4. server adds client to room, broadcasts PlayerJoined to others
5. client sends GameAction messages during play
6. server broadcasts each GameAction to all others in room
7. client disconnects (or sends LeaveRoom)
8. server removes client from room, broadcasts PlayerLeft to others
9. if room is now empty, server removes room from registry

### concurrency model
- one tokio task per connected client
- RoomRegistry wrapped in Arc<DashMap<String, Room>>
- Room contains Arc<Mutex<Vec<ConnectedPlayer>>>
- each ConnectedPlayer has an mpsc Sender for outbound messages
- incoming messages: client task reads from websocket, processes, 
  broadcasts by sending to each other player's Sender
- outgoing messages: separate write task per client reads from 
  mpsc Receiver and writes to websocket

### room codes
- 6 characters, uppercase alphanumeric (A-Z, 0-9 excluding O and 0 
  to avoid confusion — actually just use A-Z and 2-9)
- generated server-side on CreateRoom
- guaranteed unique within active rooms
- reused after room is destroyed

## message protocol
all messages are JSON over websocket text frames.

### client → server messages
```json
// create a new room
{ "type": "CreateRoom", "playerId": "uuid", "username": "juniper" }

// join an existing room
{ "type": "JoinRoom", "roomCode": "VD7X2K", "playerId": "uuid", "username": "river" }

// leave current room (also sent automatically on disconnect)
{ "type": "LeaveRoom" }

// broadcast a game action to all other players in room
{ "type": "GameAction", "payload": { ...arbitrary game state... } }

// keepalive
{ "type": "Ping" }
```

### server → client messages
```json
// room successfully created, you are the host
{ "type": "RoomCreated", "roomCode": "VD7X2K" }

// you successfully joined a room
{ "type": "RoomJoined", "roomCode": "VD7X2K", "players": [
  { "playerId": "uuid", "username": "river", "isHost": true }
]}

// another player joined your room
{ "type": "PlayerJoined", "playerId": "uuid", "username": "alex" }

// a player left or disconnected
{ "type": "PlayerLeft", "playerId": "uuid", "username": "alex" }

// a game action from another player (relay passes through unchanged)
{ "type": "GameAction", "from": "uuid", "payload": { ...unchanged... } }

// something went wrong
{ "type": "Error", "message": "room not found" }

// keepalive response
{ "type": "Pong" }

// host has changed (host disconnected, next player becomes host)
{ "type": "HostChanged", "newHostId": "uuid" }
```

### game action payload
the relay does not inspect or validate payload contents.
the voidmat client is responsible for all game action structure.
the relay simply attaches the sender's playerId as "from" and 
broadcasts to all other room members.

example game actions the client might send (relay ignores these):
- zone moves (card from hand to battlefield)
- life total changes
- stack actions (cast spell, respond, resolve)
- phase changes
- draw card
- tap/untap permanent
- counter changes

## config
loaded from relay.toml in the working directory,
with environment variable overrides:

```toml
[relay]
port = 7777
host = "0.0.0.0"
max_rooms = 1000
max_players_per_room = 8
ping_interval_secs = 30
ping_timeout_secs = 10
max_message_size_bytes = 65536
```

environment variable overrides:
RELAY_PORT=7777
RELAY_HOST=0.0.0.0
RELAY_MAX_ROOMS=1000
RELAY_MAX_PLAYERS=8

## error handling
- malformed JSON: send Error message, keep connection open
- unknown message type: send Error message, keep connection open
- room not found on JoinRoom: send Error { "room not found" }
- room full: send Error { "room is full" }
- player already in room: send Error { "already in a room" }
- player sends GameAction without being in a room: 
  send Error { "not in a room" }
- connection drops unexpectedly: treat as LeaveRoom

## host migration
the first player to create/join a room is the host.
if the host disconnects:
- next player in join order becomes host
- server sends HostChanged to all remaining players
the client uses host status to determine who can:
  - change game settings
  - kick players
  - start the game

## ping/pong keepalive
server sends Ping to each client every ping_interval_secs.
if client does not respond with Pong within ping_timeout_secs,
server closes the connection and treats as disconnect.
clients should also send Ping periodically as a keepalive.

## deployment
runs as a single binary. stateless. no database.
designed for deployment in a proxmox LXC container.
includes systemd service file and Dockerfile.

## logging
use the `tracing` crate.
log levels:
- INFO: room created, player joined, player left, room destroyed
- DEBUG: individual messages (verbose, disabled in production)
- WARN: malformed messages, unexpected disconnects
- ERROR: server errors, bind failures

log format: timestamp level target message
example: 2026-06-15T14:32:11Z INFO voidmat_relay room VD7X2K created by juniper

## running locally
cargo run
# or with custom port
RELAY_PORT=8888 cargo run

## building for production
cargo build --release
# binary at target/release/voidmat-relay
