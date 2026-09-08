# TUIC 中文文档与配置工具

本仓库包含两个独立的静态应用：使用 [Zensical](https://zensical.org/docs/) 构建的中文手册，以及使用 **Rust WASM + Svelte 5** 构建的配置生成器。

- 文档：[docs.ihsin.dev/tuic](https://docs.ihsin.dev/tuic/)。
- 生成器：[docs.ihsin.dev/tuic/config-generator/](https://docs.ihsin.dev/tuic/config-generator/)。它有自己的 HTML、CSS、WebAssembly 和主题，可在任意静态服务器独立运行，不依赖文档站或后端。

## 独立运行配置生成器

需要 Rust stable、`wasm32-unknown-unknown` 目标和 Node.js 22.12+（CI 使用 24）。以下命令均在本仓库根目录运行：

```sh
rustup target add wasm32-unknown-unknown
npm ci --prefix config-generator
npm run dev --prefix config-generator
```

打开 `http://127.0.0.1:8080/`。无需启动 Python 或 Zensical。支持配对生成、服务端或客户端单独生成、多用户、三种证书模式、SOCKS5 认证、重连、0-RTT 和 TCP/UDP 转发。

独立构建：

```sh
npm run build --prefix config-generator
```

`npm run build` 依次使用锁定的 wasm-pack 编译 Rust 库、运行 Svelte/TypeScript 检查，再由 Vite 打包本地 JS/CSS/WASM。首次构建需要下载匹配 Cargo 锁文件的 wasm-bindgen 工具。`npm run dev` 先编译 WASM，再启动 Vite；Svelte/CSS 支持热更新。修改 Rust 或 XML 后，在另一个终端运行 `npm run wasm --prefix config-generator` 并刷新浏览器。

可通过 `CONFIG_SCHEMA` 指定其他 XML，路径相对 `config-generator/`（或绝对路径）；默认 `schema/config.xml`。替代构建应输出到独立目录，并在完成后恢复环境及默认 WASM，详见 [DSL v4](docs/tools/config-dsl.md)。

产物在 `config-generator/dist/`。将整个目录交给静态服务器即可，默认使用相对资源路径，支持根路径或带结尾 `/` 的子路径。服务器需为 `.wasm` 返回 `application/wasm`；不要通过 `file://` 打开文件。固定前缀使用 `npm run build --prefix config-generator -- --base /your-prefix/`。

凭据使用浏览器 Crypto API 生成。所有输入、校验和序列化在本地 WASM 中完成；不加载第三方分析脚本，不保存输入、主题或凭据，不向网络提交配置。复制和下载包含明文密码，预览默认隐藏密码。

## 本地运行文档

需要 Python 3.10+ 和 [uv](https://docs.astral.sh/uv/)。

```sh
uv sync --locked
uv run --locked zensical serve
```

文档开发服务器只提供文档；配置生成器使用上面的 Vite 服务器单独运行。旧 `tools/config-generator/` 文档地址保留为新页面的链接入口。

## 组合构建与验证

```sh
# Rust 核心逻辑、DSL、配置及安全边界
cargo test --workspace --locked
cargo +nightly fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy --target wasm32-unknown-unknown --lib --locked -- -D warnings

# 已构建 WASM 后检查 Svelte/TypeScript（build 也会执行）
npm run check --prefix config-generator

# 构建两个独立静态应用并放入 site/，不执行发布
uv run --locked python scripts/build-site.py
uv run --locked python tests/config-generator/check-site.py

# /tuic/ 部署前缀预览
uv run --locked python tests/config-generator/preview-server.py
```

预览生成器：`http://127.0.0.1:8765/tuic/config-generator/`。先运行 `npm ci --prefix config-generator` 安装前端依赖；组合脚本先干净构建文档，再把独立生成器放在 `site/config-generator/`。干净构建前停止文档开发服务器，避免缓存冲突。

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

浏览器测试使用独立的 npm 清单和锁文件，测试依赖不打包进应用。启动上面的组合预览后运行（本地默认使用已安装的 Edge）：

```sh
npm ci --prefix tests/config-generator
node tests/config-generator/browser.mjs

# 测试已构建的独立产物，临时本地服务随测试结束自动关闭
uv run --locked python tests/config-generator/run-browser.py

# 组合站点构建使用部署前缀
uv run --locked python tests/config-generator/run-browser.py --directory site/config-generator --prefix /tuic/config-generator/

# 也可以使用 Playwright 自带的 Chromium，与 CI 一致
npm exec --prefix tests/config-generator -- playwright install chromium
BROWSER_CHANNEL=chromium node tests/config-generator/browser.mjs
```

`PLAYWRIGHT_MODULE_PATH` 可指定已有 Playwright 模块目录；`BROWSER_CHANNEL` 支持 `msedge`、`chrome` 和 `chromium`，CI 默认使用 `chromium`。独立 Vite 开发服务器使用 `PREVIEW_URL=http://127.0.0.1:8080/`。测试覆盖 WASM 加载、独立页面结构、配对一致性、用户删除、TLS 切换、输入校验、转发编辑、复制下载、转义、移动端、主题、无外部请求和输入不持久化，截图在 `.cache/`。复用检查使用 `schema/example.xml` 构建到 `.cache/generic-site`，运行 `uv run --locked python tests/config-generator/run-browser.py --directory .cache/generic-site --script tests/config-generator/browser-generic.mjs`，完整命令见 DSL 文档；CI 同样保留默认站点产物用于后续发布。

## DSL 与维护约定

[Config DSL v4](docs/tools/config-dsl.md) 使用独立的 `config-generator/schema/config.xml` 静态描述输入、默认值、枚举、条件、列表、映射及敏感字段，由 quick-xml + Serde 反序列化，不使用 Rust 宏或闭包编写配置描述。Rust 会话生成表单视图，Svelte 负责渲染；通用投影与脱敏在 `dsl.rs`，通用校验与联动在 `dsl/rules.rs`，基础地址检查在 `validation.rs`。TUIC 品牌、页面分区、提示、跨字段规则、随机值生成声明、导出命令也全部由 XML 提供。新增目标应用只需更换 XML；`schema/example.xml` 提供无 TUIC 字段的复用示例。

配置状态只由 Rust `Session` 修改。Svelte 提交通用字段/集合操作，读取 `Snapshot` 中的字段显示值、可见性、错误和预览；不解析 XML、不执行条件、不重复保存一份可修改的配置对象。跨 WASM 边界使用 JSON 字符串，字段显示值和稳定行标识均为字符串，避免 JavaScript 数字精度损失。`ui/types.ts` 对应 `session/view.rs` 的显示契约；修改契约时同步更新两侧并运行会话测试及两套浏览器测试。复制和下载通过独立的 `export` 操作获取原始文本，不从脱敏预览读取。

| 路径 | 内容 |
| --- | --- |
| `Cargo.toml` / `Cargo.lock` | 生成器 Rust workspace 与锁定依赖 |
| `config-generator/` | 可独立构建的 Rust WASM + Svelte 单页应用 |
| `config-generator/ui/` | 通用 Svelte 控件、页面布局、浏览器操作及显示契约 |
| `config-generator/src/session.rs` / `session/view.rs` | 可原生测试的编辑操作、表单视图和预览导出 |
| `config-generator/src/wasm.rs` | WASM 接口及浏览器 Crypto API 随机数适配 |
| `config-generator/package.json` / `vite.config.js` | 锁定的前端工具与静态资源打包 |
| `config-generator/schema/config.xml` | 唯一 TUIC 产品定义：界面、字段、规则、提示与输出 |
| `config-generator/src/dsl/xml.rs` / `dsl/wire.rs` / `dsl/parser.rs` | XML 子集检查、Serde 数据模型与语义校验 |
| `config-generator/src/dsl.rs` | 数据投影、类型检查与脱敏 |
| `config-generator/src/schema.rs` | 嵌入 XML、缓存解析结果、通用状态与独立行标识 |
| `config-generator/src/dsl/metadata.rs` / `dsl/rules.rs` | 页面元数据、随机值声明、校验和字段联动 |
| `config-generator/schema/example.xml` | 无 TUIC 字段的完整应用复用示例 |
| `config-generator/src/model.rs` | 配置生成入口及三种格式序列化 |
| `config-generator/tests/` | XML DSL 与配置回归测试 |
| `zensical.toml` / `docs/` | 中文文档、导航、字段说明和 DSL 文档 |
| `scripts/build-site.py` | 组合构建两个静态应用，不发布 |
| `tests/config-generator/` | 独立解析器、真实 TUIC、站点与浏览器检查 |
| `.github/workflows/deploy.yml` | GitHub Pages 构建与发布流程 |

只维护简体中文。更新 TUIC 后，核对生成器的版本基线及字段的实际运行行为，不把尚未接入客户端运行逻辑的配置暴露成可用功能。示例使用占位域名与运行时生成的测试凭据，不加入真实部署数据。

## 发布路径

[CI and Pages 工作流](.github/workflows/deploy.yml) 在 PR、推送 `main` 及手动触发时运行：

- `check`：nightly rustfmt、stable 原生与 WASM Clippy、Rust/XML DSL 测试，以及 TOML/JSON/YAML 独立解析往返。Python 固定为 3.13，依赖使用 `uv.lock`。
- `build`：使用 wasm-pack、Svelte 检查器和 Vite 构建独立 SPA，组合文档后检查站点链接及资源，再通过锁定版本的 Playwright/Chromium 运行 TUIC 及无 TUIC 字段的 XML 复用浏览器回归。Rust、uv 和 npm 使用依赖缓存。
- `deploy`：依赖 `check` 和 `build` 成功，仅在 `main` 的推送或手动运行时发布；Pages 写权限和 OIDC 权限仅授予此作业，PR 只验证和构建。

发布产物在临时目录组装，文档位于 `/tuic/`，独立生成器位于 `/tuic/config-generator/`，根目录保留 `CNAME`。Pages 来源应设为 GitHub Actions。真实 TUIC 解析与回环测试仍按上文在具备相邻仓库的环境中运行。

文档已从 MkDocs 迁移至 Zensical，不再使用 i18n 插件。旧 `/tuic/zh/` 路径不生成重定向，外部链接应更新到 `/tuic/` 下。站点构建和配置解析通过不代表远端 DNS、证书、防火墙或代理连接已经验证。
