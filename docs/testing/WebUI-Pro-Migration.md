# Ant Design Pro Web UI migration

The UI uses React 19 / Umi 4 / Ant Design 6 with native Ant Design Pro components.
Umi's core package supplies routing and builds; the unused Max plugin bundle was
removed because it installed legacy lint, DVA and react-intl stacks. Existing
application providers supply Ant locale/theme and cookie/CSRF-aware requests.
No permission checks rely on the removed, unused Max access model: API role and
CSRF checks and the application's session handling are preserved.
Template baseline: ant-design/ant-design-pro master commit
`52506486ea134486937043c5fb677d89bbcd07f2` (MIT).
Only the application architecture and dependencies are adopted; template mock
accounts, demo pages, AI chat, and deployment infrastructure are excluded.
ProComponents 3.1.14-2 is pinned because the 2.x default npm tag supports antd 5,
whereas the official Pro 6 template uses the antd 6-compatible 3.x line.

## Feature parity checklist

- Public: configured branding/SEO/footer, source discovery/search/categories,
  capabilities and health, package/system source commands, GitHub link rewrite,
  registry/image/Compose/Dockerfile rewriting, personal routing origin, CLI and
  Agent guidance, clipboard fallback, language/theme persistence.
- Account: password login/registration, email code/magic link, registration
  invitation fragments, OAuth providers and linking, password change, routing
  rotation, usage/quota/group allowance, download token lifecycle, logout.
- Administration: password/Passkey login, overview, health checks and upstreams,
  regional traffic, IP lookup/access rules, registration/site/access/quota policy,
  users/groups/local accounts/invitations, download policy/grants/domain blocks/
  usage/active transfers, identity providers, SMTP, administrator accounts,
  credentials/passkeys/sessions, ACME/TLS/DNS settings, upstreams/additional OS
  sources/outbound proxy/cache/GeoIP/alerts/rate limits, and audit pagination.
- Security: existing cookie sessions and audience-specific CSRF, role restrictions,
  secret redaction, negative authentication/CSRF cases, confirmation of destructive
  actions, private-page robots metadata.
- Delivery: `npm ci`, unit tests, Umi build to `web/dist`, Playwright, static
  embedding, native build, Docker build context, and administration smoke.

## Development and acceptance

`npm run dev` starts Umi on port 4173. Set `MIRRORPROXY_DEV_API` to a local Rust
server origin to forward API calls; production remains a static Rust deployment.
The build never requires a Node runtime on the installed server.

Migrate pages locally and accept the complete UI before production cutover. No
public API or database migration is introduced. Route modules live under
`web/src/pages`, the request boundary under `services`, and common controls under
`components`. Real API writes still use the original CSRF cookie/header protocol.

The public Agent panel offers one-command installation plus existing-client
setup. Same-origin installers are embedded with the UI and reject a client
without setup before replacement. A matching stable client must still be
published separately; deploying a dev server does not update stable assets.

Required validation: unit tests, type check/production build, full Playwright in
both themes and languages, deep-route reload/back/forward, native administrator
smoke, dependency audit, and applicable Rust gates. Verify JS/CSS chunks from the
embedded server, including cache headers, missing assets, and metadata escaping.

## Dependency review

The lockfile pins the official Umi/Ant Design baseline. Focused npm overrides
remove the newly introduced high/critical advisories without disabling CI:
ProComponents is unified on 3.x; path-to-regexp is patched within its 1.x/8.x
branches (Express 0.x remains unchanged); Axios stays on its patched 0.x line;
Immer and node-fetch use compatible patched APIs; unused alternative bundlers'
PostCSS and Vite copies receive same-major fixes. No `npm audit --force` is used.

The complete dependency tree now reports **0 vulnerabilities**, including low and
moderate severity. CI now runs `npm audit --audit-level=low`; there are no ignored
advisories or audit exclusions.

The remaining Babel, URI decoder, selector parser and esbuild advisories are
fixed with scoped version overrides. React Router 6 instances move to patched
7.18.4; legacy DVA 4.x instances are left on their own API line. Umi's Webpack
aliases use exact router package roots so v7's `/dom` export resolves correctly;
the former `/server` entry resolves to v7's root export. Real deep-route,
back/forward, authentication and configuration browser tests verify this change.

The UI never uses Node's crypto API. Both Webpack and optional Mako pulled the
unpatched Elliptic signing implementation through crypto-browserify. The private
`web/build/no-node-crypto` build adapter removes that implementation and fails
explicitly if imported. It does not implement cryptography, return placeholder
signatures, or replace browser Web Crypto/WebAuthn. All other fallback modules
remain unchanged. A dependency contract rejects the old signing packages, and a
post-build check fails if the guard appears in emitted browser JavaScript. This
proves no current UI feature depends on the removed Node crypto implementation.
Docker copies the adapter before `npm ci` so locked clean installs also work.

## Verified local acceptance — 2026-10-26

- `npm test`: 46 tests passed across 9 files. JSDOM reports known CSS parsing and
  pseudo-element limitations; computed styles and interactions are verified in
  real Chromium rather than inferred from JSDOM.
- `npm run build`: TypeScript and Umi production build passed.
- `npm run test:e2e`: 34 passed, with zero uncaught page errors in every scenario;
  covers both languages/themes, 320/375/768/1440 widths, mobile navigation,
  authentication, configuration, downloads, charts and deep-route navigation.
- `cargo fmt --all -- --check`, workspace/all-target Clippy with locked
  dependencies and denied warnings, and `cargo test --workspace --locked` passed:
  429 tests passed, 1 existing public-network test ignored.
- Native server build plus `smoke-admin-api.sh`, `smoke-admin-browser.mjs` and
  `smoke-source-details.py` passed. Every emitted JS/CSS chunk matched the bytes
  embedded in the running server. Go module proxy fallback returned the expected
  payload; real HTTP/HTTPS browser session and CSRF checks passed.
- `docker build --target web-build` passed with the locked dependencies. This
  validates the changed frontend delivery stage; a full runtime image and the
  public ecosystem client matrix were not rebuilt/run for this UI-only change.
- `npm audit --audit-level=low` passed with 0 vulnerabilities; no advisories are
  ignored. No remote CI/CodeQL run was triggered; changes are uncommitted on
  the feature branch and no production service was modified.

## Security remediation acceptance — 2026-10-26

After the remaining dependency fixes, the complete npm audit reports 0
vulnerabilities. Unit tests (46), production build, browser tests (34), Docker
web-build, native embedded-chunk digest checks, real HTTP/HTTPS browser
session/CSRF smoke, and local source protocol smoke all passed again.
The CI threshold is now low; no advisory is filtered out.

The separate Rust audit exited successfully with the repository's documented
RUSTSEC-2023-0071 and RUSTSEC-2026-0387 public-key-verification exceptions. It also
reports the existing Tokio 1.52.3 `RUSTSEC-2026-0386` I/O-safety warning. The npm
zero-vulnerability result does not imply a warning-free Rust dependency audit.
No remote CodeQL workflow was triggered because this feature branch remains
uncommitted and unpushed; its exact-commit result is still a delivery gate.

## Native Pro component alignment

The administrator console uses the official Pro defaults: a full-width mixed
header/sider layout, native icon menus, fluid PageContainer with breadcrumbs and
page actions, and built-in account/header actions. It no longer wraps ProLayout
in a separate legacy topbar or duplicated operations heading. Menu destinations
are semantic links and preserve browser navigation behavior.

Shared settings and form cards use ProCard headers, normal descriptions and
extra actions, including search controls. Overview metrics use StatisticCard;
configuration controls use the same Ant Design theme and ProForm context. Old
CSS overrides for Pro navigation, card padding and statistic typography were
removed. Desktop controls use Ant Design defaults; mobile buttons retain a
44-pixel minimum touch height. Public, account and administration controls share
the centralized theme, request and confirmation boundaries.

Browser acceptance additionally checks the native header/sider/page/card
structure and captures both themes at 320, 375, 414, 768 and 1440 pixels, plus a
Chinese settings-page screenshot. The build remains local until deployment is
explicitly authorized.

Native-Pro alignment acceptance: 46 unit tests and 35 Chromium scenarios passed;
production build and Docker web-build passed. The new visual regression captures
Pro's default header/sider, breadcrumbs, card headers and statistic typography in
both themes and five viewport widths. Unconfigured warnings and existing access
controls remain active. Real browser responsive checks wait for Pro's responsive
layout after viewport changes rather than reading geometry during the transition.
