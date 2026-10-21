# 进阶部署与维护

**先完成[基础部署](Deployment-zh-CN)和 HTTPS 登录，再按需要配置本页。**
独立管理端口、企业 CA、用户子域名都不是初次部署的必选项。

## systemd

适用于已按部署页验证过的全新 Linux 二进制安装。先 `Ctrl+C` 停止前台进程。
确认 **`/opt/mirrorproxy` 不存在、`mirrorproxy` 系统用户尚未创建**，再迁移整个目录并安装服务：

```bash
sudo useradd --system --home-dir /opt/mirrorproxy --shell /usr/sbin/nologin mirrorproxy
sudo mv -T "$HOME/mirrorproxy" /opt/mirrorproxy
sudo chown -R mirrorproxy:mirrorproxy /opt/mirrorproxy
sudo /opt/mirrorproxy/mirrorproxy-server --config /opt/mirrorproxy/config.toml install   --working-directory /opt/mirrorproxy --enable --start
sudo systemctl status mirrorproxy --no-pager
```

目录迁移包含已有数据库和 GeoIP；不重新初始化密码。`mv -T` 是 GNU/Linux 命令，目标目录非空时会拒绝覆盖。
已有部署不要重跑这组首次安装命令；先核对实际服务路径和数据目录。
生成的服务以专用用户运行；日志用 `sudo journalctl -u mirrorproxy -n 100 --no-pager` 查看。

- 预览 unit：安装命令加 `--dry-run`，不写入文件、不启动服务。
- 原生 HTTPS 绑定 80/443：安装命令加 `--privileged-ports`，只授予绑定低端口的能力。
- 服务中使用主密钥等环境变量时，通过受限的 systemd 配置注入，不要把密钥放到命令行或提交到仓库。

## 升级、备份与回退

**先备份数据和配置，验证新版本，再替换程序。** 缓存、GeoIP、ACME 目录也应保留。

Docker：修改 `compose.yml` 中固定的镜像版本，再执行：

```bash
docker compose pull mirrorproxy
docker compose up -d mirrorproxy
docker compose logs --tail=50 mirrorproxy
```

不执行 `down -v`，不删除旧数据卷。升级前备份数据库和配置，记录原镜像版本；回退时使用记录的版本，保留原卷。

二进制部署可生成一致的 SQLite 备份（目标文件必须尚不存在）：

```bash
sudo install -d -o mirrorproxy -g mirrorproxy /var/backups/mirrorproxy
sudo -u mirrorproxy /opt/mirrorproxy/mirrorproxy-server   --config /opt/mirrorproxy/config.toml backup /var/backups/mirrorproxy/before-upgrade.sqlite3
```

另外保存 `config.toml`、证书目录以及主密钥。下载匹配架构的新发布包并核验 SHA-256，
停止服务，保留旧可执行文件，替换后重启。使用 `/healthz`、`/version`、后台刷新和一次真实下载验证。

数据库恢复**必须先停止服务**；按实际路径执行 `mirrorproxy-server --config <配置> restore <备份> --force`。
命令会验证备份并保留被替换的数据。旧二进制未必兼容新数据库结构，跨版本回退前要确认兼容性。

## 配置从哪里生效

- 监听地址、服务路径等启动设置来自实际 `--config` 和对应环境变量，修改后重启。
- 一部分运行设置在首次启动时写入 SQLite，之后在后台修改；重启不会把所有 TOML 值重新覆盖进去。
- 显式的出站代理、上游 TLS 环境变量会在启动时覆盖对应已存储设置。
- `MIRRORPROXY_ACME_*` 等 ACME 环境变量会让对应后台配置只读。
- `MIRRORPROXY_ADMIN_PASSWORD` 仅初始化管理员，不是持续同步密码的配置项。

更改前确认使用的是哪个配置文件、数据卷和实例，见[常见问题](Troubleshooting-zh-CN)。

## 独立管理端口与指标

可选增加仅本机访问的管理入口：

```toml
[management]
enabled = true
listen_addr = "127.0.0.1:3001"
```

重启生效。它**不会关闭公网监听器上的 `/admin`**，仍需认证和正常的 HTTPS 入口。
`/metrics` 默认仅允许本机来源访问；需远程采集时显式设置 `metrics.local_only = false`，并在防火墙或入口限制来源。

## 企业 CA 与上游 TLS

只在企业上游使用私有 CA 时配置：

```toml
[upstream_tls]
ca_certificates = ["/etc/mirrorproxy/ca/company-root.pem"]
insecure_skip_verify = false
```

Docker 还要挂载对应文件：

```yaml
volumes:
  - ./company-root.pem:/etc/mirrorproxy/ca/company-root.pem:ro
```

对应环境变量是 `MIRRORPROXY_UPSTREAM_TLS_CA_CERTIFICATES`（多个路径以逗号分隔）和
`MIRRORPROXY_UPSTREAM_TLS_INSECURE_SKIP_VERIFY`。默认信任公共根及系统根证书。
这些设置只影响镜像上游，不影响 ACME、DNS API、OAuth 等请求。不要通过关闭证书校验修复部署问题。

## 用户子域名与多层入口

`subdomain_required` 是可选的按用户路由/计费模式。开启前准备通配符 DNS、通配符 TLS、保留 Host 的入口，
并在后台确认基础设施就绪。否则主域名上的包路径可能按策略拒绝。
多层反代只信任实际中间节点，来源地址从右向左解析，见[GeoIP 与 IP 规则](GeoIP-and-IP-Access-Control-zh-CN)。

[English](Advanced-Deployment) · [基础部署](Deployment-zh-CN)
