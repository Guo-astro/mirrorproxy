import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

const stylesheet = readFileSync(new URL('../src/styles.css', import.meta.url), 'utf8')
const dockerfile = readFileSync(new URL('../../Dockerfile', import.meta.url), 'utf8')
const config = readFileSync(new URL('../config/config.ts', import.meta.url), 'utf8')
const runtime = readFileSync(new URL('../src/app.tsx', import.meta.url), 'utf8')
assert.match(stylesheet, /@import\s+["']\.\/tokens\.css["']/)
assert.match(dockerfile, /COPY\s+web\/\s+\.\//)
assert.match(dockerfile, /COPY\s+web\/build\/no-node-crypto\//)
assert.match(config, /outputPath:\s*'dist'/)
assert.match(runtime, /initializeRequests\(\)/)
console.log('Docker includes Umi configuration, theme tokens, and the CSRF-aware runtime.')
