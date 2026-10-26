#!/usr/bin/env python3
"""Exercise local accounts through a running server with SMTP disabled."""
import hashlib
import http.cookiejar
import json
from pathlib import Path
import re
import sys
import urllib.error
import urllib.parse
import urllib.request

base = sys.argv[1]
password = "x"  # Ordinary user passwords only require nonempty content.


def client():
    jar = http.cookiejar.CookieJar()
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), urllib.request.HTTPCookieProcessor(jar))
    return opener, jar


def request(who, path, body=None, method=None, expected=200, csrf=True, origin=None):
    opener, jar = who
    headers = {"Content-Type": "application/json", "Origin": origin or base}
    if csrf:
        for cookie in jar:
            if "csrf" in cookie.name:
                headers["X-MirrorProxy-CSRF"] = cookie.value
    req = urllib.request.Request(base + path, data=json.dumps(body).encode() if body is not None else None, headers=headers, method=method)
    try:
        response = opener.open(req)
    except urllib.error.HTTPError as error:
        response = error
    payload = response.read()
    assert response.status == expected, (path, response.status, payload[:300])
    if not payload:
        return None
    try:
        return json.loads(payload)
    except json.JSONDecodeError:
        return payload.decode()


# Validate every production chunk, including Umi's lazy route and vendor chunks.
html = request(client(), "/admin/downloads")
assert re.search(r'<script[^>]+src="([^"]+)"', html)
dist = Path(__file__).resolve().parent.parent / "web" / "dist"
assets = sorted(path for path in dist.rglob("*") if path.suffix in (".js", ".css"))
assert assets and any(path.name.endswith(".async.js") for path in assets)
bundles = []
for expected_asset in assets:
    asset = "/" + expected_asset.relative_to(dist).as_posix()
    bundle = request(client(), asset)
    assert hashlib.sha256(bundle.encode()).digest() == hashlib.sha256(expected_asset.read_bytes()).digest(), asset
    if expected_asset.suffix == ".js":
        bundles.append(bundle)
assert any("Manual troubleshooting and recovery steps" in bundle for bundle in bundles)
request(client(), "/missing-ui-chunk.123456.js", expected=404)
for installer in ("install.sh", "install.ps1"):
    script = request(client(), "/" + installer)
    assert script == (dist.parent.parent / "scripts" / installer).read_text()
for icon in ("favicon.svg", "favicon.ico", "favicon-32x32.png", "apple-touch-icon.png", "icon-192.png", "icon-512.png", "icon-512-maskable.png", "site.webmanifest"):
    opener, _ = client()
    with opener.open(base + "/" + icon) as response:
        assert response.headers["Cache-Control"] == "no-cache", icon
        assert response.read() == (dist / icon).read_bytes(), icon
assert 'rel="manifest"' in html and 'href="/site.webmanifest"' in html
assert 'href="/apple-touch-icon.png"' in html


admin = client()
request(admin, "/admin/api/auth/login", {"username": "admin", "password": "MirrorProxy-Smoke-Password-1!"})
request(client(), "/admin/api/connection-info", expected=401)
connection = request(admin, "/admin/api/connection-info")
assert connection == {"peer_ip": "127.0.0.1", "trusted_proxy": True, "forwarded_https": False, "effective_https": False}
billing = {"group_id": None, "quota_mode": "custom", "monthly_gb": 3}
new_account = {"username": "smoke-friend", "password": password, "display_name": "Smoke Friend", "billing": billing}
request(client(), "/admin/api/local-users", new_account, expected=401)
csrf_error = request(admin, "/admin/api/local-users", new_account, expected=403, csrf=False)
assert csrf_error["code"] == "csrf_invalid"
user = request(admin, "/admin/api/local-users", new_account, expected=201)
assert user["username"] == "smoke-friend" and "password" not in json.dumps(user)
request(client(), "/admin/api/users/summary", expected=401)
summary = request(admin, "/admin/api/users/summary?page=1&per_page=10&search=smoke-friend")
assert summary["total"] == 1 and len(summary["items"]) == 1
assert summary["items"][0]["id"] == user["id"]
assert summary["items"][0]["billing"]["user_monthly_limit_bytes"] == 3 * 1024**3
assert summary["items"][0]["response_bytes"] == 0
assert "password" not in json.dumps(summary)
request(admin, "/admin/api/local-users", new_account, expected=409)
login = {"username": "smoke-friend", "password": password}
friend = client()
request(friend, "/api/auth/password/login", login, expected=403, origin="https://attacker.example")
request(friend, "/api/auth/password/login", {**login, "password": "wrong"}, expected=401)
request(friend, "/api/auth/password/login", login)
assert request(friend, "/api/account/profile")["user"]["id"] == user["id"]
assert request(friend, "/api/account/usage")["quota"]["limit_bytes"] == 3 * 1024**3
change = {"current_password": password, "new_password": password + "-changed"}
request(friend, "/api/account/password", change, expected=403, csrf=False)
request(friend, "/api/account/password", change, expected=204)
request(friend, "/api/account/profile", expected=401)
request(friend, "/api/auth/password/login", {**login, "password": password + "-changed"})

invite = request(admin, "/admin/api/registration-invitations", {"billing": billing, "expires_in_hours": 72}, expected=201)
url = urllib.parse.urlsplit(invite["url"])
assert url.path == "/login" and not url.query
assert url.scheme + "://" + url.netloc == base
raw_token = urllib.parse.parse_qs(url.fragment)["register"][0]
registration = {"token": raw_token, "username": "smoke-friend", "password": password, "display_name": "Invited Friend"}
new_friend = client()
request(new_friend, "/api/auth/register", registration, expected=409)
request(new_friend, "/api/auth/register", {**registration, "billing": {"quota_mode": "unlimited"}}, expected=422)
request(new_friend, "/api/auth/register", {**registration, "username": "smoke-invited"})
assert request(new_friend, "/api/account/profile")["user"]["username"] == "smoke-invited"
assert request(new_friend, "/api/account/usage")["quota"]["limit_bytes"] == 3 * 1024**3
request(client(), "/api/auth/register", {**registration, "username": "smoke-replay"}, expected=400)
items = request(admin, "/admin/api/registration-invitations")
assert items[0]["status"] == "accepted" and raw_token not in json.dumps(items)
revoked = request(admin, "/admin/api/registration-invitations", {"billing": billing, "expires_in_hours": 1}, expected=201)
request(admin, f"/admin/api/registration-invitations/{revoked['id']}", method="DELETE", expected=204)
revoked_token = urllib.parse.parse_qs(urllib.parse.urlsplit(revoked["url"]).fragment)["register"][0]
request(client(), "/api/auth/register", {**registration, "token": revoked_token, "username": "smoke-revoked"}, expected=400)
print("embedded console digest, authenticated connection diagnostics, local account login, password change, CSRF, invitation registration/replay/revocation, and quotas passed")

# Exercise download setup through real administrator/user sessions and CSRF.
report = request(admin, "/admin/api/download")
assert report["config"]["enabled"] is False
request(friend, "/admin/api/download", expected=401)
assert request(friend, "/api/account/download")["allowed"] is False
request(friend, "/api/account/download/tokens", {"name": "Smoke", "days": 1}, expected=403)
download_config = {**report["config"], "enabled": True}
request(admin, "/admin/api/download/config", download_config, method="PUT", csrf=False, expected=403)
request(admin, "/admin/api/download/config", download_config, method="PUT")
request(admin, "/admin/api/download/permissions", {"kind": "user", "subject_id": user["id"], "policy": "allow"}, method="PUT", expected=204)
assert request(friend, "/api/account/download")["allowed"] is True
scoped = request(friend, "/api/account/download/tokens", {"name": "Smoke", "days": 1}, expected=201)
assert scoped["scope"] == "download:fetch"
assert scoped["token"] not in json.dumps(request(friend, "/api/account/download"))
request(admin, "/admin/api/download/blocks", {"domain": "example.org", "subdomains": True, "reason": "smoke", "blocked": True}, method="PUT", expected=204)


def token_request(path, expected):
    req = urllib.request.Request(base + path, headers={"Authorization": "Bearer " + scoped["token"]})
    try:
        response = client()[0].open(req)
    except urllib.error.HTTPError as error:
        response = error
    assert response.status == expected, (path.split("?")[0], response.status)
    response.read()


token_request("/admin/api/download", 401)
token_request("/api/download?" + urllib.parse.urlencode({"url": "https://example.org/artifact"}), 403)
token_request("/api/download?" + urllib.parse.urlencode({"url": "https://127.0.0.1/private"}), 400)
report = request(admin, "/admin/api/download")
assert report["usage"]["totals"][0]["denied"] == 1
request(friend, f"/api/account/download/tokens/{scoped['id']}", method="DELETE", expected=204)
token_request("/api/download?" + urllib.parse.urlencode({"url": "https://example.org/artifact"}), 403)
print("download admin/user sessions, CSRF, opt-in grants, token secrecy/revocation, SSRF refusal, domain block and denial statistics passed")
