# 配置生成器说明

[打开配置生成器](/tuic/config-generator/){ .md-button .md-button--primary }

生成器面向 **Itsusinn/tuic 2.0.0-dev4**，以 TUIC [`4719113`](https://github.com/Itsusinn/tuic/tree/4719113dbc0a8dd54e3fd581199ad87c82748229) 与其 Wind 子模块 [`9025349`](https://github.com/rust-proxy/wind/tree/9025349201ba316015cea1b270e48824a04bcc21) 为核对基线。它生成现代分组配置，采用 Quinn 后端。

## 使用流程

1. 选择配对生成、仅服务端或仅客户端。
2. 填写连接与认证信息。首次打开时生成随机 UUID 和密码；连接已有服务端时替换为已有凭据。
3. 配置 TLS，并按需设置本地代理、重连和端口转发。
4. 修正标记的字段，选择 TOML、JSON 或 YAML，复制或下载配置。配对模式需要分别保存服务端和客户端文件。
5. 将证书放到配置指定的部署路径，开放 UDP 端口，再使用预览下方的命令启动对应程序。

生成器是 Rust WASM + Svelte 编写的独立单页应用，需要支持 JavaScript 和 WebAssembly 的现代浏览器。所有输入与序列化都在浏览器本地进行，不写入 URL 或浏览器存储。页面使用自身的样式和主题，不加载文档站框架或分析脚本。刷新或离开页面会丢失输入。预览默认隐藏密码，**复制与下载包含明文凭据**，应妥善保存。

表单校验通过只表示字段符合生成器的约束，不代表 DNS、证书文件、防火墙或代理连接已经验证。

## 连接地址 {#addresses}

| 表单 | 输出字段 | 含义 |
| --- | --- | --- |
| 连接域名或 IP、端口 | 客户端 `server` | 从客户端可达的服务器地址 |
| 服务端监听地址 | 服务端 `server` | 服务端本机的 IP 和 UDP 端口 |
| 客户端 SNI | 客户端 `tls.sni` | 与证书相符的 DNS 域名 |

监听地址默认为 `[::]:8443`，客户端连接端口默认为 `8443`。二者独立设置，支持容器或 NAT 端口映射。监听地址只接受 IP；IPv6 地址与端口组合时使用 `[IPv6]:端口`。

连接地址可填域名、IPv4 或 IPv6。使用 IP 连接时仍需填写证书域名作为 SNI。生成器会同时写入客户端的 `ip` 字段，避免此版本对 IPv6 字面量执行 DNS 地址拼接。

## 用户认证 {#users}

服务端 `users` 是 UUID 到密码的映射；客户端使用顶层 `uuid` 与 `password`。配对模式从服务端用户列表中选择一个用户用于客户端配置。新增、删除、切换用户后，两份输出自动更新。

生成按钮使用浏览器加密随机源，生成 UUID v4 与 24 字节随机值的十六进制密码。需要 HTTPS 或 localhost。若浏览器不支持，可手动输入。生成器拒绝空密码、全零 UUID 和重复 UUID，不修改密码中的空格或特殊字符。

## TLS 与证书 {#tls}

| 方式 | 服务端输出 | 客户端行为 |
| --- | --- | --- |
| 已有受信任证书 | `tls.certificate`、`tls.private_key`、`tls.hostname` | 默认验证系统信任链与服务端身份 |
| ACME | `tls.auto_ssl`、`tls.acme_email`、`tls.hostname`、`data_dir` | 默认验证证书 |
| 自签名测试 | `tls.self_sign`、`tls.hostname` | 配对测试需明确启用 `tls.skip_cert_verify` |

证书域名留空时沿用连接域名。仅生成服务端或以 IP 连接时，应明确填写。证书与私钥仅填写部署机器上的路径，生成器不创建或上传证书。

两端均显式输出 `tls.alpn = ["h3"]`，确保协议协商一致。此基线服务端的默认 ALPN 列表为空，客户端默认回退到 `h3`，只省略配置会导致连接失败。连接已有服务端时，请确认它也配置了 `h3`。

ACME 使用公开 DNS 域名；此基线通过 HTTP-01 在 **TCP 80** 完成验证。域名应解析到该服务器，端口可达且没有冲突。`data_dir` 用于缓存，需可写并持久化。TUIC 流量仍使用配置的 UDP 端口。

切换服务端证书方式会关闭“跳过证书校验”，并从输出移除先前模式的专用字段。自签名模式不会自动削弱客户端验证；需要手动勾选才能导出配对测试配置。

## 本地 SOCKS5 {#local}

`local.server` 默认为 `127.0.0.1:1080`。开启本地认证后同时输出 `local.username` 和 `local.password`，每项最多 255 个 UTF-8 字节。该认证与 TUIC 用户认证独立。

改为非回环监听地址时，生成器会提示未设置认证的情况。实际访问范围仍需通过本机防火墙控制。

## 日志、重连与传输 {#transport}

| 选项 | 默认值 | 输出与限制 |
| --- | --- | --- |
| 日志 | `info` | 两端 `log_level` |
| 0-RTT | 关闭 | 开启后写入 `zero_rtt_handshake`；早期数据可能重放 |
| 服务端拥塞控制 | `bbr` | `backend.quinn.congestion_control.controller` |
| 按需连接 | 开启 | 客户端 `lazy`，首次请求才连接 |
| 自动重连 | 开启 | 客户端 `reconnect` |
| 初始 / 最大重连等待 | 500 / 30000 毫秒 | 导出带 `ms` 单位的持续时间 |

服务端支持 `bbr`、`bbr3`、`cubic`、`newreno`。注意序列化值是 `newreno`；此基线的 `bbr` 和 `bbr3` 分支共用同一 BBR 工厂，不能据此承诺两者有不同性能。客户端目前固定使用 BBR，不提供算法切换控件。

## 端口转发 {#forwarding}

TCP 转发输出为 `local.tcp_forward`，UDP 转发输出为 `local.udp_forward`；每项包含 `listen` 和 `remote`，UDP 额外包含 `timeout`，默认 `60s`。

监听地址必须是 IP:端口，目标可使用域名或 IP:端口。生成器拒绝完全重复的同协议监听地址，以及与 SOCKS5 相同的监听端口；通配地址、双栈和其他进程造成的端口占用还需在部署时检查。

服务端默认拦截回环和私有目标地址。生成器保留这些默认值。访问内网目标需要另行审查服务端路由和访问控制，不能只添加转发条目。

## 当前限制 {#limitations}

- 仅输出此版本的现代配置；不导入旧 `[relay]` 配置，不生成第三方客户端格式。
- 客户端 `tls.certificates`、`disable_native_certs`、`disable_sni`、上游 `proxy`、`udp_relay_mode` 和传输窗口、MTU、拥塞控制等配置未在此基线的客户端适配层传入运行时。生成器不把这些字段作为有效功能输出。
- 不提供实验性 quiche 后端、ACL/DNS 规则编辑器或证书申请服务。
- 不尝试在浏览器中连接 TUIC 服务器或验证远程文件路径。

字段与行为依据：[客户端配置](https://github.com/Itsusinn/tuic/blob/4719113dbc0a8dd54e3fd581199ad87c82748229/crates/tuic-client/src/config.rs)、[客户端适配层](https://github.com/Itsusinn/tuic/blob/4719113dbc0a8dd54e3fd581199ad87c82748229/crates/tuic-client/src/plugin.rs)、[服务端配置](https://github.com/Itsusinn/tuic/blob/4719113dbc0a8dd54e3fd581199ad87c82748229/crates/tuic-server/src/config.rs)、[Wind TLS](https://github.com/rust-proxy/wind/blob/9025349201ba316015cea1b270e48824a04bcc21/crates/wind-tuic/src/quinn/tls.rs)。
