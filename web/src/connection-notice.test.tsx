import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import { AdminConnectionNotice } from './connection-notice'
import { CSRF_ERROR_EVENT } from './csrf-fetch'

afterEach(() => { cleanup(); vi.unstubAllGlobals(); vi.restoreAllMocks() })

it('proactively explains an untrusted Docker gateway without changing configuration', async () => {
  const fetcher = vi.fn(async () => new Response(JSON.stringify({ peer_ip: '192.168.64.1', trusted_proxy: false, forwarded_https: true, effective_https: false })))
  vi.stubGlobal('fetch', fetcher)
  const configure = vi.fn()
  render(<AdminConnectionNotice locale="zh" protocol="https:" onConfigure={configure} />)
  expect(await screen.findByText('HTTPS 反向代理识别异常')).toBeTruthy()
  expect(screen.getByText('192.168.64.1')).toBeTruthy()
  expect(screen.getByText(/仅修改公开地址不会使代理受信任/)).toBeTruthy()
  expect(screen.getByText(/只刷新页面不够/)).toBeTruthy()
  fireEvent.click(screen.getByRole('button', { name: '检查可信代理设置' }))
  expect(configure).toHaveBeenCalledOnce()
  expect(fetcher).toHaveBeenCalledExactlyOnceWith('/admin/api/connection-info')
})

it('shows general recovery for CSRF without blaming a healthy reverse proxy', async () => {
  vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ peer_ip: '127.0.0.1', trusted_proxy: true, forwarded_https: true, effective_https: true }))))
  render(<AdminConnectionNotice locale="en" protocol="https:" onConfigure={() => {}} />)
  await waitFor(() => expect(fetch).toHaveBeenCalled())
  expect(screen.queryByRole('alert')).toBeNull()
  fireEvent(window, new CustomEvent(CSRF_ERROR_EVENT, { detail: { audience: 'user' } }))
  expect(screen.queryByRole('alert')).toBeNull()
  fireEvent(window, new CustomEvent(CSRF_ERROR_EVENT, { detail: { audience: 'admin' } }))
  expect(await screen.findByText('Request security validation failed (CSRF)')).toBeTruthy()
  expect(screen.getByText(/Stale cookies or an origin mismatch/)).toBeTruthy()
  expect(screen.queryByText('HTTPS reverse proxy is not recognized')).toBeNull()
})
