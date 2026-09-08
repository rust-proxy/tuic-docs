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

[生成配置文件](tools/config-generator.md){ .md-button .md-button--primary }

使用浏览器本地运行的生成器，填写连接信息后下载配对的服务端与客户端配置。支持 TOML、JSON 和 YAML；使用前请阅读[版本限制与配置说明](tools/config-generator-reference.md)。

[程序下载](https://github.com/Itsusinn/tuic/releases) · [项目源码](https://github.com/Itsusinn/tuic) · [协议规范](https://github.com/rust-proxy/wind/blob/9025349201ba316015cea1b270e48824a04bcc21/specs/tuic.zh_CN.md)
