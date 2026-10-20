# Supported sources

Open your instance's home page, choose a source, and copy its generated command. See [Client](Client) for local configuration and [Container images](Container-Registries-and-Runtimes) for image pulls.

> **Server proxy support does not imply automatic client configuration for every target.** MirrorProxy is an on-demand read-only proxy/cache, not an unrestricted forward proxy or a private artifact upload service.

`/api/sources` lists the active instance's capabilities. Advanced settings controls adapters and upstream groups; comma-separated endpoints follow the configured selection policy.

## Covered categories

| Category | Representative targets |
| --- | --- |
| Code and releases | GitHub, GitHub Raw, Git smart HTTP clone. |
| OCI | Docker Hub, GHCR, Quay, Kubernetes Registry, GCR, Microsoft MCR, Elastic Registry, GitLab, NVIDIA NVCR, Oracle, and Homebrew OCI. |
| Language ecosystems | Composer, npm, Go, Maven, RubyGems, NuGet, CPAN, CRAN, Hackage, Julia, LuaRocks, Clojars, CocoaPods, Pub, Anaconda, PyPI, Cargo. |
| Toolchains | Rustup, NVM, Homebrew, WinGet, TeX Live, ELPA, Nix, Guix, Flatpak. |
| Operating systems | Debian, Ubuntu, Fedora, Arch, Alpine, openSUSE, Void, Gentoo, FreeBSD, plus OpenWrt, Termux, MSYS2, ROS, and configured extra static directories. |

Proxy coverage does not mean that every native client has an automatic local
configuration writer. The server forwards the protocol; the standalone client
only writes targets defined as safely configurable on that platform. For example,
FreeBSD pkg is a proxy catalog target, not a claim of full system-scope automatic
configuration in the client.

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
