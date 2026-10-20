# MirrorProxy 使用指南

MirrorProxy 把常用代码源、软件包和公开容器镜像集中到自己的域名，按需代理和缓存。

> **第一次使用？** 先看[快速开始](Getting-Started-zh-CN)。已有实例、只想给电脑改源，直接看[客户端](Client-zh-CN)。

## 从你的需求开始

| 我想做什么 | 阅读这里 |
| --- | --- |
| 用 Docker 部署，自动开启 HTTPS | [快速开始](Getting-Started-zh-CN) |
| 下载二进制运行，或使用已有反向代理 | [部署](Deployment-zh-CN) |
| 配置 Nginx、Caddy、Traefik、Apache、HAProxy | [反向代理示例](Reverse-Proxy-zh-CN) |
| 让 npm、pip、Cargo 等使用我的实例 | [客户端](Client-zh-CN) |
| 加速 Docker Hub、GHCR 等镜像拉取 | [容器镜像](Container-Registries-and-Runtimes-zh-CN) |
| 修改密码、设置配额、查看流量 | [后台管理](Administration-zh-CN) |
| 无法登录、证书错误、下载失败 | [常见问题](Troubleshooting-zh-CN) |

## 先记住这三点

- **服务端是 `mirrorproxy-server`，客户端是 `mirrorproxy`。** 客户端安装脚本不会部署服务端。
- **推荐使用 HTTPS。** v1.4.4 支持 HTTP 密码登录，但会持续显示不安全连接提醒；不要通过删除数据库解决登录问题。
- **Docker mirror 只覆盖 Docker Hub。** 其他受支持 Registry 需要改写镜像地址；服务不接受镜像推送。

## 用到时再看

[支持的软件源](Proxy-Adapters-zh-CN) · [客户端安装方式](Distribution-zh-CN) ·
[进阶部署](Advanced-Deployment-zh-CN) · [原生 HTTPS / ACME](ACME-Certificates-zh-CN) ·
[GeoIP 与 IP 规则](GeoIP-and-IP-Access-Control-zh-CN) · [开发](Development-and-Roadmap-zh-CN)

[English](Home)
