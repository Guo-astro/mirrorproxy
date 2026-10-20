# Troubleshooting

First confirm which instance you are using:

```bash
curl -f https://mirror.example.com/version
```

For Docker, use `docker compose logs --tail=100 mirrorproxy caddy`; for systemd, use
`journalctl -u mirrorproxy -n 100 --no-pager`. Redact passwords, tokens, and cookies before sharing logs.

## Correct password, but the console returns to login

**In v1.4.3, sign in at `https://your-host/admin`.** Docker and binary deployments share the same HTTP session limitation.

A login response can be 200 while the browser rejects the `Secure` session cookie and `__Host-` CSRF cookie.
Following requests return 401 and the UI misleadingly reports a password error. The initial password's strength is not the cause.
Resetting passwords or deleting the database does not fix it. Localhost exceptions do not prove remote HTTP works.

**Fixed in v1.4.4:** HTTP password login works with a persistent security banner and HTTPS guidance. HTTP and HTTPS use separate session cookies; sign in again after switching to HTTPS. HTTP passwords and sessions can still be intercepted or modified, so HTTPS is recommended. See [Issue #10](https://github.com/inbjo/MirrorProxy/issues/10) and [reverse proxy examples](Reverse-Proxy).

If HTTPS also fails, use browser developer tools to check login status, rejected cookies, and later `/admin/api/` requests.
Verify that the proxy preserves Host, supplies the correct scheme, and is the actual address listed in `trusted_proxies`.

## Changing the password environment variable has no effect

`MIRRORPROXY_ADMIN_PASSWORD` is **used only when the database has no administrators**; a non-empty value is used verbatim.
Recreating a container does not overwrite an existing password. An empty or missing value generates a random password shown only in the first startup log.

If you forgot it, run locally on the server. Save the newly generated random password securely:

```bash
# Docker
docker compose exec mirrorproxy mirrorproxy-server admin reset-password

# Binary service
sudo -u mirrorproxy /opt/mirrorproxy/mirrorproxy-server   --config /opt/mirrorproxy/config.toml admin reset-password
```

The command resets the original administrator and revokes that account's sessions.
Password changes in the console still have strength requirements. Five failures lock the account for 15 minutes;
ordinary restarts do not clear the lock. Avoid repeated attempts.

## HTTPS, 502, and downloads

| Symptom | Check first |
| --- | --- |
| Certificate issuance fails | Correct A/AAAA records, public reachability of ports 80/443, and port conflicts |
| Browser distrusts the certificate | Matching hostname, complete chain, and client trust for an enterprise CA |
| 502 Bad Gateway | Backend `/healthz` from the host; a container proxy must not use its own `127.0.0.1` |
| Console CSRF error | Host, scheme, trusted proxy, and the browser's actual HTTPS address |
| Docker Hub works but GHCR bypasses the proxy | Docker mirrors cover Docker Hub only; rewrite other image hosts |
| Image returns 401/403 | A 401 can be a normal auth challenge; check public access, login, and license requirements |
| 404 or manifest unknown | Upstream package, repository, tag, and enabled adapters; do not guess URLs |
| GeoIP shows Unknown | Private IP, missing XDB result, or incorrect trusted-proxy configuration |

## Editing a configuration file has no effect

Some runtime settings are seeded into SQLite on first use and then managed in the console;
a restart does not overwrite every stored value from TOML. Listener settings come from service startup configuration;
ACME environment variables and other specific overrides have their own precedence.
Check the actual `--config` path, container environment, and saved console values. See [Advanced deployment](Advanced-Deployment).

> **Keep the data and diagnose first.** Do not delete SQLite, run `docker compose down -v`, or disable TLS/SSRF/CSRF checks to bypass a failure.

[简体中文](Troubleshooting-zh-CN) · [Deployment](Deployment)
