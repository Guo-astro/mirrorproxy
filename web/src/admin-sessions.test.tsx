import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import { AdminSessionManagement } from './pages/admin-sessions'
import { ConfirmDialogProvider } from './confirm-dialog'

afterEach(() => { cleanup(); vi.unstubAllGlobals() })
const sessions = [
  { id: 'current-session', auth_method: 'passkey', created_at: 1792972800, last_used_at: 1792972801, expires_at: 1793059200, current: true },
  { id: 'other-session', auth_method: 'password', created_at: 1792972800, last_used_at: 1792972802, expires_at: 1793059200, current: false },
]

it('shows methods, separate time columns and current state, retaining sessions when revocation fails', async () => {
  let finish!: (value: Response) => void
  const pending = new Promise<Response>(resolve => { finish = resolve })
  const onCurrentRevoked = vi.fn()
  const fetcher = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => init?.method === 'DELETE' ? pending : new Response(JSON.stringify(sessions)))
  vi.stubGlobal('fetch', fetcher)
  const { container } = render(<ConfirmDialogProvider><AdminSessionManagement locale="en" onCurrentRevoked={onCurrentRevoked} /></ConfirmDialogProvider>)
  await screen.findByRole('columnheader', { name: 'Created' })
  expect(screen.getByRole('columnheader', { name: 'Last used' })).toBeTruthy()
  expect(screen.getByRole('columnheader', { name: 'Expires' })).toBeTruthy()
  expect(screen.getByText('Current', { exact: true })).toBeTruthy()
  expect(container.querySelectorAll('time')).toHaveLength(6)
  const row = screen.getByText('Password', { exact: true }).closest('tr')!
  fireEvent.click(within(row).getByRole('button', { name: 'Revoke' }))
  const dialog = await screen.findByRole('dialog')
  fireEvent.click(within(dialog).getByRole('button', { name: 'Revoke session' }))
  await waitFor(() => expect(fetcher.mock.calls.some(([, init]) => init?.method === 'DELETE')).toBe(true))
  expect(within(row).getByRole('button', { name: 'Working…' }).hasAttribute('disabled')).toBe(true)
  finish(new Response(JSON.stringify({ error: 'unavailable' }), { status: 503 }))
  expect((await screen.findByRole('alert')).textContent).toContain('Unable to revoke the session.')
  expect(screen.getByText('Password', { exact: true })).toBeTruthy()
  expect(onCurrentRevoked).not.toHaveBeenCalled()
})

it('confirms and signs out when the current session is successfully revoked', async () => {
  const onCurrentRevoked = vi.fn()
  const fetcher = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => init?.method === 'DELETE' ? new Response(null, { status: 204 }) : new Response(JSON.stringify(sessions)))
  vi.stubGlobal('fetch', fetcher)
  render(<ConfirmDialogProvider><AdminSessionManagement locale="zh" onCurrentRevoked={onCurrentRevoked} /></ConfirmDialogProvider>)
  const row = (await screen.findByText('当前会话', { exact: true })).closest('tr')!
  fireEvent.click(within(row).getByRole('button', { name: '撤销' }))
  const dialog = await screen.findByRole('dialog')
  expect(dialog.textContent).toContain('撤销后需要重新登录')
  expect(onCurrentRevoked).not.toHaveBeenCalled()
  fireEvent.click(within(dialog).getByRole('button', { name: '撤销会话' }))
  await waitFor(() => expect(onCurrentRevoked).toHaveBeenCalledOnce())
  expect(fetcher.mock.calls.some(([input, init]) => input === '/admin/api/auth/sessions/current-session' && init?.method === 'DELETE')).toBe(true)
})
