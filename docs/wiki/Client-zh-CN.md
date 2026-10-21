# 独立客户端

> **这里只配置你的电脑，不部署服务端。** 先准备已有实例的 HTTPS 根地址，例如 `https://mirror.example.com`，不要附加 `/npm` 等路径。

`mirrorproxy` 是独立二进制，不连接或管理服务端数据库。它读取共享目录，生成并执行本机
软件源配置，记录原始内容以便精确恢复。正式版本提供 Linux x86_64/arm64、macOS
Intel/Apple Silicon 和 Windows x64 资产与 SHA-256 校验。

## 安装

Linux/macOS：

```bash
curl -fsSL https://raw.githubusercontent.com/inbjo/MirrorProxy/main/scripts/install.sh | sh
mirrorproxy --version
```

Windows PowerShell：

```powershell
irm https://raw.githubusercontent.com/inbjo/MirrorProxy/main/scripts/install.ps1 | iex
mirrorproxy --version
```

安装脚本下载 Release 资产并验证校验和；需要固定版本或目录时，设置
`MIRRORPROXY_VERSION` 或 `MIRRORPROXY_INSTALL_DIR`。也可以从 Release 手动下载对应平台资产。

## 常用命令

```bash
# 列出目标；sources 前缀也可省略，风格与 chsrc 类似
mirrorproxy list
mirrorproxy list --category lang
mirrorproxy mirrors

# 查看或配置 npm 使用你的 MirrorProxy 实例
mirrorproxy get npm --base-url https://mirror.example.com
mirrorproxy set npm --base-url https://mirror.example.com --dry-run
mirrorproxy set npm --base-url https://mirror.example.com

# 恢复 set 前保存的原始配置
mirrorproxy reset npm
```

`set` 默认只修改用户范围，且遇到非空配置时会拒绝覆盖；确认需要覆盖时传入 `--force`。
`--dry-run` 先显示将执行的改动。选择 MirrorProxy 提供方时 `--base-url` 必填，格式为
`http(s)://host[:port]`，不应带包仓库路径。

Hugging Face 仅提供使用示例：`mirrorproxy get hf --base-url https://mirror.example.com`
会显示适合当前平台的 HTTP 下载命令，不写入 shell 配置或 Token。
必须设置 `HF_HUB_DISABLE_XET=1`，支持范围见[Hugging Face 下载](Proxy-Adapters-zh-CN#hugging-face-公开-http-下载)。

## 进阶：用户与系统范围

统一的 28 类发行版迁移、版本矩阵与 Linux 验证方法见
[Linux 系统软件源](System-Sources-zh-CN)。运行 `mirrorproxy systems` 可查看发行版目录，
已有按目标操作的命令继续可用。

用户范围适用于 npm、pip、Cargo、Go、Composer 等，回滚记录保存在用户状态目录下。
系统范围仅在 Linux 上直接支持，并会修改 APT、DNF、pacman、apk、zypper、xbps、Portage
等系统配置；它需要正常的管理员权限，APT 类目标还可能需要 `--distribution`。先用
`mirrorproxy get <target>` 和 `--dry-run` 检查，再执行系统范围写入。

客户端不会替你安装 MirrorProxy 服务，也不会上传账号、令牌或原始配置。

## Debian 和 Ubuntu 系统源

APT 未指定 `--distribution` 时，从 `/etc/os-release` 自动读取 `ID` 和
`VERSION_CODENAME`。原有生成独立配置文件的模式仍支持显式传入
`--distribution debian/bookworm` 或 `ubuntu/noble`。衍生发行版必须显式指定，
不会根据父发行版猜测。

迁移现有官方源，而不是额外添加一个仓库：

```bash
sudo mirrorproxy set apt --scope system --base-url https://mirror.example.com --replace-existing --dry-run
sudo mirrorproxy set apt --scope system --base-url https://mirror.example.com --replace-existing
sudo apt-get update
apt-cache policy hello
sudo mirrorproxy reset apt --scope system --dry-run
sudo mirrorproxy reset apt --scope system
```

现有源替换必须显式启用，目前支持 Debian / Ubuntu 配合 MirrorProxy 提供方。
它仅修改 `sources.list`、`sources.list.d/*.list` 和 `*.sources` 中识别出的官方基础仓库
地址，支持 DEB822 格式及其多行 URIs。原有更新和 backports 通道、组件、架构、
`Signed-By`、注释、禁用条目和第三方仓库均保留。不改变发行版通道，不安装或升级软件，
`--replace-existing` 不能与 `--distribution` 同用。自定义镜像站、Debian 安全源、
Ubuntu 安全源和 ports、旧版本归档地址保持原样，因为默认服务端适配器未覆盖这些
独立仓库。原有签名检查和更新通道继续生效，因此不保证所有 APT 请求均经过代理。

`--dry-run` 会校验并显示实际文件改动，不写入文件或回滚记录；原有生成文件模式遇到
非空配置时，预览和写入仍需 `--force`。预览中的敏感配置行会隐藏。替换前校验所有
文件，并将多文件回滚记录保存到 `/var/lib/mirrorproxy/sources/apt.json`；Unix 上保留
文件权限。每个文件单独原子写入，整个文件集合不具备原子性；中断后运行
`reset apt --scope system` 恢复。恢复前会检查所有路径和文件，后续人工修改需要
`--force` 才能覆盖。完成恢复前请保留本版本客户端，旧客户端无法读取新增的多文件
回滚格式；现有单文件回滚记录继续兼容。符号链接文件和目录会被拒绝。

除 `apt-get update` 外，还应检查 `apt-cache policy <包名>` 和实际下载地址；保留的
安全源或第三方仓库仍可能提供优先候选版本。恢复后再次刷新 APT 元数据。
隔离测试可使用 `--config-root <目录>`，系统识别、APT 文件及回滚记录均位于该目录下。

[English](Client)
