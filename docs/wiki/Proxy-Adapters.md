# Supported sources

Open your instance's home page, choose a source, and copy its generated command. See [Client](Client) for local configuration and [Container images](Container-Registries-and-Runtimes) for image pulls.

> **Server proxy support does not imply automatic client configuration for every target.** MirrorProxy is an on-demand read-only proxy/cache, not an unrestricted forward proxy or a private artifact upload service.

`/api/sources` lists the active instance's capabilities. Advanced settings controls adapters and upstream groups; comma-separated endpoints follow the configured selection policy.

## Covered categories

| Category | Representative targets |
| --- | --- |
| Code and releases | GitHub, GitHub Raw, Git smart HTTP clone. |
| AI models and datasets | Hugging Face public HTTP downloads; no server-side cache. |
| OCI | Docker Hub, GHCR, Quay, Kubernetes Registry, GCR, Microsoft MCR, Elastic Registry, GitLab, NVIDIA NVCR, Oracle, and Homebrew OCI. |
| Language ecosystems | Composer, npm, Go, Maven, RubyGems, NuGet, CPAN, CRAN, Hackage, Julia, LuaRocks, Clojars, CocoaPods, Pub, Anaconda, PyPI, Cargo. |
| Toolchains | Rustup, NVM, Homebrew, WinGet, TeX Live, ELPA, Nix, Guix, Flatpak. |
| Operating systems | Debian, Ubuntu, Fedora, Arch, Alpine, openSUSE, Void, Gentoo, FreeBSD, plus OpenWrt, Termux, MSYS2, ROS, and configured extra static directories. |

Proxy coverage does not mean that every native client has an automatic local
configuration writer. The server forwards the protocol; the standalone client
only writes targets defined as safely configurable on that platform. For example,
FreeBSD pkg is a proxy catalog target, not a claim of full system-scope automatic
configuration in the client.

## Hugging Face public HTTP downloads

Enable `huggingface` in Advanced settings. Existing installations with a persisted
`enabled_proxies` list must explicitly add it. The default `upstreams.huggingface`
is `https://huggingface.co`; trusted alternative Hub endpoints may include a base
path and use the usual upstream group selection.

Install the official `huggingface_hub` client, then use a POSIX shell:

```sh
export HF_ENDPOINT='https://mirror.example/huggingface'
export HF_HUB_DISABLE_XET=1
hf download gpt2 config.json
```

PowerShell:

```powershell
$env:HF_ENDPOINT='https://mirror.example/huggingface'
$env:HF_HUB_DISABLE_XET='1'
hf download gpt2 config.json
```

Set both variables **before importing Python libraries**. This endpoint supports
`hf_hub_download`, `snapshot_download`, and Hub-based downloads in
Transformers/Diffusers. Repository information, tree pagination, model files,
and dataset repository files are supported; external URLs referenced by a
dataset are outside this adapter's scope. Run
`mirrorproxy get hf --base-url https://mirror.example` for platform-specific
guidance. The CLI does not persist HF environment settings or tokens.

Only GET/HEAD are accepted. Commit/hash/size metadata, conditional requests and
Range/If-Range responses are preserved. The server follows CDN redirects so
clients do not need direct HF CDN access. Native Xet, Git/LFS clone,
private/gated repositories, uploads, and inference APIs are unsupported. Neither
client credentials nor configured upstream credentials are sent to HF. An
existing HF login does not grant private access through this route.

All HF requests bypass MirrorProxy's disk cache, including small files and
metadata; the SDK's local cache still works. Large responses stream without
whole-file buffering. `timeout.request_secs` bounds connection and read idle
time for this adapter, rather than total download duration. Existing source
access policies, rate limits, quota and usage accounting also apply to HF;
no database migration is needed.

The server needs direct Hub/CDN access. Destination validation and DNS pinning
remain enabled: with a global outbound proxy enabled, redirects to DNS-based
destinations fail closed under the existing security policy. The explicit HF
download-host allowlist may need updating when HF changes its CDN infrastructure;
unknown hosts are rejected.

Verify a local build against public upstreams in an isolated SDK environment:

```sh
uv run --no-project --with huggingface-hub==2.2.0 scripts/smoke-huggingface.py target/debug/mirrorproxy-server
```

The smoke starts a temporary server/database and checks a real model's SHA-256,
SDK snapshot/dataset downloads, HEAD metadata, Range/resume, pagination, and
absence of server cache writes. SDK HTTP hooks reject requests bypassing the
local proxy. It requires upstream network access.
For a test environment with proxy-only egress, prefix that command with
`MIRRORPROXY_HF_TEST_RELAY=1`. This uses a temporary trusted upstream relay with
normal TLS verification and the environment's HTTP proxy. Results explicitly
report this transport; it validates the alternative-upstream path rather than
direct HF egress. Production redirect and DNS checks remain enabled.

To roll back, run
`unset HF_ENDPOINT HF_HUB_DISABLE_XET` (PowerShell:
`Remove-Item Env:HF_ENDPOINT, Env:HF_HUB_DISABLE_XET -ErrorAction SilentlyContinue`),
and disable the adapter in Advanced settings to stop serving it.

## Advanced: upstreams and boundaries

- PyPI HTML indexes support `../../packages/...` download links, preserving query
  strings and hash fragments. Set `upstreams.pypi_files` to the file host/base
  directory before `packages/`; both `/pypi/packages/...` and the existing
  `/pypi/files/packages/...` routes retain that upstream path segment.
- An outbound proxy applies to mirror-upstream HTTP requests, not ACME, DNS APIs,
  or OAuth control-plane calls.
- Add enterprise CA PEM files with `upstream_tls.ca_certificates`.
  `insecure_skip_verify=true` disables all mirror-upstream TLS verification and
  is diagnostic-only.
- Git smart HTTP permits only read-only clone upload-pack POSTs; receive-pack
  writes remain rejected.
- Configure private upstream credentials and Authorization forwarding per target
  and with least privilege.
- Upstream redirects are limited to 10 hops. New destinations must resolve only
  to public addresses, and the checked addresses are pinned for the request.
  A private IP configured as an upstream may redirect within its own origin.
  Redirects through an outbound proxy are limited to the same configured IP origin
  because the outbound proxy can resolve other targets itself. If an upstream
  fails after upgrading, configure its final trusted URL directly; do not
  disable destination checks. Cross-origin redirects drop credentials.

[简体中文](Proxy-Adapters-zh-CN)
