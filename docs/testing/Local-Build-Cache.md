# Local build cache and dependency warnings

`./build.sh` builds locked release artifacts with Node.js 24+, npm, Rust stable
and the selected Rust target (default x86_64 musl). It does not deploy or publish.

The local cache lives under `${CARGO_TARGET_DIR:-target}/mirrorproxy-build-cache`.
It is generated and must not be committed. An unrecognized install is rebuilt
with `npm ci --prefer-offline --include=dev --no-fund`, including the normal audit.
Dependency keys include package/lock/local dependency inputs, Node/npm/platform
and npm installation options. npm's hidden lock and installed package directory
state must agree. Missing tools/packages or a changed hidden lock invalidate it.

Frontend keys cover source, config, public assets, scripts, local packages and
build environment settings. Generated/dependency/test output directories are
excluded. Every dist file is hashed: changed, missing or extra output invalidates
reuse. Native timestamp keys cover Rust inputs/toolchain, embedded Web/installers,
Git metadata and compiler environment; binary digests must also match. Cargo
still runs with `--locked` every time and owns compilation invalidation. Both
binaries are built together. Unchanged native builds retain their build time.
`BUILD_TIME` explicitly overrides this behavior. GeoIP inputs retain checksum
verification. Logs show each stage's duration.

Use `./build.sh --force` to reinstall locked dependencies, rebuild Web and refresh
the native build time. It does not run cargo clean or delete the Cargo cache.
Manual source builds using npm/cargo remain supported and do not depend on these
cache records. Do not edit dependency files under node_modules manually; use the
lockfile workflow or --force. Build caching does not replace a current online
vulnerability audit; run npm audit separately (CI retains its audit gate).

Peer dependency repairs are scoped to Umi's renderer (react-helmet-async 3.0.0),
its Utoo loader (less-loader 12.3.3), and the React 19 compatible external-store
shim (1.7.0). No force/legacy-peer-deps or log-level suppression is used. npm 11's
allowScripts permits only the two locked esbuild binary installers, explicitly
denying nonessential core-js, core-js-pure and es5-ext postinstall notices.

A clean install still reports 20 deprecated transitive toolchain packages,
including Umi's legacy lint/router/formatting dependencies. Removing or renaming
these through broad overrides can break their callers; they remain visible.
`npm ls --all` also reports Umi's unused react-redux 8 optional React 18 type
peers against this React 19 project. It is not a fully clean dependency-tree
result. Targeted runtime dependency checks, TypeScript, production build and
browser acceptance pass; a framework-level dependency refresh is separate work.

Validation includes four Node tests for source/output invalidation, missing
packages, changed locks, compiler/binary changes, forced builds and two identical
builds preserving binary bytes; 50 frontend tests and 45 Chromium scenarios,
including cache-panel geometry. Browser controls and metadata are exercised after
the React peer upgrades. npm audit reports 0 vulnerabilities. Actual musl release
binaries and native HTTP/HTTPS API/browser smoke checks are also exercised.

On the local validation machine, the final cache-populating build took 76 seconds
(14s locked install, 16s Web, 45s native compilation with existing Cargo artifacts).
The immediately repeated build took 1 second; Cargo reported 0.24s and both SHA-256
digests stayed identical. This is a warm-build measurement, not a clean-machine
Rust build benchmark. Docker's clean web-build also passed.
