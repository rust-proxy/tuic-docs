# Server Quick Start

## Installation

### Download Prebuilt Binary

Get the latest release from [GitHub Releases](https://github.com/Itsusinn/tuic/releases).

### Install via Cargo

```bash
cargo install --git https://github.com/Itsusinn/tuic.git tuic-server
```

### Docker

```bash
docker run --name tuic-server \
  --restart always \
  --network host \
  -v /PATH/TO/CONFIG_FILE:/etc/tuic/config.toml \
  -v /PATH/TO/CERTIFICATE:/PATH/TO/CERTIFICATE \
  -v /PATH/TO/PRIVATE_KEY:/PATH/TO/PRIVATE_KEY \
  -dit ghcr.io/itsusinn/tuic-server:latest
```

## Running

```bash
# Specify config file directly
tuic-server -c /path/to/config.toml

# Specify config directory (finds first config file alphabetically)
tuic-server -d /path/to/config/dir

# Generate an example configuration file
tuic-server --init
```

## Minimal Configuration

Create a `config.toml`:

```toml
log_level = "info"

server = "[::]:443"

[users]
f0e12827-fe60-458c-8269-a05ccb0ff8da = "your_password"

[tls]
self_sign = true
hostname = "your.domain.com"
```

This starts a TUIC server on port 443 using a self-signed TLS certificate.

See [Server Configuration](../config/server.md) for all options.
