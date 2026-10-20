# 容器镜像：先拉取，再按需配置运行时

> **`registry-mirrors` 只覆盖 Docker Hub。** GHCR、Quay、MCR 等需要在镜像名中带上 MirrorProxy 域名和原 Registry。仅支持公开镜像拉取，不支持推送。

## 直接使用（不用改运行时配置）

替换 `mirror.example.com` 后执行：

```bash
docker pull mirror.example.com/library/busybox:1.36.1
docker pull mirror.example.com/registry.k8s.io/pause:3.10
```

首页“容器 Registry 配置台”可以转换镜像名、Compose 和 Dockerfile；先检查转换结果，再实际拉取。

## 按平台配置

| 你的环境 | 详细步骤 |
| --- | --- |
| Docker Engine、Docker Desktop、OrbStack | [Docker 配置](Docker-Engine-and-Desktop-zh-CN) |
| K3s、containerd | [节点配置](Containerd-and-K3s-zh-CN) |
| NAS、Portainer、Podman、HPC、Harbor/Nexus | [平台说明](NAS-Panels-and-Other-Runtimes-zh-CN) |

## 支持范围

| Registry | 原始镜像 | MirrorProxy 路径 |
| --- | --- | --- |
| Docker Hub | `nginx:latest` | `<本站域名>/nginx:latest` |
| GHCR | `ghcr.io/owner/image:tag` | `<本站域名>/ghcr.io/owner/image:tag` |
| Quay | `quay.io/owner/image:tag` | `<本站域名>/quay.io/owner/image:tag` |
| Kubernetes | `registry.k8s.io/pause:3.10` | `<本站域名>/registry.k8s.io/pause:3.10` |
| GCR | `gcr.io/project/image:tag` | `<本站域名>/gcr.io/project/image:tag` |
| Microsoft MCR | `mcr.microsoft.com/dotnet/runtime:8.0` | `<本站域名>/mcr.microsoft.com/dotnet/runtime:8.0` |
| Elastic | `docker.elastic.co/elasticsearch/elasticsearch:8.13.4` | `<本站域名>/docker.elastic.co/elasticsearch/elasticsearch:8.13.4` |
| GitLab | `registry.gitlab.com/group/project/image:tag` | `<本站域名>/registry.gitlab.com/group/project/image:tag` |
| NVIDIA NVCR | `nvcr.io/nvidia/cuda:tag` | `<本站域名>/nvcr.io/nvidia/cuda:tag` |
| Oracle | `container-registry.oracle.com/os/oraclelinux:tag` | `<本站域名>/container-registry.oracle.com/os/oraclelinux:tag` |

`k8s.gcr.io` 仅作为旧地址兼容输入，实际使用 `registry.k8s.io` 上游。

当前适配器面向公开镜像的 `GET`/`HEAD` 拉取。私有镜像、需要点击接受许可的镜像、推送、
删除以及签名写入不属于公开代理承诺。

GitLab 私有项目、NGC 组织/团队镜像，以及要求登录或先接受条款的 Oracle 镜像不会借用
MirrorProxy 服务端凭据；上游的 `401`/`403` 会作为权限边界保留。列入支持矩阵只表示能够
匿名拉取该 Registry 中的公开项目。

## 排障时先看

- `/api/sources` 的 `container_registries` 是当前支持列表，不要凭页面名称猜路径。
- 401 可能是正常 Bearer 认证挑战；403、许可确认、私有镜像不由公共代理绕过。
- `/v2/` 正常只说明入口可达；真实拉取要检查 manifest 和 blob。
- 仍直连官方源时，检查实际使用的 Docker/containerd/K3s 配置，它们不是同一个文件。
- NAS 界面搜索和命令行拉取可能走不同链路，先以真实 `pull` 和日志定位。

[English](Container-Registries-and-Runtimes)
