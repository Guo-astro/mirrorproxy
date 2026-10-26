import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
const dist = new URL('../dist/', import.meta.url)
const files = readdirSync(dist, { recursive: true }).filter(path => path.endsWith('.js'))
assert(files.length > 0)
for (const path of files) assert(!readFileSync(new URL(path, dist), 'utf8').includes('does not provide a Node.js crypto browser polyfill'), `Unexpected Node crypto usage in ${path}`)
console.log('No browser chunk imports the unsupported Node crypto fallback.')
