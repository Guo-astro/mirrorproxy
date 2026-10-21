# Advanced deployment and operations

**Complete [basic deployment](Deployment) and HTTPS login first.** Private management listeners, enterprise CAs, and user subdomains are optional.

## systemd

For a new Linux binary installation already verified using the deployment guide. Stop the foreground process with `Ctrl+C`.
Confirm that **`/opt/mirrorproxy` does not exist and the `mirrorproxy` system account has not been created**, then migrate the whole directory:

```bash
sudo useradd --system --home-dir /opt/mirrorproxy --shell /usr/sbin/nologin mirrorproxy
sudo mv -T "$HOME/mirrorproxy" /opt/mirrorproxy
sudo chown -R mirrorproxy:mirrorproxy /opt/mirrorproxy
sudo /opt/mirrorproxy/mirrorproxy-server --config /opt/mirrorproxy/config.toml install   --working-directory /opt/mirrorproxy --enable --start
sudo systemctl status mirrorproxy --no-pager
```

This keeps the existing database and GeoIP data rather than initializing another password.
`mv -T` is a GNU/Linux command and refuses to overwrite a non-empty destination.
Do not rerun first-install commands on an existing deployment; inspect its real service and data paths first.
The generated service runs as a dedicated account. Read logs with `sudo journalctl -u mirrorproxy -n 100 --no-pager`.

- Preview the unit: add `--dry-run`; no file is written and no service starts.
- Native HTTPS on ports 80/443: add `--privileged-ports` to grant low-port binding only.
- Supply master keys and other secrets through restricted systemd configuration, not command-line arguments or committed files.

## Upgrade, back up, and roll back

**Back up data and configuration, verify the new version, then replace the program.** Preserve cache, GeoIP, and ACME directories too.

For Docker, edit the pinned image version in `compose.yml`, then:

```bash
docker compose pull mirrorproxy
docker compose up -d mirrorproxy
docker compose logs --tail=50 mirrorproxy
```

Do not run `down -v` or delete the old volumes. Back up the database/configuration and record the old image tag first;
rollback uses that tag with the same volumes.

For the binary service, create a consistent SQLite backup; the destination must not already exist:

```bash
sudo install -d -o mirrorproxy -g mirrorproxy /var/backups/mirrorproxy
sudo -u mirrorproxy /opt/mirrorproxy/mirrorproxy-server   --config /opt/mirrorproxy/config.toml backup /var/backups/mirrorproxy/before-upgrade.sqlite3
```

Also retain `config.toml`, certificates, and the master key. Download the correct architecture and verify SHA-256.
Stop the service, keep the old executable, replace it, and restart. Verify `/healthz`, `/version`, console reload, and a real download.

**Stop the service before restoring a database.** Use the actual paths with
`mirrorproxy-server --config <config> restore <backup> --force`. The command validates the backup and retains replaced data.
An older executable may not support a newer database schema; check compatibility before a cross-version rollback.

## Which configuration takes effect?

- Listener addresses and service paths come from the actual `--config` and matching environment variables; restart after changes.
- Some runtime values are seeded into SQLite on first use and then managed in the console. Restarting does not overwrite every stored value from TOML.
- Explicit outbound-proxy and upstream-TLS environment overrides replace their stored settings at startup.
- ACME variables such as `MIRRORPROXY_ACME_*` make the corresponding console settings read-only.
- `MIRRORPROXY_ADMIN_PASSWORD` initializes an administrator; it does not continuously synchronize passwords.

Confirm the active config file, volume, and instance before editing. See [Troubleshooting](Troubleshooting).

## Private management and metrics

Optionally add a local management listener:

```toml
[management]
enabled = true
listen_addr = "127.0.0.1:3001"
```

Restart to apply. This **does not remove `/admin` from the public listener**. Authentication and an HTTPS entry point are still needed.
`/metrics` is local-only by default. For remote collection, explicitly set `metrics.local_only = false` and restrict sources at a firewall or gateway.

## Enterprise CA and upstream TLS

Only needed for private CAs on enterprise upstreams:

```toml
[upstream_tls]
ca_certificates = ["/etc/mirrorproxy/ca/company-root.pem"]
insecure_skip_verify = false
```

Also mount the file in Docker:

```yaml
volumes:
  - ./company-root.pem:/etc/mirrorproxy/ca/company-root.pem:ro
```

Environment equivalents are `MIRRORPROXY_UPSTREAM_TLS_CA_CERTIFICATES` (comma-separated paths) and
`MIRRORPROXY_UPSTREAM_TLS_INSECURE_SKIP_VERIFY`. Public and operating-system roots are trusted by default.
These options affect mirror upstreams only, not ACME, DNS APIs, or OAuth. Do not disable certificate verification to bypass deployment failures.

## User subdomains and multiple proxies

`subdomain_required` is an optional per-user routing/billing mode. Prepare wildcard DNS, wildcard TLS, and a Host-preserving gateway,
then confirm infrastructure readiness in the console. Otherwise main-domain package paths can be rejected by policy.
For multiple proxies, trust actual intermediate nodes only. Address chains are read right to left; see [GeoIP and IP rules](GeoIP-and-IP-Access-Control).

[简体中文](Advanced-Deployment-zh-CN) · [Basic deployment](Deployment)
