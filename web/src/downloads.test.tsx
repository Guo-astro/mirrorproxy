import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import { AdminDownloads, UserDownloads } from './downloads'
import { ConfirmDialogProvider } from './confirm-dialog'
afterEach(() => { cleanup(); vi.unstubAllGlobals() })
const config = { enabled: false, max_file_bytes: 2147483648, daily_user_bytes: 5368709120, max_concurrent: 16, max_user_concurrent: 2, requests_per_minute: 30, max_duration_secs: 1800, bandwidth_bytes_per_sec: 20971520 }
const usage = { domain: 'cdn.example.org', entry_domain: 'example.org', day: '2026-10-24', user_id: 1, requests: 2, bytes: 1024, upstream_bytes: 2048, errors: 1, denied: 0 }
it('shows final-host traffic and saves explicit user permission and domain block scope', async () => {
  const fetcher = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    if (init?.method === 'PUT') return new Response(null, { status: 204 })
    if (String(input).startsWith('/admin/api/users/summary')) return new Response(JSON.stringify({ items: [{ id: 1, display_name: 'Alice', email: 'a@example.org' }] }))
    return new Response(JSON.stringify({ config, permissions: [], blocks: [], active: [], usage: { domains: [usage], users: [usage], daily: [usage], totals: [usage], rows: [usage], truncated: false } }))
  })
  vi.stubGlobal('fetch', fetcher)
  render(<ConfirmDialogProvider><AdminDownloads locale="en" /></ConfirmDialogProvider>)
  await screen.findByRole('button', { name: 'cdn.example.org' })
  await screen.findByRole('option', { name: /Alice/ })
  fireEvent.change(screen.getByLabelText('Permission subject'), { target: { value: '1' } })
  fireEvent.click(screen.getByRole('button', { name: 'Save permission' }))
  await waitFor(() => expect(fetcher.mock.calls.some(([url, init]) => String(url).endsWith('/permissions') && JSON.parse(init!.body as string).policy === 'allow')).toBe(true))
  await waitFor(() => expect(screen.getByRole('button', { name: 'Set block' }).hasAttribute('disabled')).toBe(false))
  fireEvent.click(screen.getByRole('button', { name: 'Set block' }))
  expect((screen.getByLabelText('Domain') as HTMLInputElement).value).toBe('cdn.example.org')
  fireEvent.click(screen.getByRole('button', { name: /^Block$/ }))
  await screen.findByRole('dialog')
  fireEvent.click(screen.getByRole('button', { name: /^Confirm$/ }))
  await waitFor(() => expect(fetcher.mock.calls.some(([url, init]) => String(url).endsWith('/blocks') && JSON.parse(init!.body as string).subdomains === true)).toBe(true))
})
it('does not expose token creation to users without permission', async () => {
  vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ enabled: true, allowed: false, tokens: [] }))))
  render(<UserDownloads locale="zh" />)
  await screen.findByText('下载通道未启用或尚未获得授权。')
  expect(screen.queryByRole('button', { name: '创建下载 token' })).toBeNull()
})
it('shows a new scoped token only in memory and clears it on revocation', async () => {
  const fetcher = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
    if (init?.method === 'POST') return new Response(JSON.stringify({ token: 'mpd_test_secret' }), { status: 201 })
    if (init?.method === 'DELETE') return new Response(null, { status: 204 })
    return new Response(JSON.stringify({ enabled: true, allowed: true, tokens: [{ id: 'id-1', name: 'Agent', expires_at: 1900000000 }] }))
  })
  vi.stubGlobal('fetch', fetcher)
  render(<UserDownloads locale="en" />)
  fireEvent.click(await screen.findByRole('button', { name: 'Create download token' }))
  const field = await screen.findByLabelText('Download token') as HTMLInputElement
  expect(field.type).toBe('password'); expect(field.value).toBe('mpd_test_secret')
  expect(localStorage.getItem('mirrorproxy-token')).toBeNull()
  await waitFor(() => expect(screen.getByRole('button', { name: 'Revoke' }).hasAttribute('disabled')).toBe(false))
  fireEvent.click(screen.getByRole('button', { name: 'Revoke' }))
  await waitFor(() => expect(screen.queryByLabelText('Download token')).toBeNull())
})
