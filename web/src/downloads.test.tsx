import { spawnSync } from 'node:child_process'
import { choose } from './test/select'
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import { AdminDownloads, UserDownloads, downloadAuthCommand } from './downloads'
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
  await waitFor(() => expect(fetcher.mock.calls.some(([url]) => String(url).startsWith('/admin/api/users/summary'))).toBe(true))
  await choose(screen.getByRole('combobox', { name: 'Permission subject' }), '1 · Alice a@example.org')
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
    if (init?.method === 'POST') return new Response(JSON.stringify({ id: 'id-1', token: 'mpd_test_secret', expires_at: 1900000000 }), { status: 201 })
    if (init?.method === 'DELETE') return new Response(null, { status: 204 })
    return new Response(JSON.stringify({ enabled: true, allowed: true, tokens: [{ id: 'id-1', name: 'Agent', expires_at: 1900000000 }] }))
  })
  vi.stubGlobal('fetch', fetcher)
  render(<UserDownloads locale="en" />)
  fireEvent.click(await screen.findByRole('button', { name: 'Create download token' }))
  const field = await screen.findByLabelText('Download token') as HTMLInputElement
  expect(field.type).toBe('text'); expect(field.value).toBe('mpd_test_secret')
  expect(localStorage.getItem('mirrorproxy-token')).toBeNull()
  await waitFor(() => expect(screen.getByRole('button', { name: 'Revoke' }).hasAttribute('disabled')).toBe(false))
  fireEvent.click(screen.getByRole('button', { name: 'Revoke' }))
  await waitFor(() => expect(screen.queryByLabelText('Download token')).toBeNull())
})


for (const locale of ['zh', 'en'] as const) {
  it(`identifies permission subjects beyond search results and explains revocation in ${locale}`, async () => {
    let removed = false
    const writes: unknown[] = []
    const fetcher = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method === 'PUT') { writes.push(JSON.parse(String(init.body))); removed = true; return new Response(null, { status: 204 }) }
      if (String(input).startsWith('/admin/api/users/summary')) return new Response(JSON.stringify({ items: [] }))
      return new Response(JSON.stringify({ config, permissions: removed ? [] : [
        { kind: 'user', subject_id: 103, policy: 'allow', subject_name: 'Flex', subject_username: 'flex-account', subject_email: 'flex@example.org' },
        { kind: 'group', subject_id: 7, policy: 'deny', subject_name: 'Restricted group' },
      ], blocks: [], active: [], usage: { domains: [], users: [], daily: [], totals: [], rows: [], truncated: false } }))
    })
    vi.stubGlobal('fetch', fetcher)
    const { container } = render(<ConfirmDialogProvider><AdminDownloads locale={locale} /></ConfirmDialogProvider>)
    await screen.findByText('Flex', { exact: true })
    const row = container.querySelector('.download-permission-row') as HTMLElement
    expect(row.textContent).toContain('flex@example.org')
    expect(row.textContent).toContain('flex-account')
    expect(row.textContent).toContain('103')
    expect(row.textContent).toContain(locale === 'zh' ? '允许自定义下载' : 'Allow custom downloads')
    expect(screen.getByText('Restricted group', { exact: true })).toBeTruthy()
    expect(screen.getByRole('button', { name: locale === 'zh' ? '移除规则' : 'Remove rule' })).toBeTruthy()
    expect(screen.getAllByText(locale === 'zh' ? '此分组不授权' : 'No download grant from this group').length).toBeGreaterThan(0)
    fireEvent.click(within(row).getByRole('button', { name: locale === 'zh' ? '撤销权限' : 'Revoke permission' }))
    const dialog = await screen.findByRole('dialog')
    expect(dialog.textContent).toContain(locale === 'zh' ? '按分组授权决定' : 'group permissions determine access')
    expect(writes).toEqual([])
    fireEvent.click(within(dialog).getByRole('button', { name: locale === 'zh' ? '撤销权限' : 'Revoke permission' }))
    await waitFor(() => expect(writes).toEqual([{ kind: 'user', subject_id: 103, policy: 'inherit' }]))
    await waitFor(() => expect(screen.queryByText('Flex', { exact: true })).toBeNull())
  })
}


it('keeps a successfully created token visible when list refresh fails and retries only the GET', async () => {
  let reads = 0
  let posts = 0
  vi.stubGlobal('fetch', vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
    if (init?.method === 'POST') { posts++; return new Response(JSON.stringify({ id: 'new-id', token: 'mpd_new_credential', expires_at: 1900000000 }), { status: 201 }) }
    reads++
    if (reads === 2) throw new TypeError('network unavailable')
    return new Response(JSON.stringify({ enabled: true, allowed: true, tokens: reads > 2 ? [{ id: 'new-id', name: 'Agent', expires_at: 1900000000 }] : [] }))
  }))
  render(<UserDownloads locale="en" />)
  fireEvent.click(await screen.findByRole('button', { name: 'Create download token' }))
  const field = await screen.findByLabelText('Download token') as HTMLInputElement
  expect(field.value).toBe('mpd_new_credential')
  expect(field.type).toBe('text')
  expect(screen.queryByRole('button', { name: 'Clear token' })).toBeNull()
  expect((await screen.findByRole('alert')).textContent).toContain('Token created.')
  expect(screen.getByText('Agent', { exact: true })).toBeTruthy()
  fireEvent.click(screen.getByRole('button', { name: 'Retry' }))
  await waitFor(() => expect(screen.queryByRole('alert')).toBeNull())
  expect(posts).toBe(1)
  expect(field.value).toBe('mpd_new_credential')
})

it('reports the actual CSRF failure without creating a token and permits a manual retry', async () => {
  let posts = 0
  vi.stubGlobal('fetch', vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
    if (init?.method === 'POST') {
      posts++
      return posts === 1 ? new Response(JSON.stringify({ error: 'invalid CSRF proof' }), { status: 403 }) : new Response(JSON.stringify({ id: 'created-id', token: 'mpd_retry_credential', expires_at: 1900000000 }), { status: 201 })
    }
    return new Response(JSON.stringify({ enabled: true, allowed: true, tokens: [] }))
  }))
  render(<UserDownloads locale="en" />)
  fireEvent.click(await screen.findByRole('button', { name: 'Create download token' }))
  expect((await screen.findByRole('alert')).textContent).toMatch(/security validation/i)
  expect(screen.queryByLabelText('Download token')).toBeNull()
  await waitFor(() => expect(screen.getByRole('button', { name: 'Create download token' }).hasAttribute('disabled')).toBe(false))
  fireEvent.click(screen.getByRole('button', { name: 'Create download token' }))
  await waitFor(() => expect((screen.getByLabelText('Download token') as HTMLInputElement).value).toBe('mpd_retry_credential'))
  expect(posts).toBe(2)
})

it('prevents concurrent creates and retains the new token when an older token is revoked', async () => {
  let finish!: (value: Response) => void
  const pending = new Promise<Response>(resolve => { finish = resolve })
  let posts = 0
  let removed = false
  vi.stubGlobal('fetch', vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
    if (init?.method === 'POST') { posts++; return pending }
    if (init?.method === 'DELETE') { removed = true; return new Response(null, { status: 204 }) }
    return new Response(JSON.stringify({ enabled: true, allowed: true, tokens: removed ? [] : [{ id: 'old-id', name: 'Older token', expires_at: 1900000000 }] }))
  }))
  render(<UserDownloads locale="en" />)
  const submit = await screen.findByRole('button', { name: 'Create download token' })
  fireEvent.submit(submit.closest('form')!)
  fireEvent.submit(submit.closest('form')!)
  expect(posts).toBe(1)
  finish(new Response(JSON.stringify({ id: 'new-id', token: 'mpd_new_value', expires_at: 1900000000 }), { status: 201 }))
  await screen.findByLabelText('Download token')
  await waitFor(() => expect(screen.getByRole('button', { name: 'Revoke' }).hasAttribute('disabled')).toBe(false))
  fireEvent.click(screen.getByRole('button', { name: 'Revoke' }))
  await waitFor(() => expect(screen.queryByText('Older token', { exact: true })).toBeNull())
  expect((screen.getByLabelText('Download token') as HTMLInputElement).value).toBe('mpd_new_value')
})


it('generates a POSIX setup command that sends the exact token via stdin without shell interpolation', () => {
  const origin = "https://mirror.example/a'b"
  const token = "mpd_test'$(printf injected)"
  const command = downloadAuthCommand(origin, token, 'posix')
  const result = spawnSync('sh', ['-c', `mirrorproxy() { printf '%s\\n' "$@"; cat; }; ${command}`], { encoding: 'utf8' })
  expect(result.status).toBe(0)
  expect(result.stdout).toBe(`download\n--base-url\n${origin}\nauth\n--token-stdin\n${token}\n`)
})

it('quotes PowerShell literals and passes the credential through the native pipeline', () => {
  expect(downloadAuthCommand("https://mirror.example/a'b", "mpd_a'b", 'powershell'))
    .toBe("'mpd_a''b' | mirrorproxy download --base-url 'https://mirror.example/a''b' auth --token-stdin")
})
