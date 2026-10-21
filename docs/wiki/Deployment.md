# Deployment: run the server, then add HTTPS

| Your situation | Start here |
| --- | --- |
| New Docker installation with automatic HTTPS | [Docker + Caddy quick start](Getting-Started) |
| Nginx/Caddy or another gateway already runs on the host | Step 1A, then choose a reverse proxy |
| No Docker | Step 1B: download the binary |
| No reverse proxy | [Native MirrorProxy HTTPS](ACME-Certificates) |

> **v1.4.4 supports HTTP password login with a persistent security banner.** HTTP passwords and sessions can be intercepted or modified; enable HTTPS using the steps below. For older login failures, see [Troubleshooting](Troubleshooting).

Replace `mirror.example.com` with your hostname and point its DNS records at the server.

## Step 1A: Docker backend for a host reverse proxy

**Skip this if you used the Docker + Caddy quick start.** This example is for Linux with the reverse proxy installed on the host.
Create `compose.yml`:

```yaml
services:
  mirrorproxy:
    image: kudang/mirrorproxy:1.5.0
    restart: unless-stopped
    ports:
      - '127.0.0.1:3000:3000'
    environment:
      MIRRORPROXY_ADMIN_PASSWORD: 'replace-with-a-long-unique-password'
      MIRRORPROXY_PUBLIC_BASE_URL: 'https://mirror.example.com'
      MIRRORPROXY_TRUSTED_PROXIES: '172.30.40.1'
    volumes:
      - mirrorproxy-data:/data
networks:
  default:
    ipam:
      config:
        - subnet: 172.30.40.0/24
          gateway: 172.30.40.1
volumes:
  mirrorproxy-data:
```

```bash
docker compose config --quiet
docker compose up -d
curl -f http://127.0.0.1:3000/healthz
```

Only the host loopback publishes the backend port. The container trusts this example's host bridge address, `172.30.40.1`.
If the subnet conflicts with your network, change the subnet, gateway, and trusted address together. Other networks need their actual proxy address.
Existing databases may contain admin-saved runtime settings; check the public URL and trusted proxies in the console too.

## Step 1B: run the binary

For a **new Linux x86_64 installation**. Rust and Node.js are not required. Check the architecture with `uname -m`.
On ARM64, use `mirrorproxy-server-aarch64-unknown-linux-gnu.tar.gz` instead.

Download, verify, and extract. Run these steps in order and stop if verification fails:

```bash
mkdir -p "$HOME/mirrorproxy"
cd "$HOME/mirrorproxy"
version=v1.5.0
archive=mirrorproxy-server-x86_64-unknown-linux-musl.tar.gz
curl -fLO "https://github.com/inbjo/MirrorProxy/releases/download/$version/$archive"
curl -fLO "https://github.com/inbjo/MirrorProxy/releases/download/$version/$archive.sha256"
sha256sum -c "$archive.sha256" && tar -xzf "$archive"
```

Create `~/mirrorproxy/config.toml`, replacing the hostname first. Other options use server defaults; the archive's `config.example.toml` lists the full configuration.

```toml
listen_addr = "127.0.0.1:3000"
public_base_url = "https://mirror.example.com"
database_path = "mirrorproxy.sqlite3"
trusted_proxies = ["127.0.0.1", "::1"]
```

Start from that directory with your own initial password:

```bash
MIRRORPROXY_ADMIN_PASSWORD='replace-with-a-long-unique-password'   ./mirrorproxy-server --config ./config.toml serve
```

Leave the terminal running. In a second terminal, run `curl -f http://127.0.0.1:3000/healthz`.
Set up HTTPS below, then use [systemd](Advanced-Deployment#systemd) for a persistent service.
**Use `mirrorproxy-server`, not the `mirrorproxy` client, and keep the included `geoip/` directory.**

## Step 2: choose one reverse proxy for HTTPS

Only one service can own the same ports 80/443.

| Software | Complete example |
| --- | --- |
| Caddy (automatic certificates, fewest steps) | [Caddy](Reverse-Proxy#caddy) |
| Nginx / OpenResty / Nginx-based panels | [Nginx and certificate issuance](Reverse-Proxy#nginx) |
| Traefik | [Static and dynamic configuration](Reverse-Proxy#traefik) |
| Apache HTTP Server | [Virtual host configuration](Reverse-Proxy#apache) |
| HAProxy | [TLS and backend configuration](Reverse-Proxy#haproxy) |
| Nginx Proxy Manager | [NPM form example](Reverse-Proxy#npm) |

For example, Caddy **running on the host** uses this `Caddyfile`:

```caddyfile
mirror.example.com {
    reverse_proxy 127.0.0.1:3000
}
```

With working DNS and reachable public ports 80/443, Caddy handles certificates and redirects HTTP to HTTPS.
See the [complete example](Reverse-Proxy#caddy) for installation, file location, and reload commands.

> **Is the proxy also a container?** Its `127.0.0.1` points to itself. Use a shared-network service name such as `mirrorproxy:3000` in the quick start, and trust only the actual proxy container address.

## Step 3: verify and sign in

```bash
curl -f https://mirror.example.com/healthz
curl -f https://mirror.example.com/version
curl -f https://mirror.example.com/npm/is-number
```

Open **`https://mirror.example.com/admin`**, sign in as `admin` with the initial password, then reload.
This checks certificates, proxying, a real package request, and the browser session. Do not hide certificate failures with `curl -k`.

## Operations and advanced topics

- [Troubleshooting](Troubleshooting): login, 502, certificates, permissions, and ineffective configuration.
- [Advanced deployment](Advanced-Deployment): systemd, upgrades, rollback, backups, management ports, enterprise CAs, and user subdomains.
- [Native HTTPS / ACME](ACME-Certificates): manage certificates without a reverse proxy.

[Caddy documentation](https://caddyserver.com/docs/automatic-https) · [简体中文](Deployment-zh-CN)
