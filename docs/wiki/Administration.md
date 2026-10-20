# Administration

Open **`https://your-host/admin`**. The first account is `admin`; use the configured initial password or the random password in the first startup log.

> **Use HTTPS for sign-in.** v1.4.4 also supports HTTP password login with a persistent security warning. For older login failures, check [Troubleshooting](Troubleshooting); do not delete the database.
> The password environment variable creates the first administrator only. Change an existing password in the console.

## Common tasks

| Task | Console page |
| --- | --- |
| Check service health, requests, and quotas | Overview |
| Check an upstream source | Mirror health |
| Change upstreams, adapters, outbound proxy, or cache | Advanced |
| Change admin passwords, passkeys, or sessions | Administrators & security |
| Manage users, quotas, billing groups, and site identity | Access & quotas; Users & groups |
| Set up mail, invitations, and identity providers | Email & invitations; Identity providers |
| Inspect regional traffic, IP rules, and actions | Regional traffic; IP access control; Audit log |

## Administrators and login

- `super_admin` changes global policy, administrators, and sensitive settings; `admin` handles daily operations and observation. The server enforces privileged actions.
- Five consecutive failures lock an account for 15 minutes; restarts do not clear it. Per-account and real-source-IP throttling and auditing also apply.
- Forgot a password? Use [local recovery](Troubleshooting).
- Passkeys need a supported secure browser context. Before enforcing them, verify RP hostname/origin, the break-glass account, and sufficient credentials for required accounts.

## Users, quotas, and alerts

Enable registration or invitations as needed, then configure billing groups and user quotas. Monthly limits can apply globally, per group, and per user;
the selected policy controls overages. Delivered bytes and billable bytes can differ when bidirectional accounting includes uploads.

Alerts support webhooks, mail, or both, with separate cooldowns. Mail alerts and invitations require SMTP first.
Access & quotas configures site title, description, keywords, icon, and left footer text; the project link and version remain fixed.

## Advanced: protect settings and data

A stable `MIRRORPROXY_MASTER_KEY` (32 bytes, hex or base64url) encrypts persisted runtime settings, SMTP/provider credentials,
mail-queue content, and ACME DNS secrets. **Back it up separately: it is not stored in SQLite, and losing it prevents recovery of the encrypted values.**
The console does not return these secrets in plaintext; fields are write-only or report that a value is configured.

See [Advanced deployment](Advanced-Deployment) for backups, restore, systemd, private management, and `/metrics`,
or [GeoIP and IP rules](GeoIP-and-IP-Access-Control) for regional access control.

[简体中文](Administration-zh-CN)
