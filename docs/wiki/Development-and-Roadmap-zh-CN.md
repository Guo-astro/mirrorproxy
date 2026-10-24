# 开发与路线图

本页面向贡献者。只想运行服务，请看[部署](Deployment-zh-CN)，无需安装开发工具链。

## 从源码构建

需要 Rust stable、Node.js 24 和 npm。服务端内嵌前端，因此先构建 Web，再编译 Rust：

```bash
cd web
npm ci
npm run build
cd ..
bash scripts/fetch-geoip.sh
cargo build --workspace --locked
```

发布式 Linux 静态构建使用 `./build.sh`，还需要目标工具链、`musl-tools` 与联网获取固定校验和的 GeoIP 数据。
生产环境仍应按部署指南配置 HTTPS。

## 本地检查

浏览器测试首次运行前，在 `web` 下执行 `npx playwright install chromium`。

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
(cd web && npm test && npm run build && npm run test:e2e)
cargo build --locked -p mirrorproxy-server
./scripts/smoke-admin-api.sh target/debug/mirrorproxy-server
```

真实客户端协议验证：

```bash
bash scripts/smoke-clients.sh
```

它使用隔离的临时客户端目录，需要相应生态工具与公网访问，不修改开发者真实配置。
某个生态通过，不代表所有 OS 仓库和容器运行时都已验证。
依赖检查使用 `cargo audit --ignore RUSTSEC-2023-0071 --ignore RUSTSEC-2026-0387`（仅限[已复核的 OIDC 公钥验签例外](https://github.com/inbjo/MirrorProxy/blob/main/SECURITY.md#scoped-dependency-advisory-exception--限定的依赖告警例外)）和 `web` 下的 `npm audit`。不带例外参数的 Rust 审计仍会报告这些未修复依赖，不能将其描述为已修复。引入私钥操作或修改 OIDC 依赖链前，必须重新评估。

## 进阶：仓库结构与交付

| 目录 | 内容 |
| --- | --- |
| `crates/server` | 服务、协议适配器、后台 API |
| `crates/client` | 本机改源客户端 |
| `crates/catalog` | 共用软件源目录和能力 |
| `web` | React 管理界面与浏览器测试 |

新增目标需同步目录、路由、配置、界面、客户端能力、文档和真实协议验证，不能只新增一个卡片。
发布流程见仓库 `AGENTS.md`、`.github/workflows` 和[发布维护者配置](Release-Maintenance-zh-CN)。

后续功能与缺陷通过 [Issues](https://github.com/inbjo/MirrorProxy/issues)、Milestones 和
[发布说明](https://github.com/inbjo/MirrorProxy/releases) 跟踪；计划中的功能要明确标注，不能当作当前版本能力。

[English](Development-and-Roadmap)
