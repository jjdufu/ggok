# ggok

webui for grok build cli

[简体中文](README.md) · **English**

## Install

`grok` must be on `PATH`. Binary goes to `~/.local/bin/ggok` (add that dir to `PATH`).

One-liner:

```shell
curl -fsSL https://github.com/jjdufu/ggok/releases/latest/download/install.sh | bash
```

Pin a version: `bash -s -- 0.0.0`. Writes `~/.config/ggok/config.toml` if missing.

Manual: download `ggok_<version>_<os>_<arch>.tar.gz` from [Releases](https://github.com/jjdufu/ggok/releases) (`ggok` + `config.toml`).

```shell
tar -xzf ggok_0.0.0_linux_amd64.tar.gz
install -m 755 ggok ~/.local/bin/ggok
mkdir -p ~/.config/ggok
cp -n config.toml ~/.config/ggok/config.toml   # do not overwrite an existing file
```

`ggok start` creates `~/.config/ggok/token` (mode `600`) and `~/.local/state/ggok/` (pid, logs).

## Start

```shell
ggok start
```

Prints the listen address and login token. Open `http://127.0.0.1:9888` and sign in with the token.

```shell
ggok status    # running?, address, token
ggok stop
ggok restart   # after editing config
```

## Logs

```shell
ggok status
tail -f ~/.local/state/ggok/ggok.log
```

## Config

File: `~/.config/ggok/config.toml`

Installer default: [`config/config.toml`](config/config.toml). All keys: [`config/config.toml.full`](config/config.toml.full).

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

Then `ggok restart`.

## Uninstall

```shell
ggok uninstall
```

Removes ggok's binary, config, logs, and cache. Leaves `~/.grok` and workspace files.

## License

[Apache License 2.0](LICENSE)
