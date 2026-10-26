# No Node crypto in the browser

The UI does not use Node's crypto API: authentication uses browser WebAuthn and
native Web Crypto. Umi's Webpack and optional Mako fallback maps nevertheless
pull crypto-browserify and Elliptic. Elliptic has no patched version for
GHSA-848j-6mx2-7j84.

This private build adapter removes that entire unused implementation and its
signing dependencies. It is not a replacement cryptography implementation and
never returns a signature, key or random bytes. If a future dependency imports
Node crypto, it fails explicitly rather than silently using a weaker algorithm.
All non-crypto fallback mappings remain untouched.

The dependency contract test verifies both Umi fallback maps resolve this guard
and rejects Elliptic in the lockfile. The production build also rejects this
guard in emitted JavaScript, proving the browser does not import Node crypto.
Real browser authentication, Passkey, chart and navigation tests remain required.
