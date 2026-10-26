import { installCsrfFetch } from '../csrf-fetch'
let installed = false
export function initializeRequests() {
  if (installed || typeof window === 'undefined') return
  installCsrfFetch()
  installed = true
}
/** All browser API traffic uses the existing audience-aware CSRF fetch boundary. */
export function request(input: RequestInfo | URL, init?: RequestInit) {
  return init === undefined ? window.fetch(input) : window.fetch(input, init)
}
