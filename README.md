# TUIC 中文文档

使用 [Zensical](https://zensical.org/docs/) 构建的中文用户手册，线上地址为 [docs.ihsin.dev/tuic](https://docs.ihsin.dev/tuic/)。

## 内容组织

当前仅保留[首页](docs/index.md)，包含项目介绍与源码、下载入口。

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

## 维护约定

| 路径 | 内容 |
| --- | --- |
| `zensical.toml` | 中文导航、主题与 Markdown 扩展 |
| `docs/index.md` | 首页 |
| `overrides/404.html` | 中文错误页 |
| `.github/workflows/deploy.yml` | GitHub Pages 构建与发布 |

新增页面时更新 `zensical.toml` 的导航，站内链接使用相对 Markdown 路径。只维护简体中文，不生成 `.zh.md` 或英文副本。依赖在 `pyproject.toml` 声明，由 `uv.lock` 锁定。

当前 Zensical 搜索支持中文内容检索，但结果计数和筛选等部分界面提示尚未本地化，参见[上游搜索说明](https://zensical.org/docs/setup/search/)。

## 发布路径

推送到 `main` 或手动触发工作流后，GitHub Actions 使用锁定依赖构建，将页面放到 Pages 的 `/tuic/` 路径，并保留根目录 `CNAME`。Pages 来源应设置为 GitHub Actions。

站点已由 MkDocs 迁移至 Zensical，不再使用 i18n 插件。旧 `/tuic/zh/` 路径不生成重定向，外部链接应更新为 `/tuic/` 下对应地址。
