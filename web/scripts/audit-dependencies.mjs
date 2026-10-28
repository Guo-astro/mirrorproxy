import { spawnSync } from 'node:child_process'

// Temporary build-tool exception; remove when upstream fixes this advisory.
const exception = 'https://github.com/advisories/GHSA-gcqx-w48m-w2h4'
const result = spawnSync('npm', ['audit', '--json', '--audit-level=low'], { encoding: 'utf8' })
if (result.error || ![0, 1].includes(result.status)) {
  console.error(result.stderr || result.error || `npm audit exited ${result.status}`)
  process.exit(1)
}
let report
try { report = JSON.parse(result.stdout) } catch { console.error('npm audit did not return valid JSON'); process.exit(1) }
if (!report.vulnerabilities || report.error) { console.error('npm audit returned an incomplete report'); process.exit(1) }
const vulnerabilities = report.vulnerabilities
function onlyExcepted(name, visiting = new Set()) {
  const finding = vulnerabilities[name]
  if (!finding || visiting.has(name) || !Array.isArray(finding.via) || !finding.via.length) return false
  const next = new Set(visiting).add(name)
  return finding.via.every(cause => typeof cause === 'string'
    ? onlyExcepted(cause, next)
    : cause.name === 'deepmerge' && cause.url === exception && cause.severity === 'low')
}
const remaining = Object.keys(vulnerabilities).filter(name => !onlyExcepted(name))
if (remaining.length) {
  console.error('Unexcepted npm audit findings:', remaining.join(', '))
  console.error(JSON.stringify(report, null, 2))
  process.exit(1)
}
if (Object.keys(vulnerabilities).length) console.warn(`Temporary build-tool exception: ${exception}; review when upstream fixes it.`)
else console.log('npm audit: no vulnerabilities; remove the temporary exception.')
