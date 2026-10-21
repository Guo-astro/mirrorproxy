#!/usr/bin/env python3
"""Real public SDK/protocol smoke; run in an isolated Python environment.

uv run --no-project --with huggingface-hub==2.2.0 scripts/smoke-huggingface.py target/debug/mirrorproxy-server
Only the temporary local server can contact HF. Client-side HTTP hooks reject
any attempt to bypass MirrorProxy. All service/client data stays in a temp dir.
"""
import hashlib
import base64
import csv
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import io
import os
from pathlib import Path
import socket
import secrets
import subprocess
import sys
import tempfile
import time
import threading
from urllib.parse import urljoin, urlsplit

os.environ["HF_HUB_DISABLE_XET"] = "1"
os.environ["HF_HUB_DISABLE_TELEMETRY"] = "1"
os.environ["HF_HUB_DISABLE_IMPLICIT_TOKEN"] = "1"
os.environ["HF_HUB_DISABLE_UPDATE_CHECK"] = "1"

try:
    import httpx2 as httpx
except ImportError:
    import httpx
from huggingface_hub import HfApi, hf_hub_download, set_client_factory, snapshot_download


@contextmanager
def upstream_relay():
    """Optional test transport for environments where direct egress is blocked.

    Uses the environment's HTTP proxy with normal TLS verification. Only the
    trusted configured upstream changes; production destination checks stay on.
    This mode validates the alternative-upstream path, not direct HF egress.
    """
    if os.environ.get("MIRRORPROXY_HF_TEST_RELAY") != "1":
        yield None
        return
    hosts = {"huggingface.co", "cdn-lfs.huggingface.co", "cdn-lfs.hf.co", "cdn-lfs-us-1.hf.co", "cdn-lfs-eu-1.hf.co", "cas-bridge.xethub.hf.co", "cas-bridge.xethub-eu.hf.co", "us.aws.cdn.hf.co", "us.gcp.cdn.hf.co", "eu.aws.cdn.hf.co"}
    with httpx.Client(timeout=60, follow_redirects=False) as client:
        class Relay(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_HEAD(self):
                self.do_GET()

            def do_GET(self):
                try:
                    target = (base64.urlsafe_b64decode(self.path[len("/_redirect/"):]).decode()
                              if self.path.startswith("/_redirect/") else f"https://huggingface.co{self.path}")
                    parsed = urlsplit(target)
                    assert parsed.scheme == "https" and parsed.hostname in hosts and parsed.port in (None, 443)
                    forwarded = {k: v for k, v in self.headers.items() if k.lower() in {"accept", "accept-encoding", "range", "if-range", "if-none-match", "if-modified-since"}}
                    with client.stream(self.command, target, headers=forwarded) as response:
                        self.send_response(response.status_code)
                        local = f"http://127.0.0.1:{self.server.server_port}"
                        for key, value in response.headers.items():
                            if key.lower() in {"connection", "transfer-encoding", "set-cookie"}:
                                continue
                            if key.lower() == "location":
                                value = local + "/_redirect/" + base64.urlsafe_b64encode(urljoin(target, value).encode()).decode()
                            if key.lower() == "link":
                                value = value.replace("https://huggingface.co/", local + "/")
                            self.send_header(key, value)
                        self.end_headers()
                        if self.command != "HEAD":
                            for chunk in response.iter_raw():
                                self.wfile.write(chunk)
                except (BrokenPipeError, ConnectionResetError):
                    pass
        relay = ThreadingHTTPServer(("127.0.0.1", 0), Relay)
        thread = threading.Thread(target=relay.serve_forever, daemon=True)
        thread.start()
        try:
            yield f"http://127.0.0.1:{relay.server_port}"
        finally:
            relay.shutdown()
            relay.server_close()
            thread.join(timeout=5)


def main():
    binary = Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/mirrorproxy-server").resolve()
    with upstream_relay() as relay, tempfile.TemporaryDirectory(prefix="mirrorproxy-hf-smoke-") as directory:
        root = Path(directory)
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
        base = f"http://127.0.0.1:{port}"
        endpoint = f"{base}/huggingface"
        os.environ["HF_ENDPOINT"] = endpoint
        config = root / "config.toml"
        config.write_text(f'''listen_addr = "127.0.0.1:{port}"
database_path = "{root / 'service.sqlite3'}"
enabled_proxies = ["huggingface"]
[geoip]
enabled = false
[quota]
enabled = false
[cache]
enabled = true
directory = "{root / 'server-cache'}"
[registration]
mode = "disabled"
[timeout]
request_secs = 30
''')
        if relay:
            with config.open("a") as output:
                output.write(f'\n[upstreams]\nhuggingface = "{relay}"\n')
        env = {key: value for key, value in os.environ.items() if not key.startswith("MIRRORPROXY_")}
        env["MIRRORPROXY_ADMIN_PASSWORD"] = secrets.token_urlsafe(32)
        with (root / "server.log").open("w+") as log:
            server = subprocess.Popen([str(binary), "--config", str(config), "serve"], cwd=root, env=env, stdout=log, stderr=log)
            try:
                with httpx.Client(trust_env=False, timeout=60) as client:
                    for _ in range(100):
                        try:
                            if client.get(f"{base}/healthz").status_code == 200:
                                break
                        except httpx.TransportError:
                            pass
                        if server.poll() is not None:
                            raise RuntimeError("temporary server exited")
                        time.sleep(0.1)
                    else:
                        raise RuntimeError("temporary server did not become ready")

                    catalog = client.get(f"{base}/api/sources").raise_for_status().json()
                    assert any(s["target_code"] == "huggingface" for s in catalog["sources"])
                    requests = []

                    def require_proxy(request):
                        assert request.url.host == "127.0.0.1" and request.url.port == port, "SDK attempted a direct upstream request"
                        assert request.url.path.startswith("/huggingface/"), "SDK left the HF adapter"
                        requests.append(request.method)

                    set_client_factory(lambda: httpx.Client(trust_env=False, timeout=60, follow_redirects=True, event_hooks={"request": [require_proxy]}))
                    cache = root / "client-cache"
                    repo = "hf-internal-testing/tiny-random-gpt2"
                    metadata = HfApi(endpoint=endpoint, token=False).model_info(repo)
                    revision = metadata.sha
                    path = Path(hf_hub_download(repo, "model.safetensors", revision=revision, endpoint=endpoint, token=False, cache_dir=cache))
                    url = f"{endpoint}/{repo}/resolve/{revision}/model.safetensors"
                    head = client.head(url).raise_for_status()
                    assert head.headers["x-repo-commit"] == revision
                    assert "location" not in head.headers and "x-xet-hash" not in head.headers
                    assert head.headers["x-mirrorproxy-cache"] == "BYPASS"
                    payload = path.read_bytes()
                    assert len(payload) == int(head.headers["content-length"])
                    digest = hashlib.sha256(payload).hexdigest()
                    assert digest == head.headers["x-linked-etag"].strip('"'), "download digest differs from Hub metadata"
                    ranged = client.get(url, headers={"Range": "bytes=16-47"}).raise_for_status()
                    assert ranged.status_code == 206 and ranged.content == payload[16:48]
                    assert ranged.headers["content-range"] == f"bytes 16-47/{len(payload)}"
                    resumed = client.get(url, headers={"Range": "bytes=48-", "If-Range": head.headers["etag"]}).raise_for_status()
                    assert resumed.status_code == 206 and payload[:48] + resumed.content == payload

                    snapshot = Path(snapshot_download(repo, revision=revision, allow_patterns=["config.json", "model.safetensors"], endpoint=endpoint, token=False, cache_dir=root / "snapshot-cache"))
                    assert json.loads((snapshot / "config.json").read_text())["model_type"] == "gpt2"
                    assert hashlib.sha256((snapshot / "model.safetensors").read_bytes()).hexdigest() == digest
                    dataset = Path(hf_hub_download("lhoestq/demo1", "data/train.csv", repo_type="dataset", endpoint=endpoint, token=False, cache_dir=cache))
                    rows = list(csv.reader(io.StringIO(dataset.read_text())))
                    assert len(rows) > 1 and len(rows[0]) > 1
                    assert all(len(row) == len(rows[0]) for row in rows)
                    dataset_head = client.head(f"{endpoint}/datasets/lhoestq/demo1/resolve/main/data/train.csv").raise_for_status()
                    data = dataset.read_bytes()
                    assert int(dataset_head.headers["content-length"]) == len(data)
                    assert hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest() == dataset_head.headers["etag"].strip('"')

                    cli_env = {key: value for key, value in os.environ.items() if key not in {"HF_TOKEN", "HUGGING_FACE_HUB_TOKEN"}}
                    cli = subprocess.run(["hf", "download", repo, "config.json", "--revision", revision, "--local-dir", str(root / "cli-model")], env=cli_env, capture_output=True, text=True, timeout=120)
                    assert cli.returncode == 0, cli.stderr
                    assert json.loads((root / "cli-model" / "config.json").read_text())["model_type"] == "gpt2"

                    # Repository tree pagination URLs must also remain local.
                    tree = client.get(f"{endpoint}/api/models/{repo}/tree/{revision}?limit=1").raise_for_status()
                    assert tree.headers.get("link", "").startswith(f"<{endpoint}/")
                    next_url = tree.headers["link"].split("<", 1)[1].split(">", 1)[0]
                    client.get(next_url).raise_for_status()
                    assert not (root / "server-cache").exists(), "HF wrote to the service disk cache"
                    assert "HEAD" in requests and "GET" in requests
                    print(json.dumps({"upstream_transport": "trusted local relay" if relay else "direct HF", "model": repo, "revision": revision, "bytes": len(payload), "sha256": digest, "sdk_requests": len(requests), "checks": ["SDK download", "hf CLI", "snapshot", "dataset Git blob digest", "HEAD metadata", "Range", "resume", "pagination", "no SDK bypass", "no server cache"]}, indent=2))
            except BaseException:
                log.flush()
                log.seek(0)
                print(log.read()[-8000:], file=sys.stderr)
                raise
            finally:
                server.terminate()
                try:
                    server.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    server.kill()
                    server.wait()


if __name__ == "__main__":
    main()
