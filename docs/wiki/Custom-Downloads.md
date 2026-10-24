# Controlled custom downloads

This optional channel supplements existing package mirrors with public artifact
downloads such as source archives. It is disabled by default. It is not an HTTP
forward proxy, CONNECT tunnel, upload API, or private artifact service.

## Enable and authorize

A super administrator opens **Custom downloads** in `/admin`, enables the
channel, and grants permission to a registered user or an existing group.
Registration alone grants nothing. A direct user deny overrides group grants;
otherwise a direct user allow or an allowed group grants access. Inherit removes
the direct rule. Group deny grants nothing but does not override another allowed
group. Existing group membership management remains under Users & groups.

The user opens `/account` and creates a named, expiring **download-only token**.
The token has only `download:fetch` scope, is shown once, and is stored hashed on
the server. Each user can hold at most 20 active tokens, lasting 1–365 days.
These tokens cannot log in or access administration APIs. Browser sessions and
administrator tokens cannot authenticate download requests. Account disablement,
permission changes, token expiry and revocation are checked again during transfer.
Revoking a token cancels that user's active downloads; permission changes cancel
active downloads so that policy takes effect promptly.

## CLI and MCP

Install a client build containing `download`. Save the token through stdin using
`mirrorproxy download --base-url https://mirror.example.org auth --token-stdin`.
On a terminal, paste the token, then send EOF (Ctrl-D on Unix; Ctrl-Z then Enter
on Windows). Input may be echoed by the terminal: use a private terminal, or feed
stdin from a local secret manager. Do not put the token in command arguments,
chat, Skill content, project files, or shared MCP configuration.

The credential is saved under `~/.mirrorproxy/credentials`, keyed by exact
normalized instance origin, with mode 0600 on Unix. On Windows it inherits the
user profile's ACL; do not use a shared profile directory. The
`MIRRORPROXY_DOWNLOAD_TOKEN` environment variable can override the saved token
for a managed local process. HTTPS is required except for loopback development.
The download client refuses redirects from the MirrorProxy origin.

```sh
mirrorproxy download --base-url https://mirror.example.org check https://example.org/source.tar.xz
mirrorproxy download --base-url https://mirror.example.org fetch https://example.org/source.tar.xz --output source.tar.xz
mirrorproxy download --base-url https://mirror.example.org auth --logout
```

Downloads refuse existing outputs and symlinks. A private temporary file is
removed on failure; on success the complete file is published without overwrite
using a hard link. The output filesystem must support hard links. Results show
path, size and SHA-256; compare with an independently trusted upstream digest or
signature. The local default file limit is 2 GiB; `--max-bytes` can lower it or
raise it up to 64 GiB, still subject to the server limit.

The existing MCP connection automatically exposes `check_download` and
`download_file`; no second MCP server is needed. The tools load the same local
credential. They never take or return tokens. `inspect_instance` reports the
public `controlled_download` policy, not personal permission or quota.
`check_download` issues HEAD through the server; it does not reserve quota or
guarantee later success. `download_file` writes a new local file and consumes
quota, and is marked as a writing, non-idempotent tool. File bodies never enter
the model context. Existing Skill installations must be refreshed with setup to
include this guidance; restart the Agent to refresh the MCP tool list.

## HTTP contract

- `GET /api/download?url=<percent-encoded HTTPS URL>`: streamed attachment.
- `HEAD /api/download?url=...`: metadata without a file body.
- `GET /api/download/check?url=...`: authenticated policy/HEAD check as JSON.
- All require `Authorization: Bearer <download-token>`. URL tokens are unsupported.
- Only port 443 and public DNS hostnames are accepted. IP literals, credentials,
  fragments, private/special IPv4 and IPv6 addresses are rejected.
- All DNS answers must be public. Connections pin those checked addresses and
  keep TLS hostname verification. Every redirect repeats URL, DNS and block
  checks, up to five redirects. HTTP downgrade and unsafe schemes are refused.
- Requests bypass environment/configured proxies and do not inherit insecure
  mirror TLS settings, upstream credentials, cookies or client request headers.
- A single `Range: bytes=start-end` or `bytes=start-` is supported by the HTTP
  endpoint when the upstream supports it. Suffix/multiple ranges are refused.
  The CLI performs complete downloads only.
- Responses are attachments with octet-stream content type and `no-store`.
  Upstream cookies, HTML content types and authentication challenges are removed.
- Upstream failures are sanitized; full URLs and query strings are not recorded
  in download statistics or application request logs. Your reverse proxy must
  also avoid recording URL query strings containing signed download links.

## Limits and statistics

`[download]` in `config.example.toml` documents defaults: 2 GiB/file,
5 GiB/user/day (UTC), 16 instance downloads, 2 per user, 30 requests/user/minute,
30-minute deadline and a shared 20 MiB/s output rate. Config values are persisted
using the existing runtime configuration mechanism. Saving download policy
cancels active transfers. Existing global, user and billing-group monthly quotas
also apply; the same transaction checks the budgets before each queued chunk.
Bidirectional monthly billing follows the existing instance setting. Daily
budgets count output bytes once. Small queued chunks may be billed even when a
client disconnects. Midstream errors terminate the body and the CLI removes its
partial file; HTTP status cannot change after headers have been sent.

The admin panel reports request/error/denial counts, metered output and upstream
body bytes by exact final hostname, user and day, with entry-host associations.
There is no shared cache in this version. Header/DNS failures do not contribute
body bytes; bytes already read from an upstream before an interruption are still
reported. Statistics retain 90 days and contain no URL paths or query strings.
Top hostname/user lists show 200 entries, details show up to 2,000 rows; totals
and daily summaries include all matching records. Filter by exact entry/final
hostname or user and select 1–90 days.

Domain blocks match exact labels, optionally including all subdomains. They
apply to entry hosts and redirect targets and cancel matching active transfers.
Admin actions record actor, time and reason in the audit log. The active list
also supports terminating individual downloads. Shared CDN domains may serve
many projects; choose the narrowest appropriate hostname when blocking.

## Upgrade and rollback

Schema v5 adds tables without changing existing users, sessions or mirror routes.
No migration grants permission automatically. Back up the SQLite database before
upgrading. To disable the feature, turn off downloads in the admin panel, revoke
tokens if needed, and remove local credentials with `auth --logout`. Existing
mirror clients and the discovery tools remain available. No release publication
or production deployment is implied by enabling this feature in source.
