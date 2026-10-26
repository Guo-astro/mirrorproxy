import { createHash } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, readdirSync, realpathSync, renameSync, statSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { execFileSync } from 'node:child_process'

const excluded = new Set(['node_modules', 'dist', 'target', '.git', '.tmp', '.umi', '.umi-production', 'test-results', 'playwright-report'])
const sha = value => createHash('sha256').update(value).digest('hex')
const json = path => { try { return JSON.parse(readFileSync(path, 'utf8')) } catch { return null } }

/** Content-based keys: mtimes alone cannot detect restored/edited build inputs. */
export function fingerprint(root, paths) {
  const hash = createHash('sha256')
  const visiting = new Set()
  const walk = path => {
    hash.update(JSON.stringify(path))
    const full = join(root, path)
    if (!existsSync(full)) { hash.update('missing'); return }
    const actual = realpathSync(full)
    if (visiting.has(actual)) throw new Error(`Cyclic build input: ${path}`)
    const stat = statSync(full)
    if (stat.isDirectory()) {
      visiting.add(actual)
      for (const name of readdirSync(full).sort()) if (!excluded.has(name)) walk(join(path, name))
      visiting.delete(actual)
    } else {
      const bytes = readFileSync(full)
      hash.update(String(bytes.length)).update(':').update(bytes)
    }
  }
  for (const path of [...paths].sort()) walk(path)
  return hash.digest('hex')
}

export function createCache(root, { env = process.env, node = process.version, npm = '', rustc = '' } = {}) {
  const targetDir = resolve(root, env.CARGO_TARGET_DIR || 'target')
  const cacheDir = join(targetDir, 'mirrorproxy-build-cache')
  const environment = pattern => Object.fromEntries(Object.entries(env).filter(([key]) => pattern.test(key)).sort(([a], [b]) => a.localeCompare(b)))
  const key = (...values) => sha(JSON.stringify([1, ...values]))
  const dependenciesKey = () => key(node, npm, process.platform, process.arch,
    fingerprint(root, ['web/package.json', 'web/package-lock.json', 'web/build', 'web/.npmrc', 'scripts/build-cache.mjs']),
    environment(/^(NODE_ENV|NPM_CONFIG_|npm_config_)/))
  const webKey = () => key(dependenciesKey(), fingerprint(root, ['web']),
    environment(/^(NODE_|BABEL_|UMI_|MAX_|BROWSERSLIST|MIRRORPROXY_DEV_API)/))
  const outputsKey = () => existsSync(join(root, 'web/dist/index.html')) ? fingerprint(join(root, 'web/dist'), readdirSync(join(root, 'web/dist'))) : null
  const hiddenLock = join(root, 'web/node_modules/.package-lock.json')
  const statePath = name => join(cacheDir, `${name}.json`)
  const save = (name, value) => {
    mkdirSync(cacheDir, { recursive: true })
    const path = statePath(name)
    const temporary = `${path}.${process.pid}.tmp`
    writeFileSync(temporary, JSON.stringify(value))
    renameSync(temporary, path)
  }
  const nativePaths = target => ['mirrorproxy-server', 'mirrorproxy'].map(name => join(targetDir, target, 'release', name + (target.includes('windows') ? '.exe' : '')))
  const nativeKey = target => key(target, rustc, fingerprint(root, ['Cargo.toml', 'Cargo.lock', '.cargo', 'rust-toolchain.toml', 'rust-toolchain', 'crates', 'scripts/install.sh', 'scripts/install.ps1', 'build.sh', 'scripts/build-cache.mjs']), outputsKey(),
    environment(/^(GIT_COMMIT$|CARGO_|RUST|CC($|_)|CXX($|_)|AR$|TARGET$|PATH$|SOURCE_DATE_EPOCH$|OPENSSL_|PKG_CONFIG_)/))
  const binaryKeys = target => nativePaths(target).map(path => existsSync(path) ? sha(readFileSync(path)) : null)
  return {
    dependenciesValid() {
      const state = json(statePath('dependencies'))
      if (!state || state.key !== dependenciesKey() || !existsSync(hiddenLock) || state.lock !== sha(readFileSync(hiddenLock))) return false
      if (!['umi', 'tsc'].every(name => existsSync(join(root, 'web/node_modules/.bin', name)))) return false
      // Match npm's hidden-lock validity rules, including removed package directories.
      const lock = json(hiddenLock)
      if (!lock?.packages) return false
      const lockTime = statSync(hiddenLock).mtimeMs
      return Object.keys(lock.packages).every(path => existsSync(join(root, 'web', path)) && statSync(join(root, 'web', path)).mtimeMs <= lockTime)
    },
    recordDependencies() { save('dependencies', { key: dependenciesKey(), lock: sha(readFileSync(hiddenLock)) }) },
    webValid() {
      const state = json(statePath('web'))
      return Boolean(state && state.key === webKey() && state.outputs && state.outputs === outputsKey())
    },
    recordWeb() {
      const outputs = outputsKey()
      if (!outputs) throw new Error('Cannot cache a missing frontend build')
      save('web', { key: webKey(), outputs })
    },
    nativeTime(target) {
      const state = json(statePath(`native-${target}`))
      const binaries = binaryKeys(target)
      if (env.MIRRORPROXY_BUILD_FORCE !== '1' && state?.key === nativeKey(target) && binaries.every(Boolean) && JSON.stringify(state.binaries) === JSON.stringify(binaries)) return state.builtAt
      return new Date().toISOString()
    },
    recordNative(target) {
      const binaries = binaryKeys(target)
      if (!binaries.every(Boolean)) throw new Error('Cannot cache missing native binaries')
      save(`native-${target}`, { key: nativeKey(target), binaries, builtAt: env.BUILD_TIME })
    },
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
  const [command, target] = process.argv.slice(2)
  const cache = createCache(root, { npm: process.env.MIRRORPROXY_BUILD_NPM_VERSION, rustc: command?.includes('native') ? execFileSync('rustc', ['-Vv'], { encoding: 'utf8' }) : '' })
  switch (command) {
    case 'dependencies-valid': process.exitCode = cache.dependenciesValid() ? 0 : 1; break
    case 'record-dependencies': cache.recordDependencies(); break
    case 'web-valid': process.exitCode = cache.webValid() ? 0 : 1; break
    case 'record-web': cache.recordWeb(); break
    case 'native-time': console.log(cache.nativeTime(target)); break
    case 'record-native': cache.recordNative(target); break
    default: throw new Error(`Unknown build-cache operation: ${command}`)
  }
}
