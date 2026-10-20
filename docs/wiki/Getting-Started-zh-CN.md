# 快速开始：Docker + Caddy

这条路径会启动 MirrorProxy，并由 Caddy 自动申请、续期 HTTPS 证书。
已有 Nginx 等入口，或不使用 Docker？看[部署](Deployment-zh-CN)。

> **准备好域名、Docker Compose，以及空闲的 80/443 端口。** 把下文所有 `mirror.example.com` 换成你的域名，A/AAAA 记录必须正确指向这台服务器。不要让其他 Web 服务同时占用这些端口。

## 1. 创建两个文件

在新目录中创建 `compose.yml`，修改域名和初始密码：

```yaml
services:
  mirrorproxy:
    image: kudang/mirrorproxy:1.4.4
    restart: unless-stopped
    environment:
      MIRRORPROXY_ADMIN_PASSWORD: 'replace-with-a-long-unique-password'
      MIRRORPROXY_PUBLIC_BASE_URL: 'https://mirror.example.com'
      MIRRORPROXY_TRUSTED_PROXIES: '172.30.40.2'
    volumes:
      - mirrorproxy-data:/data
    networks:
      default:
        ipv4_address: 172.30.40.3
  caddy:
    image: caddy:2
    restart: unless-stopped
    ports:
      - '80:80'
      - '443:443'
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy-data:/data
      - caddy-config:/config
    networks:
      default:
        ipv4_address: 172.30.40.2
volumes:
  mirrorproxy-data:
  caddy-data:
  caddy-config:
networks:
  default:
    ipam:
      config:
        - subnet: 172.30.40.0/24
          gateway: 172.30.40.1
```

同目录创建 `Caddyfile`：

```caddyfile
mirror.example.com {
    reverse_proxy mirrorproxy:3000
}
```

这里固定了两项容器地址，MirrorProxy 只信任 Caddy 的 `172.30.40.2`。
如果示例网段与你的网络冲突，换一个未使用的网段，并同步调整网关、两个地址和可信代理。

## 2. 启动并打开后台

```bash
docker compose config --quiet
docker compose up -d
docker compose logs --tail=50 caddy mirrorproxy
```

证书签发成功后，打开 **`https://mirror.example.com/admin`**，账号为 `admin`，密码为刚才填写的值。
如果失败，先看 [HTTPS 排障](Troubleshooting-zh-CN)，不要删除数据卷。

> **初始密码只使用一次。** 修改 `MIRRORPROXY_ADMIN_PASSWORD` 不会覆盖数据库里已有的管理员密码。
> **请保留数据卷。** `mirrorproxy-data` 保存数据库等数据，`caddy-data` 保存证书；不要执行 `docker compose down -v`。

## 3. 验证一次真实代理请求

```bash
curl -f https://mirror.example.com/healthz
curl -f https://mirror.example.com/npm/is-number
```

第一条验证服务，第二条应返回 npm 包元数据。再通过浏览器登录并刷新后台，确认会话正常。
单独看到 HTTP 200 不代表整个代理链路都正常。

## 接下来

- 本机改源：[客户端](Client-zh-CN)。
- 配置 Docker 拉取：[容器镜像](Container-Registries-and-Runtimes-zh-CN)。
- 配额、管理员和监控：[后台管理](Administration-zh-CN)。
- 升级、备份和 systemd：[进阶部署](Advanced-Deployment-zh-CN)。

[Caddy 官方说明](https://caddyserver.com/docs/automatic-https) · [English](Getting-Started)
