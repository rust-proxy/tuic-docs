# Client Quick Start

## Installation

### Download Prebuilt Binary

Get the latest release from [GitHub Releases](https://github.com/Itsusinn/tuic/releases).

### Install via Cargo

```bash
cargo install --git https://github.com/Itsusinn/tuic.git tuic-client
```

## Running

```bash
tuic-client -c /path/to/config.toml
```

## Minimal Configuration

Create a `config.toml`:

```toml
log_level = "info"

[relay]
server = "your.server.com:443"
uuid = "f0e12827-fe60-458c-8269-a05ccb0ff8da"
password = "your_password"
udp_relay_mode = "native"
congestion_control = "bbr"

[local]
server = "127.0.0.1:1080"
```

This starts a local SOCKS5 proxy on `127.0.0.1:1080` that tunnels traffic through your TUIC server.

See [Client Configuration](../config/client.md) for all options.
