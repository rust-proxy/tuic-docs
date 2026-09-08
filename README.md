# TUIC 中文文档与配置工具

本仓库包含两个独立的静态应用：使用 [Zensical](https://zensical.org/docs/) 构建的中文手册，以及使用 **Rust + Leptos CSR** 构建的配置生成器。

- 文档：[docs.ihsin.dev/tuic](https://docs.ihsin.dev/tuic/)。
- 生成器：[docs.ihsin.dev/tuic/config-generator/](https://docs.ihsin.dev/tuic/config-generator/)。它有自己的 HTML、CSS、WebAssembly 和主题，可在任意静态服务器独立运行，不依赖文档站或后端。

## 独立运行配置生成器

需要 Rust stable、`wasm32-unknown-unknown` 目标和 Trunk 0.21.14。以下命令均在本仓库根目录运行：

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.21.14 --locked
trunk --config config-generator/Trunk.toml serve
```

打开 `http://127.0.0.1:8080/`。无需启动 Python 或 Zensical。支持配对生成、服务端或客户端单独生成、多用户、三种证书模式、SOCKS5 认证、重连、0-RTT 和 TCP/UDP 转发。

独立构建：

```sh
trunk --config config-generator/Trunk.toml build --release
```

产物在 `config-generator/dist/`。将整个目录交给静态服务器即可，默认使用相对资源路径，支持根路径或带结尾 `/` 的子路径。服务器需为 `.wasm` 返回 `application/wasm`；不要通过 `file://` 打开文件。若明确部署到固定前缀，可传入 `--public-url /your-prefix/`。

凭据使用浏览器 Crypto API 生成。所有输入、校验和序列化在本地 WASM 中完成；不加载第三方分析脚本，不保存输入、主题或凭据，不向网络提交配置。复制和下载包含明文密码，预览默认隐藏密码。

## 本地运行文档

需要 Python 3.10+ 和 [uv](https://docs.astral.sh/uv/)。

```sh
uv sync --locked
uv run --locked zensical serve
```

文档开发服务器只提供文档；配置生成器使用上面的 Trunk 服务器单独运行。旧 `tools/config-generator/` 文档地址保留为新页面的链接入口。

## 组合构建与验证

```sh
# Rust 核心逻辑、DSL、配置及安全边界
cargo test --workspace --locked
cargo +nightly fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy --target wasm32-unknown-unknown --bin tuic-config-generator --locked -- -D warnings

# 构建两个独立静态应用并放入 site/，不执行发布
uv run --locked python scripts/build-site.py
uv run --locked python tests/config-generator/check-site.py

# /tuic/ 部署前缀预览
uv run --locked python tests/config-generator/preview-server.py
```

预览生成器：`http://127.0.0.1:8765/tuic/config-generator/`。`scripts/build-site.py --trunk /path/to/trunk` 可指定 Trunk 可执行文件；脚本先干净构建文档，再把独立生成器放在 `site/config-generator/`。干净构建前停止文档开发服务器，避免缓存冲突。

### 独立格式解析与真实 TUIC 检查

```sh
# 动态生成测试凭据和配置，只写入忽略的 .cache/
cargo run --locked --example fixtures -- .cache/config-generator-fixtures

# Python 3.11+ 的 tomllib、json 和 PyYAML 独立解析器
uv run --locked --with 'PyYAML>=6,<7' python tests/config-generator/roundtrip.py .cache/config-generator-fixtures/roundtrip.json

# 调用相邻 TUIC 的真实解析函数，并执行本机 SOCKS5 → TUIC → TCP 回显
uv run --locked python tests/config-generator/check-rust.py --offline
```

真实解析检查需要相邻 `../tuic`、子模块、缓存的依赖和对应编译工具；缺少依赖缓存时可省略 `--offline`。辅助 Cargo 项目只写入 `.cache/`，以 TUIC 的锁文件为起点，不修改 TUIC 清单、源码、锁文件或子模块。回环放行只作用于内存中的测试配置，错误诊断不会打印配置内容。

### 浏览器检查

需要 Node.js 和现有 Playwright 安装，默认使用 Edge。启动上面的组合预览后运行：

```sh
node tests/config-generator/browser.mjs
```

`PLAYWRIGHT_MODULE_PATH` 可指定已有 Playwright 模块目录，`BROWSER_CHANNEL` 可改为 Chrome。独立 Trunk 开发服务器使用 `PREVIEW_URL=http://127.0.0.1:8080/`。测试覆盖 WASM 加载、独立页面结构、配对一致性、用户删除、TLS 切换、输入校验、转发编辑、复制下载、转义、移动端、主题、无外部请求和输入不持久化，截图在 `.cache/`。

## DSL 与维护约定

[Config DSL v3](docs/tools/config-dsl.md) 使用独立的 `config-generator/schema/tuic.xml` 静态描述输入、默认值、枚举、条件、列表、映射及敏感字段，由 pest 解析，不使用 Rust 宏或闭包编写配置描述。Leptos 读取同一份元数据生成表单；通用投影与脱敏在 `dsl.rs`，跨字段 TUIC 校验在 `validation.rs`。新增普通字段时编辑 XML、说明及相关测试。

| 路径 | 内容 |
| --- | --- |
| `Cargo.toml` / `Cargo.lock` | 生成器 Rust workspace 与锁定依赖 |
| `config-generator/` | 可独立构建的 Rust + Leptos 单页应用 |
| `config-generator/schema/tuic.xml` | 静态 XML 配置描述，字段、初值、条件与输出的来源 |
| `config-generator/src/dsl/xml.pest` / `dsl/parser.rs` | pest 语法与描述检查 |
| `config-generator/src/dsl.rs` | 数据投影、类型检查与脱敏 |
| `config-generator/src/schema.rs` | 嵌入 XML、缓存解析结果与表单状态绑定 |
| `config-generator/src/model.rs` | 配置生成入口及三种格式序列化 |
| `config-generator/tests/` | XML DSL 与配置回归测试 |
| `zensical.toml` / `docs/` | 中文文档、导航、字段说明和 DSL 文档 |
| `scripts/build-site.py` | 组合构建两个静态应用，不发布 |
| `tests/config-generator/` | 独立解析器、真实 TUIC、站点与浏览器检查 |
| `.github/workflows/deploy.yml` | GitHub Pages 构建与发布流程 |

只维护简体中文。更新 TUIC 后，核对生成器的版本基线及字段的实际运行行为，不把尚未接入客户端运行逻辑的配置暴露成可用功能。示例使用占位域名与运行时生成的测试凭据，不加入真实部署数据。

## 发布路径

推送 `main` 或手动触发工作流后，GitHub Actions 构建两个应用，将文档放到 `/tuic/`，独立生成器放到 `/tuic/config-generator/`，并保留根目录 `CNAME`。Pages 来源应设为 GitHub Actions。

文档已从 MkDocs 迁移至 Zensical，不再使用 i18n 插件。旧 `/tuic/zh/` 路径不生成重定向，外部链接应更新到 `/tuic/` 下。站点构建和配置解析通过不代表远端 DNS、证书、防火墙或代理连接已经验证。
