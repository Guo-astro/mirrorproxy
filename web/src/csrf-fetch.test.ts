import { afterEach, describe, expect, it, vi } from 'vitest'
import { installCsrfFetch } from './csrf-fetch'

afterEach(() => { vi.unstubAllGlobals(); vi.restoreAllMocks() })

describe('browser CSRF fetch', () => {
  for (const protocol of ['http:', 'https:']) {
    it(`uses only the ${protocol} cookie for each audience and keeps requests same-origin`, async () => {
      const fetch = vi.fn().mockResolvedValue(new Response(null, { status: 204 }))
      const origin = `${protocol}//mirror.example`
      vi.stubGlobal('window', { fetch, location: { protocol, origin, href: `${origin}/admin` } })
      vi.spyOn(document, 'cookie', 'get').mockReturnValue([
        '__Host-mirrorproxy_admin_csrf=secure-admin', 'mirrorproxy_admin_csrf_http=http-admin',
        '__Host-mirrorproxy_user_csrf=secure-user', 'mirrorproxy_user_csrf_http=http-user',
      ].join('; '))
      installCsrfFetch()
      for (const [path, audience] of [['/admin/api/password', 'admin'], ['/api/account/profile', 'user']]) {
        await window.fetch(path, { method: 'POST', headers: { 'content-type': 'application/json' } })
        const headers = fetch.mock.lastCall![1].headers as Headers
        expect(headers.get('x-mirrorproxy-csrf')).toBe(`${protocol === 'https:' ? 'secure' : 'http'}-${audience}`)
        expect(headers.get('content-type')).toBe('application/json')
      }
      await window.fetch('https://attacker.example/api', { method: 'POST' })
      expect(fetch.mock.lastCall![1].headers).toBeUndefined()
      await window.fetch('/admin/api/config')
      expect(fetch.mock.lastCall![1]).toBeUndefined()
    })
  }

  it('does not use a stale HTTP cookie for HTTPS mutations', async () => {
    const fetch = vi.fn().mockResolvedValue(new Response(null, { status: 204 }))
    vi.stubGlobal('window', { fetch, location: { protocol: 'https:', origin: 'https://mirror.example', href: 'https://mirror.example/admin' } })
    vi.spyOn(document, 'cookie', 'get').mockReturnValue('mirrorproxy_admin_csrf_http=stale')
    installCsrfFetch()
    await window.fetch('/admin/api/password', { method: 'POST' })
    expect(fetch.mock.lastCall![1].headers).toBeUndefined()
  })
})
