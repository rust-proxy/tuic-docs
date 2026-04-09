# Configuration

TUIC supports **TOML**, **JSON5**, and **YAML** configuration formats. The format is automatically detected based on the file extension.

| Extension | Format |
| --------- | ------ |
| `.toml` | TOML (recommended) |
| `.json`, `.json5` | JSON5 (legacy) |
| `.yaml`, `.yml` | YAML |

You can also force a specific format with environment variables:

- `TUIC_CONFIG_FORMAT=toml` or `TUIC_CONFIG_FORMAT=json5`
- `TUIC_FORCE_TOML=1` to always parse as TOML

## Pages

- [Server Configuration](server.md)
- [Client Configuration](client.md)
