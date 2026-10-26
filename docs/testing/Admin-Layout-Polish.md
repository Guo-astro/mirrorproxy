# Administration layout refinements

Administration retains native Ant Design Pro layout, cards, statistics and table
controls. The homepage appearance is preserved apart from the requested footer
placement.

- Header actions are separate native buttons; sign-out is available through the
  account dropdown and retains its confirmation step. The same dropdown's
  Change password action opens `/admin/security` without ending the session.
- Mirror availability uses Pro statistic cards and Ant status tags. The source
  table expands with the page instead of introducing an internal scroll area.
- IPv4/IPv6 offline database status uses Pro cards and Ant status badges. The IP
  lookup and effective-rules cards align at the top. Rule inputs and submission
  share one responsive grid.
- HTTPS guide text and external-link icons have a 6px gap.
- Administration uses Ant's Checkbox component token for a 20px box, with a
  32px minimum label row. Checked/disabled/focus styles remain native. The
  consumer theme does not receive this override.
- Footers reach the viewport bottom on short pages and follow the content on
  long pages; they do not overlay the page. This includes the administration
  login screen and homepage.

Additional settings refinements use native information alerts for registration
and public-mode hints, a Pro user table with expandable management controls, and
a server-paginated Pro audit table. Form actions use content-sized buttons;
billing group fields and creation share a desktop row. Security forms share a
24px card rhythm and explicit field/action gaps. Advanced help text is 14px,
ACME status uses Descriptions, and menu icons use the existing Lucide dependency.
The header home action opens a new tab with opener isolation.

Billing-group quota hints preserve null as unlimited and zero as zero allowance.
Download defaults match the requested 10 GiB/file, 20 GiB/day, 16 instance / 5
user concurrency, 300 requests/minute, 1800 seconds and 100 MiB/s bandwidth.
Downloads remain disabled by default; explicit saved policies are unchanged.

Validation: 50 Vitest tests, TypeScript and production build, and 45 Chromium
scenarios. The dedicated scenario covers independent header actions, dropdown
sign-out/confirmation, a 40-row table without internal vertical scrolling,
IPv4/IPv6 badges, IP card/field geometry, whole-label checkbox toggling and
short/long-page footer positions. Responsive checks cover light/dark themes and
1920/768/375px, with short-page footer checks at 1440px. Native server/browser
smoke checks cover HTTP/HTTPS sessions, CSRF and invitation registration. New
scenarios inspect six settings pages in both themes at 1920/768/375px and exercise
audit pagination against distinct server pages. Rust formatting, Clippy and 435
workspace tests pass (1 ignored), including defaults/example agreement and
preservation of explicit older policies.

These are local changes; commit, push and deployment remain separate actions.
