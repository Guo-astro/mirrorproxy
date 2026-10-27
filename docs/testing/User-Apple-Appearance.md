# User-facing Apple appearance

The reference is the workspace's `DESIGN.md` (Apple-design-analysis). Only `/`,
`/login` (including invitation registration) and `/account` use this appearance.
Administration stays on native Ant Design Pro. No new dependencies or fonts are
downloaded; the SF Pro system stack falls back to installed platform UI fonts.

The user appearance lives in `web/src/components/consumer-theme.tsx` and
`web/src/consumer.css`. A nested ConfigProvider scopes colors, typography,
controls and feedback to user pages. CSS selectors target `.consumer-ui` and
`.consumer-modal`; they never target administration. Existing cookie, CSRF,
registration, invitation and download-token behaviors are preserved.

The public and account headers use the same site icon, title and compact
navigation controls; the brand links to the public homepage. Authentication
uses one surface with sign-in and registration tabs. Sign-in combines the
password and configured third-party methods and offers an email-method switch
when enabled. Registration follows the server policy: an invitation fragment
opens the password registration form, while open/domain registration uses only
enabled email or third-party methods. Disabled registration does not expose a
creation form. The registry input editor uses the same scoped light/dark tokens.

The portal restores the original functional order: hero, quick converters,
container registry workbench, client/Agent installation, then source catalog.
Apple presentation uses restrained white/parchment surfaces, Action Blue,
system typography, rounded utility cards and generous spacing. Dark mode has
explicit scoped tokens; dark-to-light round trips must restore the original
computed surfaces, foregrounds and borders.

The detail pass restores compact source cards with a divided metadata row,
single-column platform installation instructions and one modal body scrollbar.
Registry, CLI installation, Agent setup and source selection share the same
1360px maximum width and responsive 24px outer gutters. The homepage header,
hero, quick converters, connection notice and footer follow the same grid;
full-width background bands remain full width.
Command blocks use safe React syntax tokens, an independent copy toolbar and
visible copy feedback. Shell and PowerShell text is copied without alteration.
Windows uses a single inline command without changing execution policy.

One-command Agent installation uses the instance's embedded `/install.sh` or
`/install.ps1`, verifies the published archive checksum and checks for `setup`
before replacing an existing client. A stable release containing `setup` is
still required; release publication is a separate task. Existing-client setup
remains available.

Validated locally:

- 50 unit tests; TypeScript and production build.
- 48 Chromium scenarios with no uncaught page errors, including user/admin
  isolation, language switching, login/invitation/account behavior and source
  functionality. New visuals cover 320/375/414/768/1440 widths and theme round trips.
- Native Rust embedding: every JS/CSS chunk matched; real administration and
  HTTP/HTTPS browser session/CSRF/invitation smoke passed. Both embedded installer
  routes matched the source scripts exactly.
- Rust formatting, Clippy and 433 workspace tests passed (1 ignored).
- Unix and offline PowerShell installer fixtures cover supported setup,
  unsupported old clients and bad checksums. Native Windows policy enforcement
  has not been tested on a Windows host.
- Docker web-build and full-severity npm audit passed; 0 vulnerabilities.
- The administrator page, shared Pro controls, global stylesheet and global
  provider hashes were unchanged during this user-only styling task.

No commit, push or production deployment is part of this appearance trial.

The supplied MirrorProxy icon set is served from `web/public`: ICO, PNG browser
and touch icons plus the web manifest's standard/maskable sizes. `/favicon.svg`
remains a self-contained compatibility wrapper around the supplied 192px PNG.
Custom site icons still override browser/touch icons. Fixed-name brand assets
require cache revalidation; hashed application chunks retain immutable caching.
The homepage brand uses the new icon and its account/language/theme controls
share 44px targets, rounded corners, restrained hover and visible keyboard focus.
Email method switching and third-party sign-in use scoped light/dark hover and
keyboard-focus states. The email icon is centered with its heading text.
Installation and converter title icons are centered against the heading text,
independently of the descriptions below. HTTPS/release links have an explicit
6px icon gap. Browser geometry checks cover both languages and themes at
1440/470/320px, including wrapping titles and non-shrinking icons.
