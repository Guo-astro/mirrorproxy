# Development and roadmap

For contributors. To run the service, see [Deployment](Deployment); development tools are unnecessary.

## Build from source

Use Rust stable, Node.js 24, and npm. The server embeds the frontend, so build Web before Rust:

```bash
cd web
npm ci
npm run build
cd ..
bash scripts/fetch-geoip.sh
cargo build --workspace --locked
```

`./build.sh` performs the release-style static Linux build. It also needs the target toolchain, `musl-tools`, and network access for checksum-pinned GeoIP data.
Production installations still need HTTPS as described in the deployment guide.

## Local checks

Before the first browser run, execute `npx playwright install chromium` from `web`.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
(cd web && npm test && npm run build && npm run test:e2e)
cargo build --locked -p mirrorproxy-server
./scripts/smoke-admin-api.sh target/debug/mirrorproxy-server
```

For real client protocols:

```bash
bash scripts/smoke-clients.sh
```

The smoke uses isolated temporary client directories, needs ecosystem tools and public network access, and does not edit your real configuration.
Passing one ecosystem does not verify every OS repository or container runtime.
Audit with `cargo audit --ignore RUSTSEC-2023-0071` (the project's documented RSA private-key-path exception only) and `npm audit` from `web`.

## Advanced: repository layout and delivery

| Directory | Purpose |
| --- | --- |
| `crates/server` | Server, protocol adapters, admin API |
| `crates/client` | Local client configuration |
| `crates/catalog` | Shared sources and capabilities |
| `web` | React console and browser tests |

New targets need aligned catalog entries, routes, configuration, UI, client capabilities, docs, and real protocol checks; adding a card alone is insufficient.
Release instructions live in repository `AGENTS.md`, `.github/workflows`, and [publisher setup](Release-Maintenance).

Track features and defects through [Issues](https://github.com/inbjo/MirrorProxy/issues), milestones, and
[release notes](https://github.com/inbjo/MirrorProxy/releases). Mark planned functionality explicitly rather than presenting it as current support.

[简体中文](Development-and-Roadmap-zh-CN)
