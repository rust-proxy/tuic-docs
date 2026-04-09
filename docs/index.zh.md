---
hide:
  - toc
  - navigation
---

# 👋 欢迎来到 TUIC 用户文档

TUIC 是一个代理协议，专注于在中继时尽可能减少额外的握手延迟，同时保持协议本身简洁易于实现。

## 概述

本项目是 [原版 TUIC 仓库](https://github.com/tuic-protocol/tuic) 的一个 Fork，包含大量增强功能，提供服务端二进制 ([tuic-server](https://github.com/Itsusinn/tuic/tree/dev/tuic-server)) 和客户端二进制 ([tuic-client](https://github.com/Itsusinn/tuic/tree/dev/tuic-client))。

### [✨ 功能特性](https://github.com/Itsusinn/tuic#readme)

**协议：**

- ⚡ 基于 QUIC 的 0-RTT TCP/UDP 代理
- 🌐 0-RTT 身份验证
- 🔀 全锥形 NAT UDP 中继（`native` 和 `quic` 两种模式）
- 📡 完全多路复用连接
- 🚀 支持连接迁移

**服务端：**

- 🔒 通过 ACME 自动申请 TLS 证书（支持 IP 证书）
- 🔑 支持自签名证书及证书热重载
- 📋 灵活的 ACL（访问控制列表）及可配置的出站规则
- 🌍 支持 SOCKS5 出站代理
- 📊 提供 RESTful API 及流量统计
- 🛡 内置私有/回环地址访问保护

**客户端：**

- 🔗 本地 SOCKS5 代理入站
- 🔄 支持 TCP/UDP 端口转发
- 🎛 可选拥塞控制算法（BBR3、BBR、CUBIC、NewReno）
- ⚙️ 提供 `skip_cert_verify` 选项用于测试环境
