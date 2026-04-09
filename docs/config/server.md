# Server Configuration

## Top-level Options

| Key | Type | Default | Description |
| --- | ---- | ------- | ----------- |
| `log_level` | string | `"warn"` | Logging level: `trace`, `debug`, `info`, `warn`, `error`, `off` |
| `server` | string | — | Socket address to listen on, e.g. `"[::]:443"` |
| `data_dir` | string | `""` | Working directory for relative certificate/key paths |
| `udp_relay_ipv6` | bool | `true` | Create separate UDP sockets for IPv6 UDP relay |
| `zero_rtt_handshake` | bool | `false` | Enable 0-RTT QUIC handshake (not recommended for security) |
| `dual_stack` | bool | `true` | Whether the listening socket should be dual-stack |
| `auth_timeout` | duration | `"3s"` | How long to wait for client authentication |
| `task_negotiation_timeout` | duration | `"3s"` | Maximum duration for task negotiation |
| `gc_interval` | duration | `"10s"` | Interval between UDP packet fragment garbage collection |
| `gc_lifetime` | duration | `"30s"` | How long to keep UDP packet fragments before dropping |
| `max_external_packet_size` | integer | `1500` | Maximum packet size received from outbound UDP sockets (bytes) |
| `stream_timeout` | duration | `"60s"` | How long to preserve TCP and UDP I/O tasks |

## `[users]`

Map of user UUID to password:

```toml
[users]
f0e12827-fe60-458c-8269-a05ccb0ff8da = "password"
```

## `[tls]`

| Key | Type | Default | Description |
| --- | ---- | ------- | ----------- |
| `self_sign` | bool | `false` | Use an auto-generated self-signed certificate |
| `certificate` | string | `""` | Path to certificate file |
| `private_key` | string | `""` | Path to private key file |
| `alpn` | string[] | `[]` | ALPN protocols (e.g. `["h3"]`) |
| `hostname` | string | `"localhost"` | Domain name or IP for certificate issuance / self-sign |
| `auto_ssl` | bool | `false` | Enable ACME automatic certificate provisioning |
| `acme_email` | string | `""` | Email for ACME account (defaults to `admin@<hostname>`) |

## `[restful]`

RESTful API for monitoring and management.

| Key | Type | Default | Description |
| --- | ---- | ------- | ----------- |
| `addr` | string | `"127.0.0.1:8443"` | Address to bind the API server |
| `secret` | string | — | Bearer token for API authentication |
| `maximum_clients_per_user` | integer | `0` | Max simultaneous clients per UUID (0 = unlimited) |

### Endpoints

Authenticate using `Authorization: Bearer <secret>`.

| Method | Path | Description |
| ------ | ---- | ----------- |
| `GET` | `/online` | Number of online clients |
| `GET` | `/detailed_online` | Online clients' IPs and ports |
| `POST` | `/kick` | Kick specified users |
| `GET` | `/traffic` | Current traffic stats |
| `GET` | `/reset_traffic` | Reset and return traffic stats |

## `[quic]`

| Key | Type | Default | Description |
| --- | ---- | ------- | ----------- |
| `initial_mtu` | integer | `1200` | Initial UDP payload size before MTU discovery |
| `min_mtu` | integer | `1200` | Minimum supported UDP payload size |
| `gso` | bool | `true` | Enable Generic Segmentation Offload |
| `pmtu` | bool | `true` | Enable Path MTU Discovery |
| `send_window` | integer | `16777216` | Max bytes to transmit without acknowledgment |
| `receive_window` | integer | `8388608` | Max bytes peer may transmit without acknowledgment per stream |
| `max_idle_time` | duration | `"30s"` | How long to wait before closing an idle connection |

### `[quic.congestion_control]`

| Key | Type | Default | Description |
| --- | ---- | ------- | ----------- |
| `controller` | string | `"bbr"` | Algorithm: `bbr`, `bbr3`, `cubic`, `new_reno` |
| `initial_window` | integer | `1048576` | Initial congestion window size (bytes) |

## `[[acl]]`

Access Control List rules. Evaluated in order; the first matching rule is used.

### Array-of-tables format

```toml
[[acl]]
addr = "private"
outbound = "drop"

[[acl]]
addr = "8.8.8.8"
ports = "udp/53"
outbound = "default"
hijack = "1.1.1.1"
```

| Key | Description |
| --- | ----------- |
| `addr` | Address to match: IPv4/IPv6, CIDR, domain, wildcard, `localhost`, or `private` |
| `ports` | Optional port filter, e.g. `"udp/53,tcp/80,443"` |
| `outbound` | Outbound to use: `direct`, `default`, `drop`, or a named outbound |
| `hijack` | Optional redirect target address |

### Multi-line string format

```toml
acl = '''
drop private
drop localhost
default 8.8.4.4 udp/53 1.1.1.1
'''
```

## `[outbound]`

### `[outbound.default]`

The default outbound used when no name is specified.

```toml
[outbound.default]
type = "direct"
ip_mode = "v4first"
```

### Named outbounds

```toml
[outbound.through_socks5]
type = "socks5"
addr = "127.0.0.1:1080"
username = "optional"
password = "optional"
allow_udp = false
```

#### Direct outbound options

| Key | Type | Description |
| --- | ---- | ----------- |
| `type` | string | `"direct"` |
| `ip_mode` | string | `v4first`, `v6first`, `v4only`, `v6only` |
| `bind_ipv4` | string | Local IPv4 address to bind |
| `bind_ipv6` | string | Local IPv6 address to bind |
| `bind_device` | string | Network interface to bind |

#### SOCKS5 outbound options

| Key | Type | Description |
| --- | ---- | ----------- |
| `type` | string | `"socks5"` |
| `addr` | string | SOCKS5 proxy address |
| `username` | string | Optional SOCKS5 username |
| `password` | string | Optional SOCKS5 password |
| `allow_udp` | bool | Allow UDP via SOCKS5 (default: `false`) |

## `[experimental]`

| Key | Type | Default | Description |
| --- | ---- | ------- | ----------- |
| `drop_loopback` | bool | `true` | Drop connections to loopback addresses when no ACL rule matches |
| `drop_private` | bool | `true` | Drop connections to private/LAN addresses when no ACL rule matches |

## Full Example

```toml
log_level = "info"
server = "[::]:443"
data_dir = ""
udp_relay_ipv6 = true
zero_rtt_handshake = false
dual_stack = true
auth_timeout = "3s"
task_negotiation_timeout = "3s"
gc_interval = "10s"
gc_lifetime = "30s"
max_external_packet_size = 1500
stream_timeout = "60s"

[[acl]]
addr = "private"
outbound = "drop"

[[acl]]
addr = "localhost"
outbound = "drop"

[users]
f0e12827-fe60-458c-8269-a05ccb0ff8da = "password"

[tls]
self_sign = false
certificate = "/path/to/cert.pem"
private_key = "/path/to/key.pem"
alpn = ["h3"]
hostname = "your.domain.com"
auto_ssl = false

[restful]
addr = "127.0.0.1:8443"
secret = "YOUR_SECRET_HERE"
maximum_clients_per_user = 0

[quic]
initial_mtu = 1200
min_mtu = 1200
gso = true
pmtu = true
send_window = 16777216
receive_window = 8388608
max_idle_time = "30s"

[quic.congestion_control]
controller = "bbr"
initial_window = 1048576

[outbound.default]
type = "direct"
ip_mode = "v4first"

[experimental]
drop_loopback = true
drop_private = true
```
