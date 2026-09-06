# ggok

webui for grok build cli

[简体中文](README.md) · **English**

## Install

`grok` must be on `PATH`. Linux / macOS, `amd64` or `aarch64`. Binary goes to `~/.local/bin/ggok`.

```shell
curl -fsSL https://github.com/jjdufu/ggok/releases/latest/download/install.sh | bash
```

Pin a version: `bash -s -- 0.0.0`. Writes `~/.config/ggok/config.toml` if missing.

Manual: download `ggok_<version>_<os>_<arch>.tar.gz` from [Releases](https://github.com/jjdufu/ggok/releases) (`linux` / `darwin`, `amd64` / `aarch64`).

```shell
tar -xzf ggok_0.0.0_linux_amd64.tar.gz
install -m 755 ggok ~/.local/bin/ggok
mkdir -p ~/.config/ggok
cp -n config.toml ~/.config/ggok/config.toml
```

## Start

```shell
ggok start
```

Open the printed address (default `http://127.0.0.1:9888`) and sign in with the printed token.

```shell
ggok status
ggok update
ggok stop
ggok stop --all
ggok restart
```

`stop` / `restart` stop Web only. `--all` also stops the leader if nothing is running and this ggok started it.

## Logs

```shell
ggok status
tail -f ~/.local/state/ggok/ggok.log
```

## Config

`~/.config/ggok/config.toml`. Defaults: [`config/config.toml`](config/config.toml). All keys: [`config/config.toml.full`](config/config.toml.full).

| Key | Default | Meaning |
| --- | --- | --- |
| `bind` | `0.0.0.0:9888` | Listen address |
| `workspace_roots` | `["~/workspace"]` | Workspace roots; `~/...` ok; missing dirs skipped |
| `permission_mode` | `always-approve` | `ask` / `auto` / `always-approve` |
| `token_file` | `~/.config/ggok/token` | Absolute path, mode `600` |
| `grok_home` | `~/.grok` | Absolute path |
| `grok_bin` | `grok` | PATH name or absolute path |
| `poll_secs` | `5` | Session poll interval; `0` means `5` |
| `upload_max_bytes` | `20971520` | Upload size cap (bytes) |

`ggok start` flags and env vars override the file: `GGOK_BIND`, `GROK_HOME`, `GGOK_GROK_BIN`, `GGOK_PERMISSION_MODE`, `GGOK_CONFIG`. `GGOK_TOKEN` can replace the token file.

Then `ggok restart`.

## Uninstall

```shell
ggok uninstall
```

Leaves `~/.grok` and workspace files.

## License

[Apache License 2.0](LICENSE)
