---
hide:
  - toc
  - navigation
---
# 👋 Welcome to TUIC User Manual

TUIC is a proxy protocol focusing on minimizing the additional handshake latency caused by relaying as much as possible, while keeping the protocol simple and easy to implement.

## Overview

This is a fork of the [original TUIC repo](https://github.com/tuic-protocol/tuic) with significant enhancements and additional features, including a server binary ([tuic-server](https://github.com/Itsusinn/tuic/tree/dev/tuic-server)) and a client binary ([tuic-client](https://github.com/Itsusinn/tuic/tree/dev/tuic-client)).

### [✨ Features](https://github.com/Itsusinn/tuic#readme)

**Protocol:**

- ⚡ 0-RTT TCP/UDP proxying over QUIC
- 🌐 0-RTT authentication
- 🔀 Full NAT cone UDP relay (`native` and `quic` modes)
- 📡 Fully multiplexed connections
- 🚀 Connection migration support

**Server:**

- 🔒 Automatic TLS certificate provisioning via ACME (including IP certificates)
- 🔑 Self-signed certificate support with auto hot-reload
- 📋 Flexible ACL (Access Control List) with configurable outbound rules
- 🌍 SOCKS5 outbound proxy support
- 📊 RESTful API with traffic statistics
- 🛡 Built-in private/loopback address protection

**Client:**

- 🔗 Local SOCKS5 proxy inbound
- 🔄 TCP/UDP port forwarding
- 🎛 Congestion control algorithm selection (BBR3, BBR, CUBIC, NewReno)
- ⚙️ `skip_cert_verify` option for testing environments
