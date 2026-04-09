# 客户端快速开始

## 安装

### 下载预编译二进制

从 [GitHub Releases](https://github.com/Itsusinn/tuic/releases) 下载最新版本。

### 通过 Cargo 安装

```bash
cargo install --git https://github.com/Itsusinn/tuic.git tuic-client
```

## 运行

```bash
tuic-client -c /path/to/config.toml
```

## 最小配置示例

创建 `config.toml`：

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

这将在 `127.0.0.1:1080` 启动本地 SOCKS5 代理，并通过您的 TUIC 服务端转发流量。

完整配置请参阅 [客户端配置](../config/client.md)。
