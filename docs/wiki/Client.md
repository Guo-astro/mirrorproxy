# Standalone Client

> **This configures your computer; it does not deploy a server.** Have an existing server HTTPS root URL such as `https://mirror.example.com`, without `/npm` or other repository paths.

`mirrorproxy` is an independent binary: it neither connects to nor manages the
server database. It uses the shared catalog to write local source configuration
and preserves previous content for exact reset. Stable releases include Linux
x86_64/arm64, macOS Intel/Apple Silicon, and Windows x64 assets with checksums.

## Install

Linux/macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/inbjo/MirrorProxy/main/scripts/install.sh | sh
mirrorproxy --version
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/inbjo/MirrorProxy/main/scripts/install.ps1 | iex
mirrorproxy --version
```

The scripts verify the Release checksum. Set `MIRRORPROXY_VERSION` or
`MIRRORPROXY_INSTALL_DIR` to pin a version or change the destination.

## Common commands

```bash
# `sources` is optional, similar to chsrc.
mirrorproxy list
mirrorproxy list --category lang
mirrorproxy mirrors
mirrorproxy get npm --base-url https://mirror.example.com
mirrorproxy set npm --base-url https://mirror.example.com --dry-run
mirrorproxy set npm --base-url https://mirror.example.com
mirrorproxy reset npm
```

`set` defaults to user scope and refuses to overwrite a non-empty configuration;
use `--force` only after review. `--dry-run` previews changes. The `mirrorproxy`
provider requires `--base-url` in `http(s)://host[:port]` form, without a package
repository path.

Hugging Face is guidance-only: `mirrorproxy get hf --base-url
https://mirror.example.com` shows a platform-specific HTTP download command.
It does not write shell profiles or tokens. See [Hugging Face downloads](Proxy-Adapters#hugging-face-public-http-downloads)
for the required `HF_HUB_DISABLE_XET=1` setting and supported scope.

## Advanced: scope and safety

For the unified 28-distribution native migration, version matrix and Linux
verification, see [Linux system sources](System-Sources). Run `mirrorproxy systems`
to inspect the supported profiles. Existing target-specific commands remain available.

User scope covers npm, pip, Cargo, Go, Composer, and other supported targets.
System scope is directly supported on Linux and changes APT, DNF, pacman, apk,
zypper, xbps, Portage, and related system configurations; it needs normal
administrator rights and APT-like targets may require `--distribution`. Check
`get` and `--dry-run` before writing. The client never uploads credentials or
original configuration.

## Debian and Ubuntu system sources

APT detects `ID` and `VERSION_CODENAME` from `/etc/os-release` when
`--distribution` is omitted. Explicit `--distribution debian/bookworm` or
`ubuntu/noble` still works for the existing generated-file mode. Derivatives
require an explicit distribution; they are not guessed from their parent OS.

To migrate existing official sources instead of adding a separate repository:

```bash
sudo mirrorproxy set apt --scope system --base-url https://mirror.example.com --replace-existing --dry-run
sudo mirrorproxy set apt --scope system --base-url https://mirror.example.com --replace-existing
sudo apt-get update
apt-cache policy hello
sudo mirrorproxy reset apt --scope system --dry-run
sudo mirrorproxy reset apt --scope system
```

Replacement is opt-in and currently supports the MirrorProxy provider on Debian
and Ubuntu. It rewrites recognized official base URIs in `sources.list` and
`sources.list.d/*.list` / `*.sources`, including DEB822 URI continuations.
It preserves suites (including updates/backports), components, architectures,
`Signed-By`, comments, disabled entries, and third-party repositories. It never
changes release suites or installs/upgrades packages; `--distribution` cannot be
combined with `--replace-existing`. Custom mirror hosts, Debian security,
Ubuntu security/ports and old-release URIs remain unchanged because the default
server adapters do not cover those separate repositories. Their original
signature checks and update channels remain active. This is not a guarantee
that every APT request goes through MirrorProxy.

`--dry-run` validates and previews actual file changes without writing files or
rollback state. Existing non-empty generated configurations still need `--force`
for preview and apply. Sensitive configuration lines are redacted in previews.
Replacement validates all files before writing, saves a multi-file rollback at
`/var/lib/mirrorproxy/sources/apt.json`, and preserves file permissions on Unix.
Writes are atomic per file, not across the entire set. If interrupted, run
`reset apt --scope system` to restore the saved files. Reset validates every
path and file before restoring anything; later user edits require `--force`.
Keep this client version available until reset: older clients cannot read the
new replacement rollback format. Existing single-file rollback records remain
compatible. Symlinked files and directories are refused.

Verify `apt-cache policy <package>` and the actual download URI, in addition to
`apt-get update`: an unchanged security or third-party repository may still win
package selection. After reset, refresh APT metadata again. For isolated
testing, `--config-root <directory>` reads both OS identification and APT files
under that root and stores rollback state there.

[简体中文](Client-zh-CN)
