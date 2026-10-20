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
password = "MirrorProxy-Local-Smoke-Passphrase!"


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


# Validate that the running binary embeds the console built for this checkout.
html = request(client(), "/admin")
asset = re.search(r'<script[^>]+src="([^"]+)"', html).group(1)
bundle = request(client(), asset)
assert "Manual troubleshooting and recovery steps" in bundle
expected_asset = Path(__file__).resolve().parent.parent / "web" / "dist" / asset.lstrip("/")
assert hashlib.sha256(bundle.encode()).digest() == hashlib.sha256(expected_asset.read_bytes()).digest()

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
