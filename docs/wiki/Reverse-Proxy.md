# Common reverse proxies: HTTPS examples

Start the backend using [Deployment](Deployment), then choose one proxy below.
**An ordinary installation needs one hostname and one proxy; leave MirrorProxy ACME off.**

| Software | Certificates | Example |
| --- | --- | --- |
| Caddy | Automatic issuance and renewal | [Configuration](#caddy) |
| Nginx / OpenResty | Existing certificates or Certbot | [Configuration](#nginx) |
| Traefik v3 | Built-in ACME | [Configuration](#traefik) |
| Apache HTTP Server 2.4 | Existing certificates or Certbot | [Configuration](#apache) |
| HAProxy | PEM loaded from an external certificate tool | [Configuration](#haproxy) |
| Nginx Proxy Manager | Certificate management in its UI | [Form example](#npm) |

## Shared prerequisites

- Replace every `mirror.example.com`, point DNS at the gateway, and make its public ports 80/443 reachable.
- File examples below assume **the proxy runs on the host**, with backend `http://127.0.0.1:3000`. Proxy the whole site without a `/mirrorproxy/` prefix; preserve paths and query strings.
- The binary trusts loopback; the deployment page's Docker backend trusts its fixed bridge gateway `172.30.40.1`. **Trust only actual proxies, never `0.0.0.0/0`.**
- For a container proxy, use the shared-network service name and a fixed trusted proxy address instead. See the complete [quick start](Getting-Started).
- Merge into existing configurations, retain a copy, check syntax, and then reload. Do not overwrite other sites.

## Caddy

Install using [Caddy's instructions](https://caddyserver.com/docs/install). With its official systemd service, edit `/etc/caddy/Caddyfile`:

```caddyfile
mirror.example.com {
    reverse_proxy 127.0.0.1:3000
}
```

```bash
sudo caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile
sudo systemctl reload caddy
```

Caddy obtains and renews certificates and redirects HTTP to HTTPS. Certbot is unnecessary.
If its service has not been started yet, use `sudo systemctl enable --now caddy`.
[Automatic HTTPS](https://caddyserver.com/docs/automatic-https)

## Nginx

The proxy directives also work with OpenResty. A common Debian/Ubuntu Nginx site file is `/etc/nginx/conf.d/mirrorproxy.conf`.
With Nginx-based panels such as aaPanel or 1Panel, edit the matching site's reverse-proxy settings rather than creating duplicate sites.

**No certificate yet? Start an HTTP site, then let Certbot enable HTTPS.** Initial site:

```nginx
server {
    listen 80;
    server_name mirror.example.com;
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Host $http_host;
        proxy_set_header X-Forwarded-Host $http_host;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header X-Forwarded-For $remote_addr;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header Connection "";
        proxy_buffering off;
        proxy_read_timeout 300s;
    }
}
```

Check and load it, install Certbot using its [official instructions](https://certbot.eff.org/instructions?ws=nginx&os=snap), then run:

```bash
sudo nginx -t
sudo systemctl reload nginx
sudo certbot --nginx --redirect -d mirror.example.com
sudo certbot renew --dry-run
```

Certbot adds the TLS listener, certificate paths, and HTTP redirect. OpenResty and panel-managed installations may use different executables and service names;
use their certificate tools or a matching certificate installation, rather than blindly running the system `nginx` command.

<details>
<summary>Already have certificates? Complete 80/443 site</summary>

The certificate files must already exist. Adjust the paths:

```nginx
server {
    listen 80;
    server_name mirror.example.com;
    return 301 https://mirror.example.com$request_uri;
}
server {
    listen 443 ssl;
    server_name mirror.example.com;
    ssl_certificate /etc/letsencrypt/live/mirror.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/mirror.example.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Host $http_host;
        proxy_set_header X-Forwarded-Host $http_host;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header X-Forwarded-For $remote_addr;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header Connection "";
        proxy_buffering off;
        proxy_read_timeout 300s;
    }
}
```

Run `sudo nginx -t` and only then `sudo systemctl reload nginx`.

</details>

The example overwrites client-supplied scheme/address headers, preserves the original Host and port, and does not append a URI to `proxy_pass`.
Reload Nginx after certificate renewal. [Proxy directives](https://nginx.org/en/docs/http/ngx_http_proxy_module.html)

## Traefik

For Traefik v3 running on the host, using a file provider without a Docker socket.
Merge this static configuration into `/etc/traefik/traefik.yml` and replace the email:

```yaml
entryPoints:
  web:
    address: ':80'
    http:
      redirections:
        entryPoint:
          to: websecure
          scheme: https
  websecure:
    address: ':443'
providers:
  file:
    filename: /etc/traefik/mirrorproxy.yml
    watch: true
certificatesResolvers:
  letsencrypt:
    acme:
      email: admin@example.com
      storage: /var/lib/traefik/acme.json
      httpChallenge:
        entryPoint: web
```

Create `/etc/traefik/mirrorproxy.yml` for the dynamic route:

```yaml
http:
  routers:
    mirrorproxy:
      rule: 'Host(`mirror.example.com`)'
      entryPoints: [websecure]
      service: mirrorproxy
      tls:
        certResolver: letsencrypt
  services:
    mirrorproxy:
      loadBalancer:
        passHostHeader: true
        servers:
          - url: 'http://127.0.0.1:3000'
```

The Traefik account must be able to write `/var/lib/traefik`. Keep `acme.json` persistent and readable/writable only by that account (0600).
Restart after static configuration changes; an existing systemd service can use `sudo systemctl restart traefik`.
For foreground operation, use `traefik --configFile=/etc/traefik/traefik.yml`; binding 80/443 needs permission.
Traefik issues certificates, preserves Host, and supplies scheme/client-address headers. Do not enable `forwardedHeaders.insecure`.
[ACME reference](https://doc.traefik.io/traefik/reference/install-configuration/tls/certificate-resolvers/acme/)

## Apache

For Apache HTTP Server 2.4 with `ssl`, `headers`, `proxy`, `proxy_http`, and `alias` enabled.
Debian/Ubuntu can use `sudo a2enmod ssl headers proxy proxy_http alias`.
Prepare a valid certificate first. If needed, use the [Certbot Apache guide](https://certbot.eff.org/instructions?ws=apache&os=snap)
with an existing HTTP site before applying the complete configuration below.

Save `/etc/apache2/sites-available/mirrorproxy.conf`, using your certificate paths:

```apache
<VirtualHost *:80>
    ServerName mirror.example.com
    Redirect permanent / https://mirror.example.com/
</VirtualHost>
<VirtualHost *:443>
    ServerName mirror.example.com
    SSLEngine on
    SSLCertificateFile /etc/letsencrypt/live/mirror.example.com/fullchain.pem
    SSLCertificateKeyFile /etc/letsencrypt/live/mirror.example.com/privkey.pem
    SSLProtocol -all +TLSv1.2 +TLSv1.3
    ProxyRequests Off
    ProxyPreserveHost On
    ProxyAddHeaders Off
    RequestHeader set X-Forwarded-Proto "https"
    RequestHeader set X-Forwarded-For "expr=%{REMOTE_ADDR}"
    RequestHeader unset X-Forwarded-Host
    RequestHeader unset X-Real-IP
    AllowEncodedSlashes NoDecode
    ProxyPass / http://127.0.0.1:3000/ nocanon
    ProxyPassReverse / http://127.0.0.1:3000/
    ProxyTimeout 300
</VirtualHost>
```

```bash
sudo a2ensite mirrorproxy
sudo apachectl configtest
sudo systemctl reload apache2
```

Other distributions may use `httpd` for paths and services. Reload Apache after certificate renewal.
The example disables forward proxying, proxies the whole site, and overwrites client-supplied address headers.
[Apache proxy reference](https://httpd.apache.org/docs/2.4/mod/mod_proxy.html)

## HAProxy

Prepare a certificate first. HAProxy's PEM file contains both the chain and private key; `fullchain.pem` alone is insufficient:

```bash
sudo install -d -m 0700 /etc/haproxy/certs
sudo sh -c 'umask 077; cat /etc/letsencrypt/live/mirror.example.com/fullchain.pem /etc/letsencrypt/live/mirror.example.com/privkey.pem > /etc/haproxy/certs/mirror.pem'
```

For a dedicated `/etc/haproxy/haproxy.cfg`; merge only the relevant frontend/backend sections into a shared installation:

```haproxy
global
    log stdout format raw local0
    ssl-default-bind-options ssl-min-ver TLSv1.2

defaults
    mode http
    timeout connect 10s
    timeout client 300s
    timeout server 300s

frontend mirror_http
    bind :80
    http-request redirect scheme https code 301

frontend mirror_https
    bind :443 ssl crt /etc/haproxy/certs/mirror.pem
    http-request set-header X-Forwarded-Proto https
    http-request set-header X-Forwarded-For %[src]
    http-request del-header X-Forwarded-Host
    http-request del-header X-Real-IP
    default_backend mirrorproxy

backend mirrorproxy
    server app 127.0.0.1:3000 check
```

```bash
sudo haproxy -c -f /etc/haproxy/haproxy.cfg
sudo systemctl reload haproxy
```

After external certificate renewal, regenerate `mirror.pem`, validate, and reload HAProxy. Updating only the original certificate files is insufficient.
[HAProxy TLS example](https://www.haproxy.com/documentation/haproxy-configuration-tutorials/protocol-support/http/)

## NPM

In Nginx Proxy Manager, open **Proxy Hosts → Add Proxy Host**:

| Field | Example |
| --- | --- |
| Domain Names | `mirror.example.com` |
| Scheme | `http` |
| Forward Hostname / IP | `127.0.0.1` for a host proxy; `mirrorproxy` on a shared container network |
| Forward Port | `3000` |
| SSL Certificate | Request a new SSL Certificate, or select an existing valid certificate |
| Force SSL | Enabled |

NPM usually runs in a container: **`mirrorproxy:3000` on the same network** is the common backend.
Do not copy `127.0.0.1` into that setup. MirrorProxy should trust NPM's actual fixed container address.
NPM cannot share the same ports 80/443 with Caddy/Nginx. Save and open the HTTPS hostname.
[NPM guide](https://nginxproxymanager.com/guide/)

## Final check

```bash
curl -f https://mirror.example.com/healthz
curl -f https://mirror.example.com/npm/is-number
```

Also sign in at `/admin` and reload. For 502, certificate, or login failures, follow [Troubleshooting](Troubleshooting).

[简体中文](Reverse-Proxy-zh-CN) · [Deployment](Deployment)
