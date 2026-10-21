#!/usr/bin/env bash
set -Eeuo pipefail

# Runs destructive package-manager checks only inside disposable OS containers.
# Migrate original distribution configuration and verify package payloads through
# the selected deployment. Initial bootstrap installs may use official upstreams.

base="${MIRRORPROXY_OS_SMOKE_BASE:-https://sina.dev}"
base="${base%/}"
registry="${MIRRORPROXY_OS_SMOKE_REGISTRY:-${base#*://}}"
targets="${MIRRORPROXY_OS_SMOKE_TARGETS:-debian ubuntu fedora archlinux alpine opensuse void gentoo}"
candidate="${MIRRORPROXY_OS_SMOKE_CLIENT_BINARY:-}"

command -v docker >/dev/null 2>&1 || { echo "docker is required" >&2; exit 2; }
[[ "${base}" =~ ^https?://[^/]+$ ]] || {
  echo "MIRRORPROXY_OS_SMOKE_BASE must be an origin URL without a path" >&2
  exit 2
}
if [[ -n "${candidate}" ]]; then
  candidate="$(realpath "${candidate}")"
  [[ -x "${candidate}" ]] || { echo "candidate client is not executable: ${candidate}" >&2; exit 2; }
fi

# Expanded intentionally by the shell running inside each container.
# shellcheck disable=SC2016
install_client='if [ -x /opt/mirrorproxy-candidate ]; then
  install -m 0755 /opt/mirrorproxy-candidate /usr/local/bin/mirrorproxy
else
  curl -fsSL "${MIRROR_BASE}/https://raw.githubusercontent.com/inbjo/MirrorProxy/main/scripts/install.sh" | sh -s -- --mirror "${MIRROR_BASE}"
fi
mirrorproxy --version
mirrorproxy systems'

run_case() {
  local target=$1
  local image=$2
  local body=$3
  local -a args=(run --rm -e "MIRROR_BASE=${base}")
  if [[ -n "${candidate}" ]]; then
    args+=(-v "${candidate}:/opt/mirrorproxy-candidate:ro")
  fi
  printf '\n==> %s (%s)\n' "${target}" "${registry}/${image}"
  docker "${args[@]}" "${registry}/${image}" sh -euxc "${body}"
  printf 'PASS %s\n' "${target}"
}

for target in ${targets}; do
  case "${target}" in
    debian)
      run_case debian "${MIRRORPROXY_OS_SMOKE_DEBIAN_IMAGE:-debian:bookworm-slim}" "apt-get update
apt-get install -y --no-install-recommends ca-certificates curl
${install_client}
find /etc/apt -type f -exec sha256sum {} \\; > /tmp/apt.before
mirrorproxy set system --base-url \"\${MIRROR_BASE}\" --scope system --replace-existing --dry-run
sha256sum -c /tmp/apt.before
test ! -e /var/lib/mirrorproxy/sources/system.json
mirrorproxy set system --base-url \"\${MIRROR_BASE}\" --scope system --replace-existing
rm -rf /var/lib/apt/lists/*
apt-get update
cd /tmp
apt-get --print-uris download hello | grep -F \"\${MIRROR_BASE}/os/debian/\"
apt-get download hello
test -s hello_*.deb
mirrorproxy reset system --scope system
sha256sum -c /tmp/apt.before
test ! -e /var/lib/mirrorproxy/sources/system.json"
      ;;
    ubuntu)
      run_case ubuntu "${MIRRORPROXY_OS_SMOKE_UBUNTU_IMAGE:-ubuntu:24.04}" "apt-get update
DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends ca-certificates curl
${install_client}
find /etc/apt -type f -exec sha256sum {} \\; > /tmp/apt.before
mirrorproxy set system --base-url \"\${MIRROR_BASE}\" --scope system --replace-existing --dry-run
sha256sum -c /tmp/apt.before
test ! -e /var/lib/mirrorproxy/sources/system.json
mirrorproxy set system --base-url \"\${MIRROR_BASE}\" --scope system --replace-existing
rm -rf /var/lib/apt/lists/*
apt-get update
cd /tmp
apt-get --print-uris download hello | grep -F \"\${MIRROR_BASE}/os/ubuntu/\"
apt-get download hello
test -s hello_*.deb
mirrorproxy reset system --scope system
sha256sum -c /tmp/apt.before
test ! -e /var/lib/mirrorproxy/sources/system.json"
      ;;
    fedora|rocky|alma|oraclelinux)
      case "${target}" in
        fedora) rpm_image="${MIRRORPROXY_OS_SMOKE_FEDORA_IMAGE:-fedora:44}" ;;
        rocky) rpm_image="${MIRRORPROXY_OS_SMOKE_ROCKY_IMAGE:-rockylinux:9}" ;;
        alma) rpm_image="${MIRRORPROXY_OS_SMOKE_ALMA_IMAGE:-almalinux:9}" ;;
        oraclelinux) rpm_image="${MIRRORPROXY_OS_SMOKE_ORACLELINUX_IMAGE:-oraclelinux:9}" ;;
      esac
      run_case "${target}" "${rpm_image}" "dnf install -y curl ca-certificates
if ! dnf download --help >/dev/null 2>&1; then dnf install -y dnf-plugins-core; fi
${install_client}
find /etc/yum.repos.d -type f -exec sha256sum {} \\; > /tmp/repos.before
mirrorproxy set system --base-url \"\${MIRROR_BASE}\" --scope system --replace-existing
dnf --refresh makecache
dnf download --url setup | grep -F \"\${MIRROR_BASE}/os/\"
dnf download --destdir=/tmp setup
test -s /tmp/setup-*.rpm
rpm --import /etc/pki/rpm-gpg/RPM-GPG-KEY-*
rpmkeys --checksig /tmp/setup-*.rpm
mirrorproxy reset system --scope system
sha256sum -c /tmp/repos.before"
      ;;
    archlinux)
      run_case archlinux "${MIRRORPROXY_OS_SMOKE_ARCHLINUX_IMAGE:-archlinux:latest}" "${install_client}
sha256sum /etc/pacman.d/mirrorlist >/tmp/mirrorlist.before
mirrorproxy set system --base-url \"\${MIRROR_BASE}\" --scope system --replace-existing
grep -F \"\${MIRROR_BASE}/os/archlinux/\" /etc/pacman.d/mirrorlist
pacman -Syyw --noconfirm --cachedir /tmp filesystem
test -s /tmp/filesystem-*.pkg.tar.zst
mirrorproxy reset system --scope system
sha256sum -c /tmp/mirrorlist.before"
      ;;
    alpine)
      run_case alpine "${MIRRORPROXY_OS_SMOKE_ALPINE_IMAGE:-alpine:3.21}" "apk add --no-cache ca-certificates curl
${install_client}
sha256sum /etc/apk/repositories > /tmp/repositories.before
mirrorproxy set system --base-url \"\${MIRROR_BASE}\" --scope system --replace-existing
apk update
apk fetch --output /tmp busybox
test -s /tmp/busybox-*.apk
apk verify /tmp/busybox-*.apk
mirrorproxy reset system --scope system
sha256sum -c /tmp/repositories.before"
      ;;
    opensuse)
      run_case opensuse "${MIRRORPROXY_OS_SMOKE_OPENSUSE_IMAGE:-opensuse/leap:15.6}" "zypper --non-interactive install --no-recommends tar gzip
${install_client}
find /etc/zypp/repos.d -type f -exec sha256sum {} \\; > /tmp/repos.before
mirrorproxy set system --base-url \"\${MIRROR_BASE}\" --scope system --replace-existing
zypper --non-interactive refresh
zypper --non-interactive install --download-only --no-recommends tree
find /var/cache/zypp/packages -type f -name 'tree-*.rpm' -print -quit | grep .
mirrorproxy reset system --scope system
sha256sum -c /tmp/repos.before"
      ;;
    void)
      run_case void "${MIRRORPROXY_OS_SMOKE_VOID_IMAGE:-voidlinux/voidlinux:latest}" "xbps-install -Suy xbps
xbps-install -y ca-certificates curl
${install_client}
find /usr/share/xbps.d -type f -exec sha256sum {} \\; > /tmp/vendor-repos.before
mirrorproxy set system --base-url \"\${MIRROR_BASE}\" --scope system --replace-existing
xbps-install -S
xbps-install -y tree
command -v tree
mirrorproxy reset system --scope system
sha256sum -c /tmp/vendor-repos.before"
      ;;
    gentoo)
      run_case gentoo "${MIRRORPROXY_OS_SMOKE_GENTOO_IMAGE:-gentoo/stage3:latest}" "${install_client}
emerge-webrsync
sha256sum /etc/portage/make.conf > /tmp/make-conf.before
mirrorproxy set system --base-url \"\${MIRROR_BASE}\" --scope system --replace-existing
emerge --fetchonly --quiet-build app-misc/hello
test -s /var/cache/distfiles/hello-*.tar.gz
mirrorproxy reset system --scope system
sha256sum -c /tmp/make-conf.before"
      ;;
    *)
      echo "unknown OS smoke target: ${target}" >&2
      exit 2
      ;;
  esac
done

printf '\nOS native-client smoke passed: %s\n' "${targets}"
