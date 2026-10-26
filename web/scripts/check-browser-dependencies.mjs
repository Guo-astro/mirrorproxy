import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { readFileSync } from 'node:fs'
const require = createRequire(import.meta.url)
const lock = JSON.parse(readFileSync(new URL('../package-lock.json', import.meta.url), 'utf8'))
for (const [path, pkg] of Object.entries(lock.packages)) {
  if (path.endsWith('/node_modules/react-helmet-async') || path === 'node_modules/react-helmet-async') assert.equal(pkg.version, '3.0.0', 'Helmet must support React 19')
  if (path.endsWith('/node_modules/use-sync-external-store') || path === 'node_modules/use-sync-external-store') assert.equal(pkg.version, '1.7.0', 'External-store shim must support React 19')
}
assert.equal(lock.packages['node_modules/@umijs/bundler-utoopack/node_modules/less-loader'].version, '12.3.3', 'Utoo requires less-loader 12')
assert(!Object.keys(lock.packages).some(path => /node_modules\/(elliptic|browserify-sign|create-ecdh)$/.test(path)), 'Unused legacy signing dependencies must not return')
const guard = require.resolve('../build/no-node-crypto/index.cjs')
assert.equal(require.resolve('crypto-browserify'), guard)
const fromWebpack = createRequire(require.resolve('@umijs/bundler-webpack/package.json'))
const fallbacks = fromWebpack('node-libs-browser')
assert(fallbacks.process && fallbacks.buffer && fallbacks.stream && fallbacks.util)
assert.equal(fallbacks.crypto, guard)
// Mako consumes the polyfill file directly rather than the CommonJS map.
const fromMako = createRequire(require.resolve('@umijs/mako/package.json'))
const fromMakoFallback = createRequire(fromMako.resolve('node-libs-browser-okam/package.json'))
assert.equal(fromMakoFallback.resolve('crypto-browserify'), guard)
assert.throws(() => fromMakoFallback('./polyfill/crypto.js'), /does not provide a Node.js crypto/)
assert.throws(() => require(guard), /does not provide a Node.js crypto/)
console.log('Umi crypto fallback references exclude legacy signing code; Webpack utilities remain available.')
