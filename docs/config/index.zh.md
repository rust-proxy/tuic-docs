# 配置

TUIC 支持 **TOML**、**JSON5** 和 **YAML** 三种配置格式，根据文件扩展名自动识别。

| 扩展名 | 格式 |
| ------ | ---- |
| `.toml` | TOML（推荐） |
| `.json`, `.json5` | JSON5（旧格式，仍支持） |
| `.yaml`, `.yml` | YAML |

也可通过环境变量强制指定格式：

- `TUIC_CONFIG_FORMAT=toml` 或 `TUIC_CONFIG_FORMAT=json5`
- `TUIC_FORCE_TOML=1` 强制使用 TOML 解析

## 配置页面

- [服务端配置](server.md)
- [客户端配置](client.md)
