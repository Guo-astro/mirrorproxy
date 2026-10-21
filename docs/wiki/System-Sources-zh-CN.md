# Linux 系统软件源

独立客户端提供下列 28 类发行版的原生配置适配，参考了
[LinuxMirrors](https://github.com/SuperManito/LinuxMirrors) 的系统覆盖范围。
实现采用 Rust，不下载或执行对方脚本。`mirrorproxy systems` 展示客户端与服务端共享的目录。

版本范围表示**配置适配范围**，包括归档版本，不代表延长发行版安全支持，也不保证每个
版本和架构的上游元数据仍然可用。测试夹具覆盖全部发行版，实际下载仍需在 Linux 验证。

## 发行版与版本矩阵

| 客户端标识 | 发行版 | 配置版本范围 | 适配器 |
| --- | --- | --- | --- |
| `debian` | Debian | 8–14、testing/sid | APT |
| `ubuntu` | Ubuntu | 14–26 | APT |
| `kali` | Kali Linux | 滚动版本 | APT |
| `linuxmint` | Linux Mint / LMDE | Mint 17–22；LMDE 2–7 | APT |
| `deepin` | Deepin | 现有版本布局，含 25/beige | APT |
| `zorin` | Zorin OS | 现有 Ubuntu 基础仓库布局 | APT |
| `armbian` | Armbian | 现有 Debian/Ubuntu 布局 | APT |
| `proxmox` | Proxmox VE | 现有 Debian/PVE/Ceph 布局 | APT |
| `raspios` | Raspberry Pi OS | 现有 Debian/Raspbian 布局 | APT |
| `rhel` | Red Hat Enterprise Linux | 7–10；须配置兼容的客户镜像 | RPM |
| `fedora` | Fedora | 30–44 | RPM |
| `centos` | CentOS Linux | 7–8 | RPM |
| `centos-stream` | CentOS Stream | 8–10 | RPM |
| `rocky` | Rocky Linux | 8–10 | RPM |
| `alma` | AlmaLinux | 8–10 | RPM |
| `oraclelinux` | Oracle Linux | 8–10 | RPM |
| `openeuler` | openEuler | 20–26 | RPM |
| `opencloudos` | OpenCloudOS | 8.6–9；Stream 23 | RPM |
| `openkylin` | openKylin | 现有版本布局 | APT |
| `anolis` | Anolis OS | 8 / 23 | RPM |
| `opensuse` | openSUSE | Leap 15–16；Tumbleweed | RPM/Zypper |
| `archlinux` | Arch Linux / Arch Linux ARM | 滚动版本 | pacman |
| `manjaro` | Manjaro | stable/testing/unstable | pacman |
| `endeavouros` | EndeavourOS | Arch 与 EndeavourOS 镜像列表 | pacman |
| `alpine` | Alpine Linux | v3 / edge | apk |
| `gentoo` | Gentoo | 现有 Portage 配置 | distfile 镜像 |
| `nixos` | NixOS | 19–26 | Nix 模块及已有 root channel |
| `void` | Void Linux | 滚动版本，保留已有 musl 路径 | XBPS |

## 预览、应用与恢复

需要已有 MirrorProxy 实例，启用 `os` 适配器，并使用包含新目录中仓库根地址的服务端版本。
统一迁移当前使用 `mirrorproxy` 提供方。

```bash
mirrorproxy systems
sudo mirrorproxy set system --scope system --replace-existing --base-url https://mirror.example.com --dry-run
sudo mirrorproxy set system --scope system --replace-existing --base-url https://mirror.example.com

# 指定发行版时，客户端还会检查它是否匹配当前系统。
sudo mirrorproxy set rocky --scope system --replace-existing --base-url https://mirror.example.com --dry-run

sudo mirrorproxy reset system --scope system --dry-run
sudo mirrorproxy reset system --scope system
```

一次只执行一个 `set`，现有系统回滚记录会阻止再次迁移。
`reset <发行版标识> --scope system` 也能恢复匹配发行版的统一迁移。
已有按目标操作的命令及其回滚记录继续保留原有行为。
`set apt --replace-existing` 仍是[客户端页面](Client-zh-CN)介绍的较窄的 Debian/Ubuntu
基础仓库替换方式。

系统识别读取 `os-release`，不执行 shell 表达式。Armbian、树莓派与 Proxmox 的标记文件
用于识别仍报告父发行版 ID 的系统。系统识别文件的符号链接只能指向所选根目录内。
客户端保留已有版本代号、分支、`$releasever`、`$basearch`、`$repo`、`$arch`、组件、
密钥环和签名设置，不把 ARM 仓库猜成 x86 仓库，也不切换发行版版本。

APT 兼容 `.list` 和 DEB822 `.sources`，包括多行 URIs、更新源、安全源、ports 及多个
基础和厂商仓库。RPM 仅在可以替换的节中修改已识别的 baseurl 并禁用镜像发现。
Fedora 已知的基础、更新、modular metalink 会映射到等价目录，未知第三方节保持原样。
pacman 保留 Manjaro 分支并处理 EndeavourOS 独立列表；Void 从 `/usr/share` 厂商配置
生成 `/etc` 覆盖文件，不改动厂商原文件。

只有目录声明的官方地址和已知镜像提供方布局会被改写，未知自定义镜像保持原样。
缺少或无法识别的原生配置会拒绝处理，而不是猜测配置；修复时请先恢复发行版自身的源定义。
pacman 专用镜像列表还会识别原生 `$repo`/`$arch` URL 模板，不限于特定镜像站域名。
新增 Gentoo 设置、NixOS 模块和 Void 覆盖文件属于下文说明的例外，原先不存在的文件在
恢复时会删除。

## 归档与特殊仓库

- Debian Jessie/Stretch/Buster 分别使用基础和安全归档仓库；旧 Ubuntu 版本代号使用
  `ubuntu-old`。保留版本通道及签名策略，归档元数据可能过期；客户端不会添加
  `trusted=yes`、关闭签名或有效期检查，也不会安装替代密钥。
- CentOS 7/8 使用 Vault，将通用 `$releasever` 路径固定为 7.9.2009 / 8.5.2111。
  Fedora 30–42 使用归档树，43–44 保留活动仓库。这是目录中的仓库位置快照，上游迁移后
  应复查；归档版本不再获得正常的厂商安全更新。
- RHEL 不提供默认公开上游。请在 `upstreams.additional_os.rhel` 配置经过授权、保持
  `/content/...` 层级且兼容的客户镜像。代理不转发客户端订阅凭据，也不配置上游客户端
  证书；仅接受订阅认证的 Red Hat CDN 不能直接用于此适配器。不会把 RHEL 转成 CentOS。
- Proxmox 企业/订阅源保持原样，公开 PVE/Ceph 源可以迁移；不会修改订阅、Perl 模块、
  容器模板或企业源策略。
- Gentoo 只修改 `make.conf` 中的 `GENTOO_MIRRORS`，保留其他设置、同步方式和信任控制。
  随后执行正常同步和获取命令；多行 shell 赋值需要人工审阅。Rsync/Git 同步不通过 HTTP
  代理隧道转发。
- NixOS 生成 `/etc/nixos/mirrorproxy.nix`，将 `./mirrorproxy.nix` 加入配置的 `imports`
  后执行 `sudo nixos-rebuild switch`。原有可信公钥不变，保留官方缓存。已有 root
  `.nix-channels` 中的 HTTP 地址也可以迁移。不修改 flake 输入、不求值 Nix。
  回退时移除 import，执行 `reset system`，再 rebuild；使用 channel 时还需刷新 channel。
- openSUSE 上的 RIS 等仓库服务可能重新生成 `.repo` 文件；需要时请先用原生 Zypper
  工具管理对应服务。客户端不会卸载服务软件包或修改其服务策略。

## 安全、服务端升级与回滚

`--dry-run` 校验实际内容并展示变化行，不写文件和回滚记录，敏感行会隐藏。
写入前将文件及原始内容记录到 `/var/lib/mirrorproxy/sources/system.json`。
恢复路径限制在该发行版允许的文件内，拒绝配置文件或目录中的符号链接，Unix 上保留
文件权限。恢复前先检查所有路径与文件，迁移后的人工作业必须显式传入 `--force` 才能覆盖。

每个文件单独原子写入，整个文件集合不是单个文件系统事务。中断后可执行
`reset system --scope system` 恢复部分写入。完成恢复前保留此客户端版本，旧版本无法
读取新增的多文件记录。应用和恢复后都需刷新原生包索引。客户端不迁移数据库，也不部署
服务端。

新公开仓库根地址已加入服务端默认值。如果 TOML 显式配置了 `upstreams.additional_os`，
请合并 `config.example.toml` 中的新键并保留自定义覆盖；显式映射会替换默认映射。
`upstreams.debian`、`upstreams.fedora` 等固定根地址继续使用原配置字段。
预览成功不代表已经验证远程连接。

隔离测试可增加 `--config-root <目录>`，系统识别、原生配置和回滚状态均位于该目录下。
默认系统范围为 Linux，其他平台必须显式提供测试根目录。

## 在 Linux 验证

迁移后检查有效配置和真实下载：

| 适配器 | 验证示例 |
| --- | --- |
| APT | `apt-get update`；`apt-cache policy hello`；`apt-get --print-uris download hello`；`apt-get download hello` |
| RPM | `dnf repolist -v`；`dnf --refresh makecache`；通过 DNF 下载软件包 |
| Zypper | `zypper lr -u`；`zypper refresh`；下载软件包 |
| pacman | 在一次性环境运行 `pacman -Syyw --noconfirm <包名>` |
| apk | `apk update`；`apk fetch busybox` |
| XBPS | `xbps-query -L`；`xbps-install -S`；在容器内获取/安装小型软件包 |
| Gentoo | 正常同步后运行 `emerge --fetchonly <包名>` |
| NixOS | 导入/rebuild、检查 substituter、获取经过签名验证的 store 路径 |

检查实际选中的 URI 及生态自身的签名/摘要校验，不只看 HTTP 状态或生成文本；保留的
第三方仓库仍可能提供优先候选版本。恢复后比较原始文件校验和。

一次性容器烟测由操作者在 Linux 执行：

```bash
cargo build --locked --release -p mirrorproxy-cli --target x86_64-unknown-linux-musl
MIRRORPROXY_OS_SMOKE_BASE=https://mirror.example.com \
MIRRORPROXY_OS_SMOKE_CLIENT_BINARY="$PWD/target/x86_64-unknown-linux-musl/release/mirrorproxy" \
MIRRORPROXY_OS_SMOKE_TARGETS="debian ubuntu fedora archlinux alpine opensuse void gentoo" \
bash scripts/smoke-os-clients.sh
```

该脚本验证代表性的包管理器家族，不覆盖矩阵中的每个版本。其他发行版和版本的镜像在
生产使用前也需要独立真实验证。容器执行及软件包下载需要网络。
musl 构建需要安装对应 Rust target 和 musl 工具链，ARM 主机应选择匹配架构。
可选烟测目标还包括 `rocky`、`alma`、`oraclelinux`。可用
`MIRRORPROXY_OS_SMOKE_DEBIAN_IMAGE=debian:trixie-slim` 或
`MIRRORPROXY_OS_SMOKE_FEDORA_IMAGE=fedora:43` 等变量更换家族镜像、验证其他版本。

[English](System-Sources)
