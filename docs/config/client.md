# Client Configuration

## Top-level Options

| Key | Type | Default | Description |
| --- | ---- | ------- | ----------- |
| `log_level` | string | `"warn"` | Logging level: `trace`, `debug`, `info`, `warn`, `error`, `off` |

## `[relay]`

Options for connecting to the TUIC server.

| Key | Type | Default | Description |
| --- | ---- | ------- | ----------- |
| `server` | string | — | Server address, e.g. `"example.com:443"` |
| `uuid` | string | — | User UUID |
| `password` | string | — | User password |
| `ip` | string | — | Optional outbound IP address to bind |
| `ipstack_prefer` | string | `"v4first"` | IP stack preference: `v4first`, `v6first`, `v4only`, `v6only` |
| `certificates` | string[] | `[]` | Custom certificate paths for server verification |
| `udp_relay_mode` | string | `"native"` | UDP relay mode: `native` or `quic` |
| `congestion_control` | string | `"cubic"` | Congestion control: `cubic`, `new_reno`, `bbr`, `bbr3` |
| `alpn` | string[] | `[]` | ALPN protocols (e.g. `["h3", "h2"]`) |
| `zero_rtt_handshake` | bool | `false` | Enable 0-RTT handshake |
| `disable_sni` | bool | `false` | Disable SNI (Server Name Indication) |
| `sni` | string | — | Override SNI hostname |
| `timeout` | duration | `"8s"` | Connection timeout |
| `heartbeat` | duration | `"3s"` | Heartbeat interval |
| `disable_native_certs` | bool | `false` | Disable the native OS certificate store |
| `send_window` | integer | `16777216` | QUIC send window size (bytes) |
| `receive_window` | integer | `8388608` | QUIC receive window size (bytes) |
| `initial_mtu` | integer | `1200` | Initial MTU |
| `min_mtu` | integer | `1200` | Minimum MTU |
| `gso` | bool | `true` | Enable Generic Segmentation Offload |
| `pmtu` | bool | `true` | Enable Path MTU Discovery |
| `gc_interval` | duration | `"3s"` | Garbage collection interval |
| `gc_lifetime` | duration | `"15s"` | Garbage collection lifetime |
| `skip_cert_verify` | bool | `false` | Skip TLS certificate verification (insecure, testing only) |

## `[local]`

Options for the local SOCKS5 server.

| Key | Type | Default | Description |
| --- | ---- | ------- | ----------- |
| `server` | string | — | Local SOCKS5 listen address, e.g. `"127.0.0.1:1080"` |
| `username` | string | — | Optional SOCKS5 authentication username |
| `password` | string | — | Optional SOCKS5 authentication password |
| `dual_stack` | bool | `true` | Enable dual-stack (IPv4 and IPv6) |
| `max_packet_size` | integer | `1500` | Maximum UDP packet size |

### TCP Port Forwarding

```toml
[[local.tcp_forward]]
listen = "127.0.0.1:8080"
remote = "example.com:80"
```

### UDP Port Forwarding

```toml
[[local.udp_forward]]
listen = "127.0.0.1:5353"
remote = "8.8.8.8:53"
timeout = "60s"
```

## Full Example

```toml
log_level = "info"

[relay]
server = "example.com:443"
uuid = "f0e12827-fe60-458c-8269-a05ccb0ff8da"
password = "your_password_here"
ipstack_prefer = "v4first"
udp_relay_mode = "native"
congestion_control = "bbr"
alpn = []
zero_rtt_handshake = false
disable_sni = false
timeout = "8s"
heartbeat = "3s"
disable_native_certs = false
send_window = 16777216
receive_window = 8388608
initial_mtu = 1200
min_mtu = 1200
gso = true
pmtu = true
gc_interval = "3s"
gc_lifetime = "15s"
skip_cert_verify = false

[local]
server = "127.0.0.1:1080"
dual_stack = true
max_packet_size = 1500

# [[local.tcp_forward]]
# listen = "127.0.0.1:8080"
# remote = "example.com:80"

# [[local.udp_forward]]
# listen = "127.0.0.1:5353"
# remote = "8.8.8.8:53"
# timeout = "60s"
```
