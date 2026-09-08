---
hide:
  - toc
  - navigation
---

# TUIC

通过 QUIC 连接应用与远端网络。

TUIC 为 TCP 和 UDP 流量提供代理通道。本手册介绍 **Itsusinn/tuic** 的独立服务端与客户端：在服务器上运行 `tuic-server`，在本机运行 `tuic-client`，再让应用使用本地 SOCKS5 代理。

## TUIC 如何工作

```text
应用 → 本地 SOCKS5 / 转发端口 → tuic-client
                                      │
                                  QUIC / UDP
                                      │
                                 tuic-server → 目标服务
```

应用到客户端是本地连接，客户端到服务端使用 QUIC。即使代理的是 TCP 请求，服务器也需要开放 **UDP** 监听端口。

[程序下载](https://github.com/Itsusinn/tuic/releases) · [项目源码](https://github.com/Itsusinn/tuic) · [协议规范](https://github.com/rust-proxy/wind/blob/9025349201ba316015cea1b270e48824a04bcc21/specs/tuic.zh_CN.md)
