# 部署：先运行服务，再接入 HTTPS

| 你的情况 | 从这里开始 |
| --- | --- |
| 新手，想用 Docker 一次配好 HTTPS | [Docker + Caddy 快速开始](Getting-Started-zh-CN) |
| 已有宿主机 Nginx/Caddy 等入口 | 本页第一步 A，再选反向代理 |
| 不使用 Docker | 本页第一步 B，下载二进制 |
| 不使用任何反向代理 | [MirrorProxy 原生 HTTPS](ACME-Certificates-zh-CN) |

> **v1.4.4 支持 HTTP 密码登录，并持续显示不安全连接提醒。** 密码和会话在 HTTP 下可能被窃听或篡改，推荐按下方步骤启用 HTTPS。旧版登录问题见[常见问题](Troubleshooting-zh-CN)。

以下使用 `mirror.example.com`，请替换为你的真实域名，并让 DNS 指向服务器。

## 第一步 A：Docker 后端，接宿主机反向代理

**如果已采用快速开始的 Docker + Caddy，就跳过本节。** 以下适用于反代软件装在宿主机的 Linux 服务器。
创建 `compose.yml`：

```yaml
services:
  mirrorproxy:
    image: kudang/mirrorproxy:1.4.4
    restart: unless-stopped
    ports:
      - '127.0.0.1:3000:3000'
    environment:
      MIRRORPROXY_ADMIN_PASSWORD: 'replace-with-a-long-unique-password'
      MIRRORPROXY_PUBLIC_BASE_URL: 'https://mirror.example.com'
      MIRRORPROXY_TRUSTED_PROXIES: '172.30.40.1'
    volumes:
      - mirrorproxy-data:/data
networks:
  default:
    ipam:
      config:
        - subnet: 172.30.40.0/24
          gateway: 172.30.40.1
volumes:
  mirrorproxy-data:
```

```bash
docker compose config --quiet
docker compose up -d
curl -f http://127.0.0.1:3000/healthz
```

后台端口只发布到宿主机回环地址，不直接对公网开放。容器把本示例的宿主机网桥地址
`172.30.40.1` 作为可信代理。网段冲突时同步修改网段、网关和可信地址；不要照搬到其他网络。
已有数据库的运行设置可能由后台保存，可信代理和公开地址也要在后台核对。

## 第一步 B：直接运行二进制

适用于**全新 Linux x86_64 安装**，无需安装 Rust 或 Node.js。先确认架构：`uname -m`。
ARM64 请把下面的归档名换成 `mirrorproxy-server-aarch64-unknown-linux-gnu.tar.gz`。

下载、校验并解压，以下命令分步执行，校验失败时停止：

```bash
mkdir -p "$HOME/mirrorproxy"
cd "$HOME/mirrorproxy"
version=v1.4.4
archive=mirrorproxy-server-x86_64-unknown-linux-musl.tar.gz
curl -fLO "https://github.com/inbjo/MirrorProxy/releases/download/$version/$archive"
curl -fLO "https://github.com/inbjo/MirrorProxy/releases/download/$version/$archive.sha256"
sha256sum -c "$archive.sha256" && tar -xzf "$archive"
```

在 `~/mirrorproxy/config.toml` 新建最小配置，先替换域名。其余设置使用服务默认值；完整字段见随包的 `config.example.toml`。

```toml
listen_addr = "127.0.0.1:3000"
public_base_url = "https://mirror.example.com"
database_path = "mirrorproxy.sqlite3"
trusted_proxies = ["127.0.0.1", "::1"]
```

从该目录启动，填写自己的初始密码：

```bash
MIRRORPROXY_ADMIN_PASSWORD='replace-with-a-long-unique-password'   ./mirrorproxy-server --config ./config.toml serve
```

保持这个终端运行，在另一终端验证 `curl -f http://127.0.0.1:3000/healthz`。
接着配置下面的 HTTPS；全部正常后，按[systemd 常驻](Advanced-Deployment-zh-CN#systemd)改为后台服务。
**不要安装成客户端 `mirrorproxy`，也不要只复制可执行文件而遗漏 `geoip/`。**

## 第二步：选择反向代理，启用 HTTPS

只选一种；Caddy、Nginx、Traefik 等不能同时占用同一台机器的 80/443。

| 软件 | 完整配置 |
| --- | --- |
| Caddy（自动证书，最省步骤） | [Caddy 示例](Reverse-Proxy-zh-CN#caddy) |
| Nginx / OpenResty / 使用 Nginx 的面板 | [Nginx 示例与证书申请](Reverse-Proxy-zh-CN#nginx) |
| Traefik | [Traefik 静态和动态配置](Reverse-Proxy-zh-CN#traefik) |
| Apache HTTP Server | [Apache 虚拟主机配置](Reverse-Proxy-zh-CN#apache) |
| HAProxy | [HAProxy TLS 与后端配置](Reverse-Proxy-zh-CN#haproxy) |
| Nginx Proxy Manager | [NPM 界面填写示例](Reverse-Proxy-zh-CN#npm) |

例如，在**宿主机**上运行的 Caddy 使用以下 `Caddyfile`：

```caddyfile
mirror.example.com {
    reverse_proxy 127.0.0.1:3000
}
```

域名解析正确且公网 80/443 可达时，Caddy 会自动处理证书和 HTTP 到 HTTPS 的跳转。
安装、配置文件位置及重载命令见[完整示例](Reverse-Proxy-zh-CN#caddy)。

> **反代也在容器里？** 容器内的 `127.0.0.1` 指它自己。请使用共享网络中的服务名，
> 如快速开始的 `mirrorproxy:3000`，并只信任实际反代容器的地址。

## 第三步：验证并登录

```bash
curl -f https://mirror.example.com/healthz
curl -f https://mirror.example.com/version
curl -f https://mirror.example.com/npm/is-number
```

打开 **`https://mirror.example.com/admin`**，使用 `admin` 和初始化密码登录，再刷新页面。
这同时验证了证书、反代、真实包代理和登录会话。不要用 `curl -k` 跳过证书问题。

## 日常操作与进阶

- [常见问题](Troubleshooting-zh-CN)：登录、502、证书、权限和配置不生效。
- [进阶部署](Advanced-Deployment-zh-CN)：systemd、升级回滚、备份、独立管理端口、企业 CA、用户子域名。
- [原生 HTTPS / ACME](ACME-Certificates-zh-CN)：不用反代时自行签发证书。

[Caddy 官方说明](https://caddyserver.com/docs/automatic-https) · [English](Deployment)
