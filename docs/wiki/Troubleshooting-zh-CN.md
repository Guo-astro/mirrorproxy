# 常见问题

先确认正在访问哪个实例：

```bash
curl -f https://mirror.example.com/version
```

Docker 查看 `docker compose logs --tail=100 mirrorproxy caddy`；systemd 查看
`journalctl -u mirrorproxy -n 100 --no-pager`。分享日志前遮盖密码、Token 和 Cookie。

## 密码正确，后台仍退回登录页

**v1.4.3 请通过 `https://你的域名/admin` 登录。** Docker 和二进制都有同样的 HTTP 会话限制。

登录返回 200 后，浏览器仍可能拒绝 `Secure` 会话 Cookie 和带 `__Host-` 前缀的 CSRF Cookie；
后续请求返回 401，页面误提示密码错误。不是初始密码强度不够，也不能通过重置密码或删除数据库解决。
localhost 的特殊处理不能代表远端 IP/域名的行为。

**v1.4.4 已修复：支持 HTTP 密码登录，并持续显示不安全连接横幅，引导使用 HTTPS。** HTTP 和 HTTPS 使用独立会话 Cookie；切换到 HTTPS 后需要重新登录。HTTP 下密码和会话仍可能被窃听或篡改，推荐启用 HTTPS。
参见 [Issue #10](https://github.com/inbjo/MirrorProxy/issues/10) 和[反向代理示例](Reverse-Proxy-zh-CN)。

HTTPS 仍失败时，在浏览器开发者工具查看登录状态码、Cookie 是否被拒绝及后续 `/admin/api/` 请求。
同时确认反代保留 Host、传入正确协议，并只把实际反代地址加入 `trusted_proxies`。

## 修改环境变量密码，没有生效

`MIRRORPROXY_ADMIN_PASSWORD` **仅在数据库没有管理员时使用**，非空值原样采用。
已有管理员时，重建容器也不会覆盖密码。空值或未设置会生成随机初始密码，只在首次日志中显示。

忘记密码时，在服务器本地执行（输出的是新的随机密码，请妥善保存）：

```bash
# Docker
docker compose exec mirrorproxy mirrorproxy-server admin reset-password

# 二进制常驻服务
sudo -u mirrorproxy /opt/mirrorproxy/mirrorproxy-server   --config /opt/mirrorproxy/config.toml admin reset-password
```

该命令重置最初创建的管理员，并撤销其会话。后台修改密码仍有强度要求。
连续失败五次后账号会锁定 15 分钟；普通重启不清除锁定，不要反复尝试。

## HTTPS、502 和下载问题

| 现象 | 先检查 |
| --- | --- |
| 证书签发失败 | A/AAAA 是否正确、80/443 是否到达签发服务、端口是否被占用 |
| 浏览器不信任证书 | 域名与证书是否匹配、证书链是否完整；企业 CA 是否装入客户端信任库 |
| 502 Bad Gateway | 宿主机访问后端 `/healthz`；容器反代不要连接自身的 `127.0.0.1` |
| 后台 CSRF 错误 | Host、协议头、可信代理与浏览器实际 HTTPS 地址是否一致 |
| Docker Hub 能拉，GHCR 不走代理 | `registry-mirrors` 仅覆盖 Docker Hub，其他镜像需要改写地址 |
| 镜像 401/403 | 401 可能是正常认证挑战；检查镜像是否公开、是否要求登录或许可 |
| 404 或 manifest unknown | 检查上游包名、仓库、标签与启用的代理目标，不要猜 URL |
| 地域为 Unknown | 可能是私网 IP、无 XDB 数据或可信代理配置不正确 |

## 改了配置文件，却没有变化

首次运行会把一部分运行配置保存到 SQLite，之后通常由后台管理；不是所有 TOML 字段都会在重启时覆盖数据库。
监听器设置由服务启动配置决定；ACME 环境变量等有单独的优先级。
先核对进程实际使用的 `--config` 路径、容器环境、后台已保存值，再按[进阶部署](Advanced-Deployment-zh-CN)处理。

> **保留数据，先定位原因。** 不要删除 SQLite、运行 `docker compose down -v`，或关闭 TLS/SSRF/CSRF 检查来排障。

[English](Troubleshooting) · [部署](Deployment-zh-CN)
