# TUIC 中文文档

使用 [Zensical](https://zensical.org/docs/) 构建的中文用户手册，线上地址为 [docs.ihsin.dev/tuic](https://docs.ihsin.dev/tuic/)。

## 内容组织

包含[首页](docs/index.md)、[配置生成器](docs/tools/config-generator.md)和[生成器说明](docs/tools/config-generator-reference.md)。生成器在浏览器本地生成服务端、客户端或配对配置，支持 TOML、JSON、YAML、多用户、TLS、SOCKS5 与端口转发。

## 本地开发

需要 Python 3.10 或更高版本和 [uv](https://docs.astral.sh/uv/)。在本仓库根目录运行：

```sh
uv sync --locked
uv run --locked zensical serve
```

打开命令输出的本地地址。默认端口为 `8000`，保存文件后页面自动刷新。

## 构建与验证

```sh
uv run --locked zensical build --clean
```

构建产物在 `site/`。干净构建前先停止开发服务器，避免两个进程同时操作缓存。

修改内容后检查导航、站内链接、页内锚点和搜索结果；解析配置示例，并根据对应版本的源码核对默认值与实际行为。示例中的域名、UUID 和密码使用明确占位符，不放入真实部署信息。构建成功只证明站点可以生成，不能替代代理连接测试。

### 配置生成器测试

需要 Node.js 22 或更高版本。生成器使用浏览器原生 ES modules，没有运行时第三方依赖，也不需要额外的前端打包步骤。

```sh
# DSL、模型、校验及 TOML / JSON / YAML 独立解析器往返检查
node --test tests/config-generator/*.test.mjs

# 构建后的站内链接、锚点及生成器模板检查
uv run --locked python tests/config-generator/check-site.py

# 调用相邻 tuic 仓库的真实配置解析函数，并执行本机 SOCKS5 → TUIC → TCP 回显测试
uv run --locked python tests/config-generator/check-rust.py --offline

# 构建后，以实际发布的 /tuic/ 前缀启动本机预览
uv run --locked python tests/config-generator/preview-server.py
```

默认使用 Windows 的 `.venv/Scripts/python.exe` 执行格式检查；其他环境请将 `PYTHON` 环境变量设为安装了 PyYAML 的 Python 3.11+ 路径。Rust 检查要求相邻 `../tuic` 及其子模块可用，并需要对应的构建工具；本地未缓存依赖时可省略 `--offline`。辅助 Cargo 项目和动态生成的测试凭据只写入忽略的 `.cache/`，以 TUIC 的锁文件为起点，不修改 TUIC 清单、代码或锁文件。测试中的回环目标放行仅作用于内存中的测试配置。

预览地址为 `http://127.0.0.1:8765/tuic/tools/config-generator/`。预览运行期间，可使用已有 Playwright 安装运行浏览器测试：

```sh
node tests/config-generator/browser.mjs
```

测试默认使用本机 Edge；`BROWSER_CHANNEL` 可指定已安装的 Chrome。`PLAYWRIGHT_MODULE_PATH` 可指向现有 Playwright 模块目录，`PREVIEW_URL` 可指定预览地址。覆盖配对一致性、TLS 字段切换、复制下载、输入转义、移动端布局、主题及无分析脚本；截图保存在 `.cache/`。

生成器使用 [Config DSL v1](docs/tools/config-dsl.md) 统一描述输入类型、默认值、字段约束、界面元数据和配置输出。定义位于 `schema.mjs` 的 `INPUT` / `OUTPUT` 中，由 `dsl.mjs` 在浏览器直接解释；配置模型及密码隐藏均从输出定义派生。新增字段时更新 DSL 定义、表单布局与测试，关联规则仍在 `validation.mjs` 中维护。

升级 TUIC 后，先核对 `schema.mjs` 的版本基线、字段校验与实际运行时，再更新 DSL 定义及生成器说明。不要直接将所有可解析字段暴露为有效选项，部分客户端字段在当前实现中未接入运行时。生成器页面使用专用模板关闭分析脚本；避免在此页面引入会读取表单的第三方脚本。

## 维护约定

| 路径 | 内容 |
| --- | --- |
| `zensical.toml` | 中文导航、主题与 Markdown 扩展 |
| `docs/index.md` | 首页 |
| `docs/tools/` | 配置生成器入口及字段说明 |
| `docs/assets/config-generator/` | 表单、配置模型、校验、序列化与样式 |
| `overrides/config-generator.html` | 生成器模板、页面资源及分析脚本隔离 |
| `tests/config-generator/` | 模型、格式、真实解析器与浏览器检查 |
| `overrides/404.html` | 中文错误页 |
| `.github/workflows/deploy.yml` | GitHub Pages 构建与发布 |

新增页面时更新 `zensical.toml` 的导航，站内链接使用相对 Markdown 路径。只维护简体中文，不生成 `.zh.md` 或英文副本。依赖在 `pyproject.toml` 声明，由 `uv.lock` 锁定。

当前 Zensical 搜索支持中文内容检索，但结果计数和筛选等部分界面提示尚未本地化，参见[上游搜索说明](https://zensical.org/docs/setup/search/)。

## 发布路径

推送到 `main` 或手动触发工作流后，GitHub Actions 使用锁定依赖构建，将页面放到 Pages 的 `/tuic/` 路径，并保留根目录 `CNAME`。Pages 来源应设置为 GitHub Actions。

站点已由 MkDocs 迁移至 Zensical，不再使用 i18n 插件。旧 `/tuic/zh/` 路径不生成重定向，外部链接应更新为 `/tuic/` 下对应地址。
