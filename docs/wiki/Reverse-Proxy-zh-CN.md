# 常见反向代理：HTTPS 配置示例

先按[部署](Deployment-zh-CN)启动后端，再选下列一种软件。**普通部署只需一个域名和一个代理，不必开启 MirrorProxy ACME。**

| 软件 | 证书处理 | 跳转 |
| --- | --- | --- |
| Caddy | 自动申请与续期 | [配置](#caddy) |
| Nginx / OpenResty | 已有证书，或使用 Certbot | [配置](#nginx) |
| Traefik v3 | 内置 ACME | [配置](#traefik) |
| Apache HTTP Server 2.4 | 已有证书，或使用 Certbot | [配置](#apache) |
| HAProxy | 加载 PEM，外部工具管理证书 | [配置](#haproxy) |
| Nginx Proxy Manager | 界面申请证书 | [界面示例](#npm) |

## 共用前提

- 将 `mirror.example.com` 全部替换为真实域名，DNS 指向入口，开放所选服务的公网 80/443。
- 以下文件示例假设**反代运行在宿主机**，后端为 `http://127.0.0.1:3000`，整站转发，保留路径和查询参数，不添加 `/mirrorproxy/` 前缀。
- 二进制后端信任回环地址；部署页的 Docker 后端信任固定网桥 `172.30.40.1`。**只信任实际反代，不要填 `0.0.0.0/0`。**
- 反代若在容器内，用共享网络服务名替换 `127.0.0.1`，并配置反代容器的固定可信地址；完整例子见[快速开始](Getting-Started-zh-CN)。
- 已有站点时，将示例合并到现有配置，先备份、检查语法再重载，不要覆盖其他站点。

## Caddy

先按 [Caddy 安装说明](https://caddyserver.com/docs/install)安装。使用官方 systemd 服务时，编辑 `/etc/caddy/Caddyfile`：

```caddyfile
mirror.example.com {
    reverse_proxy 127.0.0.1:3000
}
```

```bash
sudo caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile
sudo systemctl reload caddy
```

Caddy 自动获取、续期证书并把 HTTP 跳转到 HTTPS，无需再运行 Certbot。
首次运行还未启动服务时用 `sudo systemctl enable --now caddy`。
[自动 HTTPS 说明](https://caddyserver.com/docs/automatic-https)

## Nginx

Nginx/OpenResty 共用下面的代理指令。常见 Debian/Ubuntu Nginx 可在 `/etc/nginx/conf.d/mirrorproxy.conf` 新建站点；
宝塔、1Panel 等面板应编辑对应站点的反向代理配置，不要重复创建同名站点。

**没有证书：先启用 HTTP 站点，再由 Certbot 开启 HTTPS。** 初始站点内容：

```nginx
server {
    listen 80;
    server_name mirror.example.com;
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Host $http_host;
        proxy_set_header X-Forwarded-Host $http_host;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header X-Forwarded-For $remote_addr;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header Connection "";
        proxy_buffering off;
        proxy_read_timeout 300s;
    }
}
```

先验证并加载，再按 [Certbot 官方安装说明](https://certbot.eff.org/instructions?ws=nginx&os=snap)安装 Certbot，执行：

```bash
sudo nginx -t
sudo systemctl reload nginx
sudo certbot --nginx --redirect -d mirror.example.com
sudo certbot renew --dry-run
```

Certbot 会写入 HTTPS 监听、证书路径和 HTTP 跳转。OpenResty 或面板自带 Nginx 的可执行路径、服务名可能不同，
请用面板证书功能或与该安装匹配的证书工具，不要盲目运行系统 `nginx` 命令。

<details>
<summary>已有证书：完整的 80/443 站点示例</summary>

证书文件必须已存在，路径请按实际情况替换：

```nginx
server {
    listen 80;
    server_name mirror.example.com;
    return 301 https://mirror.example.com$request_uri;
}
server {
    listen 443 ssl;
    server_name mirror.example.com;
    ssl_certificate /etc/letsencrypt/live/mirror.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/mirror.example.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Host $http_host;
        proxy_set_header X-Forwarded-Host $http_host;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header X-Forwarded-For $remote_addr;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header Connection "";
        proxy_buffering off;
        proxy_read_timeout 300s;
    }
}
```

保存后执行 `sudo nginx -t`，成功才执行 `sudo systemctl reload nginx`。

</details>

示例覆盖客户端传入的协议与来源地址头，Host 保留原始端口；`proxy_pass` 不追加路径，避免破坏包下载链接。
证书续期后需要重载 Nginx。[代理指令说明](https://nginx.org/en/docs/http/ngx_http_proxy_module.html)

## Traefik

适用于宿主机上的 Traefik v3，使用 file provider，不挂载 Docker socket。
在 `/etc/traefik/traefik.yml` 合并静态配置，替换邮箱：

```yaml
entryPoints:
  web:
    address: ':80'
    http:
      redirections:
        entryPoint:
          to: websecure
          scheme: https
  websecure:
    address: ':443'
providers:
  file:
    filename: /etc/traefik/mirrorproxy.yml
    watch: true
certificatesResolvers:
  letsencrypt:
    acme:
      email: admin@example.com
      storage: /var/lib/traefik/acme.json
      httpChallenge:
        entryPoint: web
```

在 `/etc/traefik/mirrorproxy.yml` 写入动态路由：

```yaml
http:
  routers:
    mirrorproxy:
      rule: 'Host(`mirror.example.com`)'
      entryPoints: [websecure]
      service: mirrorproxy
      tls:
        certResolver: letsencrypt
  services:
    mirrorproxy:
      loadBalancer:
        passHostHeader: true
        servers:
          - url: 'http://127.0.0.1:3000'
```

运行 Traefik 的用户必须能写入 `/var/lib/traefik`；`acme.json` 仅该用户可读写（0600），并应持久保存。
静态配置变更后重启服务；已有 systemd 服务可用 `sudo systemctl restart traefik`。
若前台运行，可用 `traefik --configFile=/etc/traefik/traefik.yml`，绑定 80/443 需相应权限。
Traefik 会签发证书并保留 Host、传递协议和来源地址。不要启用 `forwardedHeaders.insecure`。
[ACME 配置说明](https://doc.traefik.io/traefik/reference/install-configuration/tls/certificate-resolvers/acme/)

## Apache

适用于 Apache HTTP Server 2.4，需启用 `ssl`、`headers`、`proxy`、`proxy_http` 和 `alias` 模块。
Debian/Ubuntu 可执行 `sudo a2enmod ssl headers proxy proxy_http alias`。
先准备有效证书；没有证书时按 [Certbot Apache 指南](https://certbot.eff.org/instructions?ws=apache&os=snap)
为已有 HTTP 站点签发证书，再使用下面的完整配置。

保存为 `/etc/apache2/sites-available/mirrorproxy.conf`，合并实际证书路径：

```apache
<VirtualHost *:80>
    ServerName mirror.example.com
    Redirect permanent / https://mirror.example.com/
</VirtualHost>
<VirtualHost *:443>
    ServerName mirror.example.com
    SSLEngine on
    SSLCertificateFile /etc/letsencrypt/live/mirror.example.com/fullchain.pem
    SSLCertificateKeyFile /etc/letsencrypt/live/mirror.example.com/privkey.pem
    SSLProtocol -all +TLSv1.2 +TLSv1.3
    ProxyRequests Off
    ProxyPreserveHost On
    ProxyAddHeaders Off
    RequestHeader set X-Forwarded-Proto "https"
    RequestHeader set X-Forwarded-For "expr=%{REMOTE_ADDR}"
    RequestHeader unset X-Forwarded-Host
    RequestHeader unset X-Real-IP
    AllowEncodedSlashes NoDecode
    ProxyPass / http://127.0.0.1:3000/ nocanon
    ProxyPassReverse / http://127.0.0.1:3000/
    ProxyTimeout 300
</VirtualHost>
```

```bash
sudo a2ensite mirrorproxy
sudo apachectl configtest
sudo systemctl reload apache2
```

其他发行版的目录和服务名可能是 `httpd`。证书续期后重载 Apache。
配置关闭正向代理，只把整站请求转给 MirrorProxy，并覆盖客户端伪造的来源地址。
[Apache 代理说明](https://httpd.apache.org/docs/2.4/mod/mod_proxy.html)

## HAProxy

先准备域名证书。HAProxy 的 PEM 文件同时包含证书链和私钥，不能只填写 `fullchain.pem`：

```bash
sudo install -d -m 0700 /etc/haproxy/certs
sudo sh -c 'umask 077; cat /etc/letsencrypt/live/mirror.example.com/fullchain.pem /etc/letsencrypt/live/mirror.example.com/privkey.pem > /etc/haproxy/certs/mirror.pem'
```

以下是独立实例的 `/etc/haproxy/haproxy.cfg` 示例；已有实例只合并相关 frontend/backend：

```haproxy
global
    log stdout format raw local0
    ssl-default-bind-options ssl-min-ver TLSv1.2

defaults
    mode http
    timeout connect 10s
    timeout client 300s
    timeout server 300s

frontend mirror_http
    bind :80
    http-request redirect scheme https code 301

frontend mirror_https
    bind :443 ssl crt /etc/haproxy/certs/mirror.pem
    http-request set-header X-Forwarded-Proto https
    http-request set-header X-Forwarded-For %[src]
    http-request del-header X-Forwarded-Host
    http-request del-header X-Real-IP
    default_backend mirrorproxy

backend mirrorproxy
    server app 127.0.0.1:3000 check
```

```bash
sudo haproxy -c -f /etc/haproxy/haproxy.cfg
sudo systemctl reload haproxy
```

外部证书工具续期后，需要重新生成 `mirror.pem`，再检查配置并重载 HAProxy；不能只更新原始证书文件。
[HAProxy TLS 示例](https://www.haproxy.com/documentation/haproxy-configuration-tutorials/protocol-support/http/)

## NPM

Nginx Proxy Manager 的 **Proxy Hosts → Add Proxy Host** 中填写：

| 字段 | 示例 |
| --- | --- |
| Domain Names | `mirror.example.com` |
| Scheme | `http` |
| Forward Hostname / IP | 宿主机反代填 `127.0.0.1`；共享容器网络填 `mirrorproxy` |
| Forward Port | `3000` |
| SSL Certificate | Request a new SSL Certificate，或选择已有有效证书 |
| Force SSL | 开启 |

NPM 通常运行在容器中，因此最常用的是**同一网络中的 `mirrorproxy:3000`**。
不要照抄 `127.0.0.1`；MirrorProxy 的可信代理应为 NPM 的实际固定容器地址。
NPM 与 Caddy/Nginx 不能同时占用同一组 80/443 端口。保存后直接访问 HTTPS 域名。
[NPM 官方指南](https://nginxproxymanager.com/guide/)

## 最后检查

```bash
curl -f https://mirror.example.com/healthz
curl -f https://mirror.example.com/npm/is-number
```

再登录 `/admin` 并刷新页面。若 502、证书或登录失败，按[常见问题](Troubleshooting-zh-CN)逐项排查。

[English](Reverse-Proxy) · [返回部署](Deployment-zh-CN)
