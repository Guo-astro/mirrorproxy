# MirrorProxy guide

MirrorProxy puts code sources, packages, and public container images behind your own hostname, with on-demand proxying and caching.

> **New installation?** Start with [Getting Started](Getting-Started). Already have a server and only want to configure your computer? Go to [Client](Client).

## Start with your task

| I want to… | Read |
| --- | --- |
| Deploy with Docker and automatic HTTPS | [Getting Started](Getting-Started) |
| Run the binary or use an existing reverse proxy | [Deployment](Deployment) |
| Configure Nginx, Caddy, Traefik, Apache, or HAProxy | [Reverse proxy examples](Reverse-Proxy) |
| Point npm, pip, Cargo, and other tools at my server | [Client](Client) |
| Accelerate Docker Hub, GHCR, and other image pulls | [Container images](Container-Registries-and-Runtimes) |
| Change passwords, manage quotas, or view traffic | [Administration](Administration) |
| Fix login, certificate, or download failures | [Troubleshooting](Troubleshooting) |

## Three things to remember

- **The server is `mirrorproxy-server`; the client is `mirrorproxy`.** The client installer does not deploy a server.
- **Use HTTPS.** v1.4.4 supports HTTP password login with a persistent security warning; do not delete the database to fix login.
- **Docker mirrors cover Docker Hub only.** Rewrite other supported registry hosts explicitly. Image pushes are not supported.

## Read when needed

[Supported sources](Proxy-Adapters) · [Client installation options](Distribution) ·
[Advanced deployment](Advanced-Deployment) · [Native HTTPS / ACME](ACME-Certificates) ·
[GeoIP and IP rules](GeoIP-and-IP-Access-Control) · [Development](Development-and-Roadmap)

[简体中文](Home-zh-CN)
