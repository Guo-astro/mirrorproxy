# Container images: pull first, configure the runtime when needed

> **`registry-mirrors` covers Docker Hub only.** GHCR, Quay, MCR, and other supported registries need explicit image-host rewriting. Public pulls are supported; pushes are not.

## Use directly without changing runtime configuration

Replace `mirror.example.com`:

```bash
docker pull mirror.example.com/library/busybox:1.36.1
docker pull mirror.example.com/registry.k8s.io/pause:3.10
```

The home page's Container Registry Workbench converts image names, Compose, and Dockerfiles. Review the result and verify a real pull.

## Configure your platform

| Environment | Guide |
| --- | --- |
| Docker Engine, Docker Desktop, OrbStack | [Docker configuration](Docker-Engine-and-Desktop) |
| K3s and containerd | [Node configuration](Containerd-and-K3s) |
| NAS, Portainer, Podman, HPC, Harbor/Nexus | [Platform guides](NAS-Panels-and-Other-Runtimes) |

## Supported registries

| Registry | Original image | MirrorProxy path |
| --- | --- | --- |
| Docker Hub | `nginx:latest` | `<mirror-host>/nginx:latest` |
| GHCR | `ghcr.io/owner/image:tag` | `<mirror-host>/ghcr.io/owner/image:tag` |
| Quay | `quay.io/owner/image:tag` | `<mirror-host>/quay.io/owner/image:tag` |
| Kubernetes | `registry.k8s.io/pause:3.10` | `<mirror-host>/registry.k8s.io/pause:3.10` |
| GCR | `gcr.io/project/image:tag` | `<mirror-host>/gcr.io/project/image:tag` |
| Microsoft MCR | `mcr.microsoft.com/dotnet/runtime:8.0` | `<mirror-host>/mcr.microsoft.com/dotnet/runtime:8.0` |
| Elastic | `docker.elastic.co/elasticsearch/elasticsearch:8.13.4` | `<mirror-host>/docker.elastic.co/elasticsearch/elasticsearch:8.13.4` |
| GitLab | `registry.gitlab.com/group/project/image:tag` | `<mirror-host>/registry.gitlab.com/group/project/image:tag` |
| NVIDIA NVCR | `nvcr.io/nvidia/cuda:tag` | `<mirror-host>/nvcr.io/nvidia/cuda:tag` |
| Oracle | `container-registry.oracle.com/os/oraclelinux:tag` | `<mirror-host>/container-registry.oracle.com/os/oraclelinux:tag` |

`k8s.gcr.io` is accepted only as a legacy input alias and is served from the current
`registry.k8s.io` upstream. The OCI adapter is a public, read-only `GET`/`HEAD` pull path;
private-image credentials, license acceptance, pushes, deletes, and signing writes are outside this
contract.

Private GitLab projects, NGC organization or team images, and Oracle images that require login or
license acceptance never borrow server-side MirrorProxy credentials. Upstream `401` and `403`
responses remain authorization boundaries. A registry in this table means that anonymous public
projects can be pulled, not that private content is available.

## Troubleshooting checks

- `container_registries` in `/api/sources` is the live list; do not guess paths from page names.
- A 401 may be a normal Bearer challenge. Public proxying does not bypass 403 responses, license acceptance, or private-image authentication.
- A successful `/v2/` checks reachability only. Verify both manifests and blobs with a real pull.
- For direct-upstream traffic, check the active Docker/containerd/K3s configuration; they use different files.
- NAS search and CLI pulls may use different paths. Diagnose with real pulls and logs.

[简体中文](Container-Registries-and-Runtimes-zh-CN)
