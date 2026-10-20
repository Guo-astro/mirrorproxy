import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import { AdminLocalAccounts, PasswordAccess } from './local-accounts'

afterEach(() => { cleanup(); vi.restoreAllMocks() })

it('only submits invitation registration after matching passwords and retains failures for retry', async () => {
  const signedIn = vi.fn(async () => {})
  const fetcher = vi.fn(async (_url: string, _init?: RequestInit) => new Response(JSON.stringify({ error: 'account already exists' }), { status: 409 }))
  vi.stubGlobal('fetch', fetcher)
  render(<PasswordAccess locale="zh" token="invitation-token" onSignedIn={signedIn} />)
  fireEvent.change(screen.getByLabelText('账号'), { target: { value: 'friend' } })
  fireEvent.change(screen.getByLabelText('显示名称'), { target: { value: 'Friend' } })
  fireEvent.change(screen.getByLabelText('密码'), { target: { value: 'a-unique-passphrase' } })
  fireEvent.change(screen.getByLabelText('确认密码'), { target: { value: 'different-passphrase' } })
  fireEvent.submit(screen.getByRole('button', { name: '完成注册' }).closest('form')!)
  expect(await screen.findByText('两次输入的密码不一致。')).toBeTruthy()
  expect(fetcher).not.toHaveBeenCalled()
  fireEvent.change(screen.getByLabelText('确认密码'), { target: { value: 'a-unique-passphrase' } })
  fireEvent.submit(screen.getByRole('button', { name: '完成注册' }).closest('form')!)
  expect(await screen.findByText('账号已存在，请换一个账号重试。')).toBeTruthy()
  expect(fetcher.mock.calls[0]).toEqual(['/api/auth/register', expect.objectContaining({ body: JSON.stringify({ token: 'invitation-token', username: 'friend', password: 'a-unique-passphrase', display_name: 'Friend' }) })])
  expect(signedIn).not.toHaveBeenCalled()
  expect((screen.getByLabelText('账号') as HTMLInputElement).value).toBe('friend')
})

it('generates and copies links with administrator-selected quota', async () => {
  const clipboard = vi.fn(async () => {})
  Object.assign(navigator, { clipboard: { writeText: clipboard } })
  const fetcher = vi.fn(async (url: string) => new Response(JSON.stringify(url === '/admin/api/registration-invitations' ? [] : {})))
  fetcher.mockImplementation(async (_url: string, init?: RequestInit) => new Response(JSON.stringify(init?.method === 'POST' ? { url: 'https://mirror.example/login#register=secret' } : [])))
  vi.stubGlobal('fetch', fetcher)
  render(<AdminLocalAccounts locale="en" groups={[{ id: 7, name: 'Friends' }]} reload={async () => {}} />)
  const inviteForm = screen.getByRole('button', { name: 'Generate invitation link' }).closest('form')!
  fireEvent.change(inviteForm.querySelector('select')!, { target: { value: '7' } })
  fireEvent.change(inviteForm.querySelectorAll('select')[1], { target: { value: 'custom' } })
  fireEvent.change(screen.getByLabelText('Monthly quota GB'), { target: { value: '25' } })
  fireEvent.submit(inviteForm)
  await screen.findByLabelText('Registration link (shown only now)')
  fireEvent.click(screen.getByRole('button', { name: 'Copy invitation link' }))
  await waitFor(() => expect(clipboard).toHaveBeenCalledWith('https://mirror.example/login#register=secret'))
  expect(fetcher).toHaveBeenCalledWith('/admin/api/registration-invitations', expect.objectContaining({ body: JSON.stringify({ billing: { group_id: 7, quota_mode: 'custom', monthly_gb: 25 }, expires_in_hours: 72 }) }))
})
