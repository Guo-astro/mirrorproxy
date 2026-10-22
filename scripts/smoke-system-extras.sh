#!/usr/bin/env bash
set -Eeuo pipefail

# All package installation and trust bootstrap occurs in disposable containers.
base="${MIRRORPROXY_OS_SMOKE_BASE:-https://sina.dev}"
registry="${MIRRORPROXY_OS_SMOKE_REGISTRY:-sina.dev}"
candidate="${MIRRORPROXY_OS_SMOKE_CLIENT_BINARY:?set a static candidate client binary}"
candidate="$(realpath "${candidate}")"
targets="${MIRRORPROXY_EXTRA_SMOKE_TARGETS:-epel epel-next epel-archive alma-vault docker-ce docker-ce-rpm gentoo-sync}"
[[ -x "${candidate}" && "${base}" =~ ^https?://[^/]+/?$ ]] || exit 2
base="${base%/}"
container_dir="$(mktemp -d /tmp/mirrorproxy-extra-container.XXXXXX)"
cleanup_container() {
  local container_id=""
  if [[ -s "${container_dir}/container.id" ]]; then
    read -r container_id <"${container_dir}/container.id" || true
    if [[ "${container_id}" =~ ^[a-f0-9]{64}$ ]]; then
      docker rm -f "${container_id}" >/dev/null 2>&1 || true
    fi
    rm -f "${container_dir}/container.id"
  fi
  rmdir "${container_dir}" 2>/dev/null || true
}
trap cleanup_container EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

run_case() {
  curl --noproxy '*' --retry 3 --retry-connrefused --connect-timeout 5 --max-time 20 -fsS "${base}/healthz" >/dev/null
  docker run --rm --init --cidfile "${container_dir}/container.id" -e "MIRROR_BASE=${base}" \
    -v "${candidate}:/usr/local/bin/mirrorproxy:ro" \
    "${registry}/$1" sh -euxc "$2"
  rm -f "${container_dir}/container.id"
}

for target in ${targets}; do
  case "${target}" in
    epel)
      # The epel-release RPM establishes Fedora's official key before migration.
      # shellcheck disable=SC2016
      run_case almalinux:9 '
dnf install -y epel-release dnf-plugins-core
find /etc/yum.repos.d -type f -exec sha256sum {} \; >/tmp/repos.before
mirrorproxy set epel --scope system --replace-existing --base-url "${MIRROR_BASE}"
dnf --disablerepo="*" --enablerepo=epel --refresh makecache
dnf --disablerepo="*" --enablerepo=epel download --url htop | grep -F "${MIRROR_BASE}/os/epel/"
dnf --disablerepo="*" --enablerepo=epel download --destdir=/tmp htop
rpm --import /etc/pki/rpm-gpg/RPM-GPG-KEY-EPEL-9
rpmkeys --checksig /tmp/htop-*.rpm
mirrorproxy reset epel --scope system
sha256sum -c /tmp/repos.before'
      ;;
    epel-next)
      # shellcheck disable=SC2016
      run_case almalinux:9 '
dnf install -y epel-release dnf-plugins-core
dnf install -y https://dl.fedoraproject.org/pub/epel/epel-next-release-latest-9.noarch.rpm
sha256sum /etc/yum.repos.d/*.repo >/tmp/repos.before
mirrorproxy set epel --scope system --replace-existing --base-url "${MIRROR_BASE}"
dnf --disablerepo="*" --enablerepo=epel-next --refresh makecache
grep -F "${MIRROR_BASE}/os/epel-next/" /etc/yum.repos.d/epel-next.repo
mirrorproxy reset epel --scope system
sha256sum -c /tmp/repos.before'
      ;;
    alma-vault)
      # shellcheck disable=SC2016
      run_case almalinux:9 '
dnf install -y dnf-plugins-core
printf "%s\n" "[vault-test]" "name=AlmaLinux Vault 9.5" "baseurl=https://vault.almalinux.org/9.5/BaseOS/\$basearch/os/" "enabled=1" "gpgcheck=1" "gpgkey=file:///etc/pki/rpm-gpg/RPM-GPG-KEY-AlmaLinux-9" >/etc/yum.repos.d/vault-test.repo
sha256sum /etc/yum.repos.d/*.repo >/tmp/repos.before
mirrorproxy set system --scope system --replace-existing --base-url "${MIRROR_BASE}"
dnf --disablerepo="*" --enablerepo=vault-test --refresh makecache
dnf --disablerepo="*" --enablerepo=vault-test download --url setup | grep -F "${MIRROR_BASE}/os/alma-vault/9.5/"
dnf --disablerepo="*" --enablerepo=vault-test download --destdir=/tmp setup
rpm --import /etc/pki/rpm-gpg/RPM-GPG-KEY-AlmaLinux-9
rpmkeys --checksig /tmp/setup-*.rpm
mirrorproxy reset system --scope system
sha256sum -c /tmp/repos.before'
      ;;
    epel-archive)
      # shellcheck disable=SC2016
      run_case almalinux:9 '
dnf install -y dnf-plugins-core
printf "%s\n" "[epel-archive-test]" "name=EPEL 7 Archive" "baseurl=https://archives.fedoraproject.org/pub/archive/epel/7/\$basearch/" "enabled=1" "gpgcheck=1" "gpgkey=https://dl.fedoraproject.org/pub/epel/RPM-GPG-KEY-EPEL-7" >/etc/yum.repos.d/epel-archive-test.repo
sha256sum /etc/yum.repos.d/*.repo >/tmp/repos.before
mirrorproxy set epel --scope system --replace-existing --base-url "${MIRROR_BASE}"
dnf --disablerepo="*" --enablerepo=epel-archive-test --refresh makecache
dnf --disablerepo="*" --enablerepo=epel-archive-test download --url htop | grep -F "${MIRROR_BASE}/os/epel-archive/7/"
dnf --disablerepo="*" --enablerepo=epel-archive-test download --destdir=/tmp htop
rpm --import https://dl.fedoraproject.org/pub/epel/RPM-GPG-KEY-EPEL-7
rpmkeys --checksig /tmp/htop-*.rpm
mirrorproxy reset epel --scope system
sha256sum -c /tmp/repos.before'
      ;;
    docker-ce)
      # Docker packages are downloaded and verified; no daemon is started.
      # shellcheck disable=SC2016
      run_case debian:bookworm-slim '
mirrorproxy set system --scope system --replace-existing --base-url "${MIRROR_BASE}"
apt-get update -o APT::Update::Error-Mode=any
apt-get install -y --no-install-recommends ca-certificates curl gnupg
mirrorproxy reset system --scope system
install -d -m 0755 /etc/apt/keyrings
curl --retry 3 --retry-all-errors -fsSL "${MIRROR_BASE}/os/docker-ce/linux/debian/gpg" -o /etc/apt/keyrings/docker.asc
gpg --show-keys --with-colons /etc/apt/keyrings/docker.asc | grep -F "fpr:::::::::9DC858229FC7DD38854AE2D88D81803C0EBFCD88:"
printf "%s\n" "deb [signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian bookworm stable" >/etc/apt/sources.list.d/docker.list
sha256sum /etc/apt/sources.list.d/docker.list /etc/apt/keyrings/docker.asc >/tmp/docker.before
mirrorproxy set docker-ce --scope system --replace-existing --base-url "${MIRROR_BASE}"
apt-get update -o APT::Update::Error-Mode=any -o Dir::Etc::sourcelist=/etc/apt/sources.list.d/docker.list -o Dir::Etc::sourceparts=-
cd /tmp
apt-get --print-uris download docker-ce-cli | grep -F "${MIRROR_BASE}/os/docker-ce/"
apt-get download docker-ce-cli
test -s docker-ce-cli_*.deb
mirrorproxy reset docker-ce --scope system
sha256sum -c /tmp/docker.before
rm /etc/apt/sources.list.d/docker.list
mirrorproxy set docker-ce --scope system --replace-existing --base-url "${MIRROR_BASE}"
grep -F "Signed-By: /etc/apt/keyrings/docker.asc" /etc/apt/sources.list.d/mirrorproxy-docker-ce.sources
apt-get update -o APT::Update::Error-Mode=any -o Dir::Etc::sourcelist=/etc/apt/sources.list.d/mirrorproxy-docker-ce.sources -o Dir::Etc::sourceparts=-
mirrorproxy reset docker-ce --scope system
test ! -e /etc/apt/sources.list.d/mirrorproxy-docker-ce.sources'
      ;;
    docker-ce-rpm)
      # shellcheck disable=SC2016
      run_case fedora:44 '
sha256sum /etc/yum.repos.d/*.repo >/tmp/repos.before
mirrorproxy set docker-ce --scope system --replace-existing --base-url "${MIRROR_BASE}"
dnf --disablerepo="*" --enablerepo=docker-ce-stable --refresh makecache
dnf --disablerepo="*" --enablerepo=docker-ce-stable download --url docker-ce-cli | grep -F "${MIRROR_BASE}/os/docker-ce/"
dnf --disablerepo="*" --enablerepo=docker-ce-stable download --destdir=/tmp docker-ce-cli
curl --retry 3 --retry-all-errors -fsSL "${MIRROR_BASE}/os/docker-ce/linux/fedora/gpg" -o /tmp/docker-rpm-gpg
echo "e6c650e0700b1bf4868b693b30761b926844befc8a0acb7ac0dd9b1faf1b7423  /tmp/docker-rpm-gpg" | sha256sum -c -
rpm --import /tmp/docker-rpm-gpg
rpmkeys --checksig /tmp/docker-ce-cli-*.rpm
mirrorproxy reset docker-ce --scope system
sha256sum -c /tmp/repos.before
test ! -e /etc/yum.repos.d/mirrorproxy-docker-ce.repo'
      ;;
    gentoo-sync)
      # emerge uses webrsync and its configured official OpenPGP verifier.
      # shellcheck disable=SC2016
      run_case gentoo/stage3:latest '
find /etc/portage -type f -exec sha256sum {} \; >/tmp/portage.before
mirrorproxy set gentoo-sync --scope system --replace-existing --base-url "${MIRROR_BASE}"
emerge --sync --quiet
test -s /var/db/repos/gentoo/metadata/timestamp.chk
mirrorproxy reset gentoo-sync --scope system
sha256sum -c /tmp/portage.before
test ! -e /etc/portage/repos.conf/gentoo.conf'
      ;;
    *) echo "unknown extra smoke target: ${target}" >&2; exit 2 ;;
  esac
  printf 'PASS %s\n' "${target}"
done
