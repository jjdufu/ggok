# ggok

非官方的 grok build cli 网页界面。

**简体中文** · [English](README_EN.md)

## 安装

需要 `grok` 在 `PATH` 上。二进制放 `~/.local/bin/ggok`（把该目录加入 PATH）。

一键：

```shell
curl -fsSL https://github.com/jjdufu/ggok/releases/latest/download/install.sh | bash
```

指定版本：`bash -s -- 0.0.0`。没有配置时写入 `~/.config/ggok/config.toml`。

手动：从 [Releases](https://github.com/jjdufu/ggok/releases) 下载 `ggok_<版本>_<os>_<arch>.tar.gz`（包内是 `ggok` + `config.toml`）。

```shell
tar -xzf ggok_0.0.0_linux_amd64.tar.gz
install -m 755 ggok ~/.local/bin/ggok
mkdir -p ~/.config/ggok
cp -n config.toml ~/.config/ggok/config.toml   # 已有配置不要覆盖
```

`ggok start` 会自动生成 `~/.config/ggok/token`（权限 `600`）和 `~/.local/state/ggok/`（pid、日志）。

## 启动

```shell
ggok start
```

终端会打印监听地址和登录 token。浏览器打开 `http://127.0.0.1:9888`，用 token 登录。

```shell
ggok status    # pid、地址、token、leader、各会话占用
ggok update    # 下载最新 Release，替换二进制；web 在跑则重启 web
ggok stop      # 只停 Web，不杀 grok leader
ggok stop --all  # 无进行中会话且 leader 由 ggok 拉起时才停 leader
ggok restart   # 改配置后用这个；`--all` 约束同 stop --all
```

## 日志

```shell
ggok status          # 会打印日志路径
tail -f ~/.local/state/ggok/ggok.log
```

## 配置

文件：`~/.config/ggok/config.toml`

一键安装写入的内容见 [`config/config.toml`](config/config.toml)，全部键见 [`config/config.toml.full`](config/config.toml.full)。

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

改完执行 `ggok restart`。

## 卸载

```shell
ggok uninstall
```

删掉 ggok 的二进制、配置、日志、缓存。不删 `~/.grok` 和工作区文件。

## 许可证

[Apache License 2.0](LICENSE)
