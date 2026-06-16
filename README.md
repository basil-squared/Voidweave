# voidmat-relay

websocket relay server for the voidmat MTG board simulator.

## what it does
dumb message broker. knows nothing about magic rules.
manages rooms, broadcasts messages between players.
see CLAUDE.md for full architecture documentation.

## quick start
```bash
cargo run
```

## configuration
copy relay.toml and edit as needed.
environment variables override relay.toml values.

```bash
RELAY_PORT=8888 cargo run
```

## deployment on proxmox LXC

### 1. create LXC container
- debian 12 template
- 256mb RAM, 1 core, 4gb disk
- static IP on your network

### 2. install
```bash
apt update && apt install -y curl build-essential
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
git clone https://github.com/yourusername/voidmat-relay
cd voidmat-relay
cargo build --release
cp target/release/voidmat-relay /usr/local/bin/
mkdir -p /etc/voidmat
cp relay.toml /etc/voidmat/
```

### 3. systemd service
```bash
cp voidmat-relay.service /etc/systemd/system/
useradd -r -s /bin/false voidmat
systemctl daemon-reload
systemctl enable voidmat-relay
systemctl start voidmat-relay
```

### 4. cloudflare tunnel (recommended)
install cloudflared:
```bash
curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 -o /usr/local/bin/cloudflared
chmod +x /usr/local/bin/cloudflared
```

authenticate and create tunnel:
```bash
cloudflared tunnel login
cloudflared tunnel create voidmat-relay
cloudflared tunnel route dns voidmat-relay relay.voidmat.gg
```

create /etc/cloudflared/config.yml:
```yaml
tunnel: <tunnel-id>
credentials-file: /root/.cloudflared/<tunnel-id>.json
ingress:
  - hostname: relay.voidmat.gg
    service: ws://localhost:7777
  - service: http_status:404
```

run as service:
```bash
cloudflared service install
systemctl start cloudflared
```

### 5. verify
```bash
systemctl status voidmat-relay
journalctl -u voidmat-relay -f
```

## message protocol
see CLAUDE.md for full protocol documentation.

## license
MPL-2.0
