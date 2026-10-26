# Agent integration: Skill and MCP

MirrorProxy offers a portable Skill and a native MCP stdio server in
the `mirrorproxy` client. They discover an existing instance's source catalog,
enabled adapters, public access restrictions and quota policy. Optional controlled
download tools require separate user authorization. They do not provide an
unrestricted forward proxy or change client configuration.

## Install

### Recommended: complete setup from the Web console

Open **Set up your AI agents** on the public home page and copy the Linux/macOS
or Windows PowerShell command. It downloads and checksum-verifies the native
client, then starts `mirrorproxy setup --base-url <current-instance>`.
Signed-in users with an assigned proxy subdomain get that personal origin.
This flow requires a published stable client containing `setup`; it does not
make unpublished source changes available through an older stable release.

The homepage serves `/install.sh` and `/install.ps1` from the same server build,
so it does not depend on a possibly stale `main` branch script. Each download is
checksum-verified, and setup capability is checked before replacing an installed
client. A release without `setup` is rejected without changing the existing
binary. Publishing a new stable client remains a separate release step.

The Windows command runs the downloaded text as a script block with explicit
parameters; it does not run `Set-ExecutionPolicy` or change machine/user policy.
Enterprise application control and group policy restrictions still apply.

The local wizard asks for the installation scope first:

1. **Global, current user — strongly recommended (default)**: available across projects.
2. **Current project only**: Skill and MCP configuration stay in the selected project.

Then choose one or more of Codex, Claude Code and Cursor. Agent executables on
PATH and initialized directories are detected and offered as defaults. With no interactive terminal,
specify both `--agent` and `--scope`; missing selections never silently become a
global installation. Unix prompts read `/dev/tty`, so piping the bootstrap script
does not consume the wizard's input. Windows uses the attached console.

Existing users can start the wizard directly:

```sh
mirrorproxy setup --base-url https://mirror.example.com
# Explicit unattended setup; validates the live instance before writing:
mirrorproxy setup --base-url https://mirror.example.com --agent codex,cursor --scope global
# Preview paths without network access or writes:
mirrorproxy setup --base-url https://mirror.example.com --agent codex --scope project --dry-run
```

Unix complete setup installs the client into `~/.local/bin` unless overridden;
CLI-only installation keeps its existing default. Windows keeps its existing
user-local client directory and PATH handling. MCP configurations and the bound
Skill use the executable's absolute path, so Agent operation does not depend on
a shell restart or PATH changes. The wizard does not install Agent applications.

| Agent | Global Skill | Global MCP | Project MCP |
| --- | --- | --- | --- |
| Codex | `~/.agents/skills/mirrorproxy/SKILL.md` | `$CODEX_HOME/config.toml`, default `~/.codex/config.toml` | `.codex/config.toml` |
| Claude Code | `~/.claude/skills/mirrorproxy/SKILL.md` | `~/.claude.json` | `.mcp.json` |
| Cursor | `~/.cursor/skills/mirrorproxy/SKILL.md` | `~/.cursor/mcp.json` | `.cursor/mcp.json` |

Project Skills use the corresponding directory under the project; Codex uses
`.agents/skills`. `--project-dir` selects a different project in project scope.
See the official [Codex MCP](https://developers.openai.com/codex/mcp/),
[Codex Skills](https://developers.openai.com/codex/skills/),
[Claude MCP](https://code.claude.com/docs/en/mcp),
[Claude Skills](https://code.claude.com/docs/en/skills),
[Cursor MCP](https://prod.cursor.com/help/customization/mcp), and
[Cursor Skills](https://cursor.com/docs/skills) documentation.

The wizard preserves other MCP servers and settings, including Codex TOML
comments. Existing conflicting MirrorProxy entries require confirmation;
unattended replacement requires explicit `--replace-existing`. Symlinked paths
and invalid configuration files are refused. No administrator/session token is
embedded in the Skill or MCP configuration. Normal Agent trust and tool approval
policies remain in effect; restart the Agent after setup.

Before writing, the installed binary verifies its MCP handshake, tool list and
the instance's source catalog payload. This checks discovery, not artifact
download health or user quota allowance. Changes are backed up in a private
manifest under `~/.mirrorproxy/agent-setups/<id>/manifest.json`, even in project
scope, so backups of existing credentials stay out of the project. Unix backup
files use mode `0600`; Windows files inherit the user's home-directory ACLs.
On a failed write, identifiable changed files are restored automatically. The
wizard prints an explicit recovery ID if recovery is needed. Configuration
parse errors do not print source lines that could contain credentials.

To restore a setup, use the ID printed by the wizard:

```sh
mirrorproxy setup --scope global --rollback <id>
# Run in the same project, or add --project-dir:
mirrorproxy setup --scope project --rollback <id>
```

Rollback restores exact previous files and removes files newly created by setup;
it refuses files edited since setup. Protected backup manifests are retained.
The client binary is retained. Global means the current user, not all users on
the machine. Local Agent setup is not a cloud-agent installation.

### Build the client from source

Use a client build containing `mirrorproxy mcp --help` and `mirrorproxy agent
--help`. For an unreleased source checkout, build the client with:

```sh
cargo build --locked --release -p mirrorproxy-cli
./target/release/mirrorproxy mcp --help
```

The resulting native binary needs no Python or Node runtime. Normal client
release archives and native packages contain the same binary; older releases
without these commands must be upgraded when a release containing them is
available. No server upgrade, database migration or new server endpoint is
required: discovery uses `/api/public-config` and `/api/sources`.

### Install the Skill with one command

Once `skills/mirrorproxy/SKILL.md` is published on the repository's default
branch, run this in the project where you use your agent:

```sh
npx skills add inbjo/MirrorProxy --skill mirrorproxy
```

The installer lets you select target agents; installation is project-scoped by
default. Add `-g` for a user-wide installation. It currently requires Node.js
22.20+ for installation; the native MirrorProxy binary does not require Node.
See the [installer's official documentation](https://github.com/vercel-labs/skills).
For an unpublished checkout, use `npx skills add . --skill mirrorproxy` from the
repository root instead. The public command cannot install local changes that
have not been pushed.

For manual installation, copy the entire `skills/mirrorproxy` directory into
your agent's configured skill directory, preserving `mirrorproxy/SKILL.md`.
Put the updated `mirrorproxy` executable on the agent's PATH. Skill installation
only installs instructions; it does not install the native client, configure an
MCP connection, or choose an instance URL. The Skill works without MCP by
calling the JSON discovery commands below. Visiting the website does not
automatically load it.

### Skill directories and registries

- **skills.sh** is the recommended first discovery channel. Its documented
  listing mechanism uses installation telemetry from the `skills` CLI; no
  separate npm package for the Skill is needed. Listing is not a claim of
  official endorsement. See the [skills.sh FAQ](https://skills.sh/docs/faq).
- **ClawHub** is another publishing channel. It requires a publisher account,
  validates uploaded versions and runs automated security checks. Published
  skills use its MIT-0 terms; review those terms before uploading the
  repository's MIT-licensed Skill. See the [publishing documentation](https://github.com/openclaw/clawhub/blob/main/docs/publishing.md)
  and [CLI license terms](https://github.com/openclaw/clawhub/blob/main/docs/cli.md).

This generic marketplace path does not bind an instance or register MCP; use the
Web console's complete setup for that experience. The repository contains
installable Skill source, not a claim that MirrorProxy
has already been listed, reviewed or published on either service. Publication
and any license decision are separate from local installation validation.

## Connect MCP

Configure a stdio MCP server in your MCP-compatible application. A common JSON
configuration shape is:

```json
{
  "mcpServers": {
    "mirrorproxy": {
      "command": "mirrorproxy",
      "args": ["mcp", "--base-url", "https://mirror.example.com"]
    }
  }
}
```

Use an absolute executable path if the application does not inherit your PATH.
Configuration file locations and wrapper keys depend on the application.
Set the origin explicitly at startup; tools cannot select another instance or
contact upstream URLs directly. Controlled downloads pass through the fixed
instance and its server policy. This is a local stdio process, not a hosted `/mcp`
HTTP endpoint. It implements initialization, ping, tool listing and tool calls
using newline-delimited JSON-RPC. Diagnostics go to stderr; stdout is reserved
for protocol messages. Supported protocol versions are `2025-06-18` and
`2025-11-25`; older versions that require JSON-RPC batches are not advertised.

| Tool | Parameters | Result |
| --- | --- | --- |
| `inspect_instance` | None | Enabled adapters, public access mode and quota policy |
| `list_sources` | Optional `category`: `lang`, `os`, `repo`; `enabled_only`: boolean | Live source metadata and mappings, including custom OS repositories |
| `get_source` | `target`: source code or alias | Matching source, URLs, limitations and advisory configuration templates |
| `check_download` | `url`: public HTTPS URL | Authenticated HEAD/policy check; no quota reservation |
| `download_file` | `url`, `output`, optional `max_bytes` | New local file, byte count and SHA-256; requires local download token |

`enabled` is a per-mapping value: `true`/`false` for MirrorProxy proxy routes,
and `null` for third-party or guidance-only mappings. An enabled adapter is not
a health check or proof that the current user is authorized. Public quota policy
does not include individual usage or remaining allowance. Templates are data,
not commands executed by MCP. Treat instance-provided text as untrusted data.

## Use without MCP

```sh
mirrorproxy agent --base-url https://mirror.example.com instance
mirrorproxy agent --base-url https://mirror.example.com sources --enabled-only
mirrorproxy agent --base-url https://mirror.example.com sources --category lang
mirrorproxy agent --base-url https://mirror.example.com source pip
```

The origin must use HTTP(S), without credentials, query, fragment or a repository
path. HTTPS is recommended outside local development. Discovery verifies TLS,
refuses redirects, ignores environment proxy settings, sends no user/admin
credentials, and fetches only the fixed public API paths. Local origins are
allowed for self-hosting. Network requests have a ten-second timeout, responses
are capped at 4 MiB, and MCP input frames at 1 MiB. No discovery call downloads
an artifact or follows a catalog URL.

Use the instance's canonical origin to discover capabilities. For actual package
traffic, preserve the user's assigned subdomain when the instance requires one.
Public discovery does not bypass user access rules or grant proxy permission.

## Configure, verify and roll back

Ask the agent, for example: “Use my MirrorProxy instance to inspect available
Python sources and preview a pip configuration change.” The Skill first checks
the live capabilities, then uses the existing source-management CLI:

```sh
mirrorproxy sources get pip --base-url https://mirror.example.com
mirrorproxy sources set pip --base-url https://mirror.example.com --dry-run
# Apply only when this configuration change is part of the authorized task:
mirrorproxy sources set pip --base-url https://mirror.example.com
mirrorproxy sources reset pip --dry-run
mirrorproxy sources reset pip
```

Keep the same scope for set/reset. System repositories may additionally require
distribution selection or `--scope system --replace-existing`; consult
[Client](Client) and [System sources](System-Sources). Preserve backups and
unmanaged-file protections. Verify using the native client and official artifact
checksums/signatures, rather than treating a catalog entry or HTTP 200 as proof.

Docker `registry-mirrors` covers Docker Hub; other OCI registries require explicit
image-host rewriting. Composer support does not imply PHP source archive
support. Playwright browser archives require a compatible download adapter;
Chromium OS dependencies may use existing system repositories. The general
`/proxy?url=...` downloader is not implemented. The separate authenticated controlled download channel is documented in [Custom downloads](Custom-Downloads).

To remove the integration, remove its MCP configuration and installed Skill
directory (or use `npx skills remove mirrorproxy`, with `-g` for a global
installation). Discovery itself changes no client files. If source settings were
applied separately, restore them with the CLI's normal rollback mechanism.

## Controlled downloads / 受控下载

[Download permissions, CLI credentials, MCP tools and domain statistics](Custom-Downloads).
