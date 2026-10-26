# Toolchain deprecations, Vitest advisories and Windows fixture isolation

The reported CI run used commit 4e05ba0. Its Windows client download test failed
when a second download's refusal was followed by a missing original output file.
Test fixture directories used process ID and wall-clock nanoseconds only. Coarse
Windows clock ticks could reuse a path across parallel tests, whose Drop handlers
then removed another fixture's files. A process-local atomic sequence now forms
part of the directory name and create_dir rejects unexpected reuse. A deterministic
test allocates 32 fixtures with an identical timestamp and checks independent
lifetimes. No download overwrite/symlink/credential checks have been weakened.

Two GitHub Dependabot alerts (#5/#6) affect @vitest/mocker 5.0.0:
GHSA-xp26-3454-mjgp and GHSA-qxg6-89mg-rqv8. Vitest and its mocker now resolve to
5.0.5, beyond the 5.0.3 fixes. Application behavior and test coverage remain intact.
GitHub alerts track the default main branch; a repair on dev alone does not close
them and no alert dismissal or default-branch merge is performed by this change.

Umi 4.7.28 core replaces the Max dependency bundle. Routing and builds remain Umi;
layout/forms/tables remain native Ant Design Pro and Ant Design. Existing providers
already handle locale, theme and request initialization, and components already
handle their own sessions. Unused Max initial-state/access/model boilerplate,
lint, DVA and legacy react-intl dependency trees are removed. Application session,
API authorization and CSRF checks are preserved. No second framework is added.

Targeted toolchain upgrades:

- tsx 4.20.6 replaces the deprecated esbuild-kit loaders in Umi run.
- SVGO 3.3.5 removes stable and contains current SVG parser/sanitizer fixes.
- test-exclude 8 and rimraf 6 replace the glob 7/10 and inflight chains.

Four dedicated tests exercise SVGR component generation, coverage exclusion,
async rimraf removal scope and the actual tsx runner. CI executes them alongside
Vitest. A dependency contract rejects the old Max bundle, vulnerable mocker patch
versions and newly introduced deprecated packages.

Clean npm installation reports 3 deprecated packages instead of 20:
@stagewise/toolbar 0.6.2, @floating-ui/react-dom-interactions 0.3.1, and
node-domexception 1.0.0. These come from Umi's optional development-toolbar,
click-to-source and server-fetch tooling. Their upstream replacements are not
API-compatible drop-ins. They remain visible and installed rather than hidden
with empty stubs, ignored logs or removed package metadata.

Local validation: 50 Vitest tests, 45 Chromium scenarios, 4 toolchain API tests,
4 build-cache tests, 436 Rust tests (1 ignored), formatting/Clippy, TypeScript and
production build. The dependency tree is valid (npm ls --all) and all-severity
npm audit reports zero vulnerabilities. Real server API/HTTP/HTTPS browser smoke
and clean Docker web-build pass. The standard local musl build took 71s with
existing Cargo artifacts; the immediate repeat took 1s and preserved both binary
hashes. Remote Windows validation must be
checked on the new commit rather than inferred from a Linux test result.
