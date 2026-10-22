# Linux source acceptance — 2026-10-22

Scope: the local `dev` worktree based on `c69ad6f`, including uncommitted Linux
source changes. No release, push or production replacement is included.
The comparison reference is LinuxMirrors `76fbf1738c22108cdbcc73973612bdb07ad6789b`.

## Implemented changes

1. EPEL, EPEL Next, EPEL Archive and AlmaLinux Vault repository roots, constrained
   native migration, preserved trust settings and backward-compatible defaults.
2. Explicit Ubuntu flavour IDs gated by their declared base, plus vendor Mint
   release identification when the container retains its Ubuntu parent ID.
3. Container cases for every profile, independent matrix results, and explicit
   `BLOCKED` results when authentic images/customer repositories are missing.
4. Mint 19–22 and LMDE 6–7 claims; pre-existing rollback records remain readable.
5. Recognition of 96 LinuxMirrors mirror roots, including path-prefixed overseas
   mirrors and cloud-internal mirrors. Domains and repository path boundaries
   must match; unknown repositories are retained. Recognition does not select
   the server's upstream or assert that every mirror carries every repository.
6. Independent `epel`, `docker-ce` and `gentoo-sync` operations with previews and
   protected rollback. Docker CE supports existing APT/RPM source migration and
   initial signed source creation; APT requires an operator-provisioned official
   Docker key. Gentoo uses signed HTTP webrsync snapshots with its existing key.

Additional failures found during real tests were repaired: CentOS Stream
metalink-only source files, Void vendor-source aliases, the Fedora default
upstream, and healthy package streams being cut off by a 60-second total timeout.
Connection/read idle timeouts and redirect destination validation remain active.

## Native package-manager coverage

These are representative x86_64 Linux environments, not proof of every advertised
release, architecture, optional repository or complete OS installation. The first
11 cases were executed in the preceding Docker validation; the additional cases
below were executed during the LinuxMirrors parity work. Relevant failures were
rerun after fixes. The image prefix was `sina.dev/`; candidate servers ran locally
with temporary SQLite databases and static musl client binaries.

| Profile | Environment | Result and evidence |
| --- | --- | --- |
| Debian | `debian:bookworm-slim` | PASS: signed indexes, hello DEB, URI, dry-run and exact rollback |
| Ubuntu | `ubuntu:24.04` | PASS: base/security indexes, hello DEB and rollback |
| Fedora | `fedora:44` | PASS: setup RPM, native signature check and rollback |
| Rocky | `rockylinux:9` | PASS: large metadata, setup RPM signature and rollback |
| AlmaLinux | `almalinux:9` | PASS: setup RPM signature and rollback |
| Oracle Linux | `oraclelinux:9` | PASS: native Oracle BaseOS/AppStream, setup RPM signature and rollback |
| Arch Linux | `archlinux:latest` | PASS: pacman indexes, filesystem package integrity and rollback |
| Alpine | `alpine:3.21` | PASS: apk indexes, busybox package verification and rollback |
| openSUSE | `opensuse/leap:15.6` | PASS: tree RPM download and rollback; the base image reports expired upstream metadata/key warnings |
| Void | `voidlinux/voidlinux:latest` | PASS: XBPS self-update, signed tree install, removal of generated overrides |
| Gentoo distfiles | `gentoo/stage3:latest` | PASS: fixed hello distfile SHA-256 and make.conf rollback |
| Kali | `kalilinux/kali-rolling:latest` | PASS: native signed APT indexes, bash DEB and rollback |
| Deepin | `linuxdeepin/deepin:beige` | PASS: beige indexes, bash DEB and rollback |
| openKylin | `openkylin/openkylin:2.0` | PASS: native signed indexes, bash DEB and rollback |
| openEuler | `openeuler/openeuler:24.03-lts` | PASS: native DNF metadata, setup RPM signature and rollback |
| OpenCloudOS | `opencloudos/opencloudos:9.0` | PASS: native DNF metadata, setup RPM signature and rollback |
| Anolis | `openanolis/anolisos:8.8` | PASS: native DNF metadata, setup RPM signature and rollback |
| Manjaro | `manjarolinux/base:latest` | PASS: stable-branch indexes, native package integrity and rollback |
| Linux Mint | `linuxmintd/mint22-amd64:latest` plus official `mint-info-cinnamon` | PASS: explicit Mint profile, both Mint/Ubuntu roots, signed indexes, bash DEB and rollback; this is the vendor build container, not a booted desktop |
| CentOS | `centos:7` | PASS: Vault 7.9.2009, setup RPM signature and rollback, using a configured Huawei Cloud Vault upstream |
| CentOS Stream | `quay.io/centos/centos:stream9` | PASS: metalink migration, setup RPM signature and rollback, using a configured TUNA Stream upstream |
| Zorin | authentic image not supplied/selected | BLOCKED: runner exit 2 |
| Armbian | authentic image/hardware test host not supplied | BLOCKED: runner exit 2 |
| Proxmox VE | authentic installed-system image/test host not supplied | BLOCKED: runner exit 2 |
| Raspberry Pi OS | authentic matching-architecture image/test host not supplied | BLOCKED: runner exit 2 |
| RHEL | entitled image and compatible customer mirror not supplied | BLOCKED: runner exit 2 |
| EndeavourOS | authentic installed-system image not supplied/selected | BLOCKED: runner exit 2 |
| NixOS | authentic NixOS environment not supplied/selected | BLOCKED: runner exit 2; `nixos/nix` alone is not a NixOS system |

Both official CentOS endpoints returned a CloudFront response explicitly blocking
the test machine's country. The alternative upstream checks retained native GPG
verification and did not change production or global server defaults.

## Supplemental repository checks

| Check | Result |
| --- | --- |
| EPEL 9 | PASS: htop RPM obtained through candidate, digests/signatures OK, exact rollback |
| EPEL Next 9 | PASS: real Next metadata through `/os/epel-next/`, native source file restored |
| EPEL 7 Archive | PASS: archived htop RPM, official EPEL 7 key, digests/signatures OK, rollback |
| AlmaLinux Vault 9.5 | PASS: pinned Vault metadata and setup RPM signature, rollback |
| Docker CE RPM | PASS: generated Fedora repo, docker-ce-cli RPM, pinned official key and native signature verification, rollback removes generated file |
| Docker CE APT | PASS: signed Docker indexes and docker-ce-cli DEB through candidate, exact existing-source/key rollback, initial DEB822 creation and removal; Debian tool bootstrap used a configured TUNA upstream |
| Gentoo sync | PASS: actual `emerge --sync`, signed snapshot download from candidate, populated Portage tree and exact configuration rollback |

No Docker daemon is installed or started by these checks. Distribution release
keys are provisioned using their native vendor workflow in disposable containers;
the client preserves existing trust and does not execute repository configuration.

## Reproduction and remaining acceptance

Build the candidate client with locked resolution:

```bash
cargo build --locked --release -p mirrorproxy-cli --target x86_64-unknown-linux-musl
export MIRRORPROXY_OS_SMOKE_CLIENT_BINARY="$PWD/target/x86_64-unknown-linux-musl/release/mirrorproxy"
export MIRRORPROXY_OS_SMOKE_BASE=https://candidate.example.com
export MIRRORPROXY_OS_SMOKE_REGISTRY=sina.dev
bash scripts/smoke-os-matrix.sh
bash scripts/smoke-system-extras.sh
```

The candidate server must enable `os` and contain the new roots. The matrix
runner writes one log per profile and `results.tsv`; any failure or blocked
profile makes it exit nonzero. Supply the seven blocked authentic environments
through `MIRRORPROXY_OS_SMOKE_<UPPERCASE_PROFILE>_IMAGE`, or execute equivalent
commands on disposable native test hosts. RHEL additionally needs an authorized
customer mirror. Parent-distribution images with hand-edited IDs are not native
acceptance evidence. No VM provisioning tools are installed in this workspace.

Temporary execution logs for this session are under
`/tmp/mirrorproxy-linux-matrix/`; they are not release artifacts or tracked files.

The full Rust workspace gate passes 385 tests (7 catalog, 44 client unit,
10 CLI integration, 324 server). Strict Clippy, formatting, ShellCheck, Bash
syntax, Docker build-context checks and `git diff --check` passed. RustSec audit
passed with the repository's existing `RUSTSEC-2023-0071` RSA public-verification
exception. Frontend code/dependencies were not changed in this task. No new
release or remote CI/CodeQL run has been requested.

Real administration smoke and source-detail API smoke also passed, including
login/session/CSRF, encrypted SQLite checks and backup, source capabilities,
credential isolation, fallback/ranking and administrator authorization.

The final worktree Docker image `mirrorproxy-local-test:linux-sources` built and
ran successfully on `linux/amd64`. Local image ID:
`sha256:e825a6353a27a63556ab67ecb47dba98c1acd877a64e5ad1f122db6b8eaf1c71`.
The container was `healthy`, ran as UID/GID 10001, exposed the five supplemental
roots through `/api/sources`, and preserved the pinned IPv4/IPv6 GeoIP SHA-256
values. The public port was bound only to localhost. The test container and
temporary standalone services/databases were removed afterward; logs and local
test images are retained for inspection. Nothing was pushed or deployed.
