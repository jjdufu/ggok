# ggok

webui for grok build cli

[简体中文](README.md) · **English**

## Install

`grok` must be on `PATH`. Linux / macOS, `amd64` or `aarch64`. Binary goes to `~/.local/bin/ggok` (add that dir to `PATH`).

One-liner:

```shell
curl -fsSL https://github.com/jjdufu/ggok/releases/latest/download/install.sh | bash
```

Pin a version: `bash -s -- 0.0.0`. Writes `~/.config/ggok/config.toml` if missing.

Manual: download `ggok_<version>_<os>_<arch>.tar.gz` from [Releases](https://github.com/jjdufu/ggok/releases) (`os` is `linux` / `darwin`, `arch` is `amd64` / `aarch64`; archive is `ggok` + `config.toml`).

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

Prints the listen address and login token. Open `http://127.0.0.1:9888` and sign in with the token (listens on `0.0.0.0:9888` by default).

```shell
ggok status    # pid, address, token, leader, per-session occupancy
ggok update    # download the latest Release and replace this binary; restart web if it is running
ggok stop      # stop Web only; do not kill the grok leader
ggok stop --all  # stop the leader only if nothing is running and this ggok started it
ggok restart   # after editing config; `--all` uses the same rules as stop --all
```

`ggok update` prints `Already up to date (x.y.z).` when current. Running it from a cargo `target/` build is refused. Needs `curl` and `tar` on the machine.

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

`ggok start` flags and env vars override the file: `GGOK_BIND`, `GROK_HOME`, `GGOK_GROK_BIN`, `GGOK_PERMISSION_MODE`, `GGOK_CONFIG`. `GGOK_TOKEN` can replace the token file. `ggok update` reads `GGOK_REPO` (default `jjdufu/ggok`).

Then `ggok restart`.

## Uninstall

```shell
ggok uninstall
```

Removes ggok's binary, config, logs, and cache. Leaves `~/.grok` and workspace files.

## License

[Apache License 2.0](LICENSE)
