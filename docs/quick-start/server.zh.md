# 服务端快速开始

## 安装

### 下载预编译二进制

从 [GitHub Releases](https://github.com/Itsusinn/tuic/releases) 下载最新版本。

### 通过 Cargo 安装

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

## 运行

```bash
# 直接指定配置文件
tuic-server -c /path/to/config.toml

# 指定配置目录（按字母顺序查找第一个配置文件）
tuic-server -d /path/to/config/dir

# 生成示例配置文件
tuic-server --init
```

## 最小配置示例

创建 `config.toml`：

```toml
log_level = "info"

server = "[::]:443"

[users]
f0e12827-fe60-458c-8269-a05ccb0ff8da = "your_password"

[tls]
self_sign = true
hostname = "your.domain.com"
```

这将在 443 端口启动 TUIC 服务端，并使用自签名 TLS 证书。

完整配置请参阅 [服务端配置](../config/server.md)。
