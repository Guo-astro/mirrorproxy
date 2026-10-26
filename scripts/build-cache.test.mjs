import assert from 'node:assert/strict'
import { test } from 'node:test'
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync, copyFileSync, chmodSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { execFileSync } from 'node:child_process'
import { createCache } from './build-cache.mjs'

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'mirrorproxy-build-'))
  t.after(() => rmSync(root, { recursive: true, force: true }))
  const put = (path, value) => {
    mkdirSync(join(root, path, '..'), { recursive: true })
    writeFileSync(join(root, path), value)
  }
  put('web/package.json', '{}')
  put('web/package-lock.json', '{"packages":{}}')
  put('web/node_modules/.bin/max', '')
  put('web/node_modules/.bin/tsc', '')
  put('web/node_modules/example/package.json', '{}')
  put('web/node_modules/.package-lock.json', '{"packages":{"node_modules/example":{}}}')
  put('web/src/app.tsx', 'original source')
  put('web/dist/index.html', '<script src="chunk.js"></script>')
  put('web/dist/chunk.js', 'original output')
  put('crates/server/src/main.rs', 'original server')
  const env = { BUILD_TIME: '2026-01-01T00:00:00Z', GIT_COMMIT: 'fixture' }
  const cache = createCache(root, { env, node: 'v24', npm: '11', rustc: 'stable' })
  return { root, put, env, cache }
}

test('dependency cache checks lock inputs, installed tree and hidden lock integrity', t => {
  const { root, put, cache } = fixture(t)
  cache.recordDependencies()
  assert.equal(cache.dependenciesValid(), true)
  put('web/src/app.tsx', 'UI changes do not require reinstalling dependencies')
  assert.equal(cache.dependenciesValid(), true)
  put('web/package-lock.json', '{"packages":{},"changed":true}')
  assert.equal(cache.dependenciesValid(), false)
  cache.recordDependencies()
  rmSync(join(root, 'web/node_modules/example'), { recursive: true })
  assert.equal(cache.dependenciesValid(), false)
  put('web/node_modules/example/package.json', '{}')
  put('web/node_modules/.package-lock.json', '{"packages":{"node_modules/example":{}},"changed":true}')
  assert.equal(cache.dependenciesValid(), false)
})

test('frontend cache detects changed, missing or unexpected output and changed source/env', t => {
  const { root, put, cache, env } = fixture(t)
  cache.recordWeb()
  assert.equal(cache.webValid(), true)
  put('web/dist/chunk.js', 'corrupt output')
  assert.equal(cache.webValid(), false)
  put('web/dist/chunk.js', 'original output')
  assert.equal(cache.webValid(), true)
  put('web/dist/unexpected.js', 'stale chunk')
  assert.equal(cache.webValid(), false)
  rmSync(join(root, 'web/dist/unexpected.js'))
  rmSync(join(root, 'web/dist/chunk.js'))
  assert.equal(cache.webValid(), false)
  put('web/dist/chunk.js', 'original output')
  put('web/src/app.tsx', 'new source')
  assert.equal(cache.webValid(), false)
  cache.recordWeb()
  env.UMI_ENV = 'different'
  assert.equal(cache.webValid(), false)
})

test('build time is reused only for unchanged sources, toolchain and binary digests', t => {
  const { put, cache, env, root } = fixture(t)
  const target = 'x86_64-unknown-linux-musl'
  put(`target/${target}/release/mirrorproxy-server`, 'server')
  put(`target/${target}/release/mirrorproxy`, 'client')
  cache.recordNative(target)
  assert.equal(cache.nativeTime(target), env.BUILD_TIME)
  put('crates/server/src/main.rs', 'new server')
  assert.notEqual(cache.nativeTime(target), env.BUILD_TIME)
  put('crates/server/src/main.rs', 'original server')
  put(`target/${target}/release/mirrorproxy-server`, 'corrupt server')
  assert.notEqual(cache.nativeTime(target), env.BUILD_TIME)
  put(`target/${target}/release/mirrorproxy-server`, 'server')
  env.MIRRORPROXY_BUILD_FORCE = '1'
  assert.notEqual(cache.nativeTime(target), env.BUILD_TIME)
  delete env.MIRRORPROXY_BUILD_FORCE
  assert.notEqual(createCache(root, { env, node: 'v24', npm: '11', rustc: 'different' }).nativeTime(target), env.BUILD_TIME)
})

test('build.sh reuses verified inputs, always invokes locked Cargo, and invalidates changes', t => {
  const { root, put } = fixture(t)
  copyFileSync(new URL('../build.sh', import.meta.url), join(root, 'build.sh'))
  mkdirSync(join(root, 'scripts'), { recursive: true })
  copyFileSync(new URL('./build-cache.mjs', import.meta.url), join(root, 'scripts/build-cache.mjs'))
  put('scripts/fetch-geoip.sh', '#!/bin/sh\nexit 0\n')
  put('web/scripts/check-browser-build.mjs', "import {readFileSync} from 'node:fs'; if(!readFileSync('dist/chunk.js','utf8').includes('built')) throw Error('bad output')")
  const executable = (name, content) => { put(`bin/${name}`, content); chmodSync(join(root, `bin/${name}`), 0o755) }
  executable('npm', `#!/usr/bin/env node
const fs=require('node:fs'),p=require('node:path');
const args=process.argv.slice(2);
if(args[0]==='--version'){console.log('11.19.0');process.exit(0)}
fs.appendFileSync(process.env.MOCK_LOG, JSON.stringify(['npm',...args])+'\\n');
if(args[0]==='ci'){
 fs.mkdirSync('node_modules/example',{recursive:true});fs.mkdirSync('node_modules/.bin',{recursive:true});
 fs.writeFileSync('node_modules/.bin/max','');fs.writeFileSync('node_modules/.bin/tsc','');
 fs.writeFileSync('node_modules/.package-lock.json',JSON.stringify({packages:{'node_modules/example':{}}}));
}else if(args[0]==='run'&&args[1]==='build'){
 fs.mkdirSync('dist',{recursive:true});fs.writeFileSync('dist/index.html','built index');fs.writeFileSync('dist/chunk.js','built chunk');
}else process.exit(1);
`)
  executable('rustc', '#!/bin/sh\necho "rustc fixture"\n')
  executable('rustup', '#!/bin/sh\necho x86_64-unknown-linux-musl\n')
  executable('musl-gcc', '#!/bin/sh\nexit 0\n')
  executable('cargo', `#!/usr/bin/env node
const fs=require('node:fs'),p=require('node:path');
const args=process.argv.slice(2);fs.appendFileSync(process.env.MOCK_LOG,JSON.stringify(['cargo',...args])+'\\n');
const target=args[args.indexOf('--target')+1],dir=p.join('target',target,'release');fs.mkdirSync(dir,{recursive:true});
for(const name of ['mirrorproxy-server','mirrorproxy'])fs.writeFileSync(p.join(dir,name),name+process.env.BUILD_TIME);
`)
  const env = { ...process.env, PATH: `${join(root, 'bin')}:${process.env.PATH}`, GIT_COMMIT: 'fixture', MOCK_LOG: join(root, 'commands.log') }
  for (const name of ['BUILD_TIME', 'CARGO_TARGET_DIR', 'TARGET', 'MIRRORPROXY_BUILD_FORCE']) delete env[name]
  const run = args => execFileSync('bash', ['build.sh', ...args], { cwd: root, env, encoding: 'utf8' })
  const commands = () => readFileSync(env.MOCK_LOG, 'utf8').trim().split('\n').map(line => JSON.parse(line))
  run([])
  const firstServer = readFileSync(join(root, 'target/x86_64-unknown-linux-musl/release/mirrorproxy-server'), 'utf8')
  assert.match(run([]), /reusing node_modules[\s\S]*reusing web\/dist/)
  assert.equal(readFileSync(join(root, 'target/x86_64-unknown-linux-musl/release/mirrorproxy-server'), 'utf8'), firstServer, 'unchanged builds retain build time and binary bytes')
  assert.equal(commands().filter(c => c[0] === 'npm' && c[1] === 'ci').length, 1)
  assert.equal(commands().filter(c => c[0] === 'npm' && c[2] === 'build').length, 1)
  assert.equal(commands().filter(c => c[0] === 'cargo').length, 2)
  assert(commands().filter(c => c[0] === 'cargo').every(c => c.includes('--locked') && c.includes('mirrorproxy-server') && c.includes('mirrorproxy-cli')))
  put('web/dist/chunk.js', 'tampered output')
  run([])
  assert.equal(commands().filter(c => c[0] === 'npm' && c[2] === 'build').length, 2)
  put('web/package-lock.json', '{"packages":{},"changed":true}')
  run([])
  assert.equal(commands().filter(c => c[0] === 'npm' && c[1] === 'ci').length, 2)
  run(['--force'])
  assert.equal(commands().filter(c => c[0] === 'npm' && c[1] === 'ci').length, 3)
  assert.equal(commands().filter(c => c[0] === 'npm' && c[2] === 'build').length, 4)
})
