export const CSRF_ERROR_EVENT = 'mirrorproxy:csrf-error'

const UNSAFE_METHODS = new Set(['POST', 'PUT', 'PATCH', 'DELETE'])

function cookieValue(name: string) {
  const prefix = `${name}=`
  const entry = document.cookie.split(';').map((value) => value.trim()).find((value) => value.startsWith(prefix))
  return entry ? decodeURIComponent(entry.slice(prefix.length)) : null
}

export function installCsrfFetch() {
  const originalFetch = window.fetch.bind(window)
  window.fetch = ((input: RequestInfo | URL, init?: RequestInit) => {
    const method = (init?.method ?? (input instanceof Request ? input.method : 'GET')).toUpperCase()
    if (!UNSAFE_METHODS.has(method)) return originalFetch(input, init)

    const rawUrl = input instanceof Request ? input.url : input.toString()
    const url = new URL(rawUrl, window.location.href)
    if (url.origin !== window.location.origin) return originalFetch(input, init)

    const audience = url.pathname.startsWith('/admin/') ? 'admin' : 'user'
    const cookieName = window.location.protocol === 'https:'
      ? `__Host-mirrorproxy_${audience}_csrf`
      : `mirrorproxy_${audience}_csrf_http`
    const token = cookieValue(cookieName)
    const headers = new Headers(input instanceof Request ? input.headers : undefined)
    new Headers(init?.headers).forEach((value, name) => headers.set(name, value))
    if (token) headers.set('x-mirrorproxy-csrf', token)
    const requestInit = token ? { ...init, headers } : init
    return originalFetch(input, requestInit).then(async (response) => {
      if (response.status === 403 && (url.pathname.startsWith('/admin/api/') || url.pathname.startsWith('/api/account/') || url.pathname === '/api/auth/logout')) {
        try {
          const body = await response.clone().json() as { code?: string; error?: string }
          if (body.code === 'csrf_invalid' || body.error === 'invalid CSRF proof') {
            window.dispatchEvent(new CustomEvent(CSRF_ERROR_EVENT, { detail: { audience } }))
          }
        } catch { /* Keep non-JSON errors and response bodies unchanged. */ }
      }
      return response
    })
  }) as typeof window.fetch
}
