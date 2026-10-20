# Getting started: Docker + Caddy

This path runs MirrorProxy with Caddy handling HTTPS certificates and renewal.
Already have Nginx or another gateway, or prefer the binary? See [Deployment](Deployment).

> **Prepare a hostname, Docker Compose, and free ports 80/443.** Replace every `mirror.example.com` below. Its A/AAAA records must point correctly to this server. Other web servers must not occupy these ports.

## 1. Create two files

Create `compose.yml` in a new directory. Change the hostname and initial password:

```yaml
services:
  mirrorproxy:
    image: kudang/mirrorproxy:1.4.4
    restart: unless-stopped
    environment:
      MIRRORPROXY_ADMIN_PASSWORD: 'replace-with-a-long-unique-password'
      MIRRORPROXY_PUBLIC_BASE_URL: 'https://mirror.example.com'
      MIRRORPROXY_TRUSTED_PROXIES: '172.30.40.2'
    volumes:
      - mirrorproxy-data:/data
    networks:
      default:
        ipv4_address: 172.30.40.3
  caddy:
    image: caddy:2
    restart: unless-stopped
    ports:
      - '80:80'
      - '443:443'
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy-data:/data
      - caddy-config:/config
    networks:
      default:
        ipv4_address: 172.30.40.2
volumes:
  mirrorproxy-data:
  caddy-data:
  caddy-config:
networks:
  default:
    ipam:
      config:
        - subnet: 172.30.40.0/24
          gateway: 172.30.40.1
```

Create `Caddyfile` beside it:

```caddyfile
mirror.example.com {
    reverse_proxy mirrorproxy:3000
}
```

The two container addresses are fixed; MirrorProxy trusts only Caddy at `172.30.40.2`.
If this subnet conflicts with your network, choose an unused subnet and update the gateway, both addresses, and trusted proxy together.

## 2. Start and sign in

```bash
docker compose config --quiet
docker compose up -d
docker compose logs --tail=50 caddy mirrorproxy
```

After certificate issuance succeeds, open **`https://mirror.example.com/admin`**.
Use `admin` and the password you configured. If it fails, check [Troubleshooting](Troubleshooting); do not delete the data volume.

> **The initial password is used once.** Changing `MIRRORPROXY_ADMIN_PASSWORD` does not replace an administrator password already in the database.
> **Keep the volumes.** `mirrorproxy-data` stores application data; `caddy-data` stores certificates. Do not run `docker compose down -v`.

## 3. Verify a real proxy request

```bash
curl -f https://mirror.example.com/healthz
curl -f https://mirror.example.com/npm/is-number
```

The first request checks the service; the second should return npm package metadata.
Also sign in and reload the console to verify the browser session. An HTTP 200 alone does not prove the whole proxy path works.

## Next

- Configure your computer: [Client](Client).
- Configure container pulls: [Container images](Container-Registries-and-Runtimes).
- Manage accounts, quotas, and monitoring: [Administration](Administration).
- Upgrade, back up, or use systemd: [Advanced deployment](Advanced-Deployment).

[Caddy documentation](https://caddyserver.com/docs/automatic-https) · [简体中文](Getting-Started-zh-CN)
