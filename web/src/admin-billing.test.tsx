import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import AdminBillingManagement from './admin-billing'
import { ConfirmDialogProvider } from './confirm-dialog'

afterEach(() => { cleanup(); vi.unstubAllGlobals() })

it('loads paginated summaries without per-user reads and resets pagination when searching', async () => {
  const fetcher = vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input)
    if (url.startsWith('/admin/api/users/summary?')) {
      const params = new URL(url, 'http://localhost').searchParams
      const search = params.get('search')
      const id = params.get('page') === '2' ? 11 : 1
      return new Response(JSON.stringify({
        items: [{ id, email: `user${id}@example.com`, display_name: search || `User ${id}`, disabled: false, routing_id: `route${id}`, billing: { group_id: null, quota_mode: 'default', user_monthly_limit_bytes: null }, response_bytes: 1024 }],
        total: search ? 1 : 11, page: Number(params.get('page')), per_page: 10,
      }))
    }
    if (url === '/admin/api/groups' || url === '/admin/api/registration-invitations') return new Response('[]')
    throw new Error(`Unexpected request: ${url}`)
  })
  vi.stubGlobal('fetch', fetcher)
  render(<ConfirmDialogProvider><AdminBillingManagement locale="en" targets={[]} /></ConfirmDialogProvider>)
  await screen.findByText('User 1')
  expect(screen.getByText(/1.0 KB this month/)).toBeTruthy()
  fireEvent.click(screen.getByRole('button', { name: 'Next' }))
  await screen.findByText('User 11')
  fireEvent.change(screen.getByRole('searchbox'), { target: { value: 'Friend' } })
  await screen.findByText('Friend')
  await waitFor(() => expect(fetcher.mock.calls.some(([url]) => String(url).includes('page=1&per_page=10&search=Friend'))).toBe(true))
  expect(fetcher.mock.calls.some(([url]) => /\/users\/\d+\/(billing|usage)/.test(String(url)))).toBe(false)
})

it('shows a retry action when the summary cannot be loaded', async () => {
  let failed = true
  vi.stubGlobal('fetch', vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input)
    if (url.startsWith('/admin/api/users/summary?')) return failed
      ? new Response('{}', { status: 500 })
      : new Response(JSON.stringify({ items: [], total: 0, page: 1, per_page: 10 }))
    return new Response('[]')
  }))
  render(<ConfirmDialogProvider><AdminBillingManagement locale="zh" targets={[]} /></ConfirmDialogProvider>)
  await screen.findByRole('alert')
  failed = false
  fireEvent.click(screen.getByRole('button', { name: '重试' }))
  await waitFor(() => expect(screen.queryByRole('alert')).toBeNull())
})
