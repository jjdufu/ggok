# ggok

非官方的 grok build cli 网页界面。

**简体中文** · [English](README_EN.md)

## 安装

需要 `grok` 在 `PATH` 上。Linux / macOS，`amd64` 或 `aarch64`。二进制装到 `~/.local/bin/ggok`。

```shell
curl -fsSL https://github.com/jjdufu/ggok/releases/latest/download/install.sh | bash
```

指定版本：`bash -s -- 0.0.0`。没有配置时写入 `~/.config/ggok/config.toml`。

手动安装：从 [Releases](https://github.com/jjdufu/ggok/releases) 下载 `ggok_<版本>_<os>_<arch>.tar.gz`（`linux` / `darwin`，`amd64` / `aarch64`）。

```shell
tar -xzf ggok_0.0.0_linux_amd64.tar.gz
install -m 755 ggok ~/.local/bin/ggok
mkdir -p ~/.config/ggok
cp -n config.toml ~/.config/ggok/config.toml
```

## 启动

```shell
ggok start
```

打开终端打印的地址（默认 `http://127.0.0.1:9888`），用打印的 token 登录。

```shell
ggok status
ggok update
ggok stop
ggok stop --all
ggok restart
```

`stop` / `restart` 只停 Web；加 `--all` 时，无进行中会话且 leader 由 ggok 拉起才会停 leader。

## 日志

```shell
ggok status
tail -f ~/.local/state/ggok/ggok.log
```

## 配置

`~/.config/ggok/config.toml`。默认见 [`config/config.toml`](config/config.toml)，全部键见 [`config/config.toml.full`](config/config.toml.full)。

| 键 | 默认 | 说明 |
| --- | --- | --- |
| `bind` | `0.0.0.0:9888` | 监听地址 |
| `workspace_roots` | `["~/workspace"]` | 工作区根，支持 `~/...`，不存在的跳过 |
| `permission_mode` | `always-approve` | `ask` / `auto` / `always-approve` |
| `token_file` | `~/.config/ggok/token` | 须绝对路径，权限 `600` |
| `grok_home` | `~/.grok` | 须绝对路径 |
| `grok_bin` | `grok` | PATH 名或绝对路径 |
| `poll_secs` | `5` | 会话轮询秒数，`0` 当作 `5` |
| `upload_max_bytes` | `20971520` | 上传上限（字节） |

`ggok start` 可用同名 flag 或环境变量覆盖：`GGOK_BIND`、`GROK_HOME`、`GGOK_GROK_BIN`、`GGOK_PERMISSION_MODE`、`GGOK_CONFIG`。也可用 `GGOK_TOKEN` 代替 token 文件。

改完执行 `ggok restart`。

## 卸载

```shell
ggok uninstall
```

不删 `~/.grok` 和工作区文件。

## 许可证

[Apache License 2.0](LICENSE)
