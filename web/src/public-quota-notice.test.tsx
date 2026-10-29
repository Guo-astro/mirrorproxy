import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import { PublicQuotaNotice } from './components/public-quota-notice'
import { App } from './main'
import { type PublicConfig } from './types'

const quota: PublicConfig['quota'] = { enabled: true, bidirectional_accounting: true, monthly_gb: 500, timezone: 'Asia/Shanghai', on_exceeded: 'stop_proxy', month: '2026-10', status: 'exhausted' }

afterEach(() => { cleanup(); vi.unstubAllGlobals(); localStorage.clear(); document.documentElement.removeAttribute('data-theme') })

it('announces an exhausted quota in both languages with the configured reset rule', () => {
  const { rerender } = render(<PublicQuotaNotice quota={quota} locale="en" />)
  expect(screen.getByRole('alert', { name: 'Monthly traffic limit reached' }).textContent).toContain('Proxy downloads are paused')
  expect(screen.getByText(/New allowance on the 1st/).textContent).toContain('Asia/Shanghai')
  rerender(<PublicQuotaNotice quota={quota} locale="zh" />)
  expect(screen.getByRole('alert', { name: '本月流量额度已用尽' }).textContent).toContain('本站代理下载已暂停')
  expect(screen.getByText(/每月 1 日/).textContent).toContain('500 GB/月')
})

it('distinguishes temporary capacity limits and the throttle policy', () => {
  const { rerender } = render(<PublicQuotaNotice quota={{ ...quota, status: 'reserved' }} locale="en" />)
  expect(screen.getByRole('alert').textContent).toContain('Please retry shortly')
  expect(screen.queryByText('Monthly traffic limit reached')).toBeNull()
  rerender(<PublicQuotaNotice quota={{ ...quota, on_exceeded: 'throttle', timezone: 'local' }} locale="en" />)
  expect(screen.getByRole('alert').textContent).toContain('Proxy requests are rate limited')
  expect(screen.getByRole('alert').textContent).toContain('server local time')
})

it.each([
  { ...quota, enabled: false },
  { ...quota, status: 'available' as const },
  { ...quota, status: 'unavailable' as const },
  { ...quota, status: undefined },
])('does not announce exhausted traffic without a current enabled block: %j', value => {
  render(<PublicQuotaNotice quota={value} locale="en" />)
  expect(screen.queryByRole('alert')).toBeNull()
})

it('refreshes availability on window focus and removes the warning after recovery', async () => {
  window.history.replaceState({}, '', '/')
  localStorage.setItem('mirrorproxy.locale', 'en')
  let status: PublicConfig['quota']['status'] = 'exhausted'
  vi.stubGlobal('fetch', vi.fn(async (input: string) => {
    if (input === '/api/public-config') return new Response(JSON.stringify({ public_base_url: 'https://mirror.example', enabled_proxies: [], quota: { ...quota, status } }))
    return new Response('{}', { status: 404 })
  }))
  const { container } = render(<App />)
  await screen.findByRole('alert', { name: 'Monthly traffic limit reached' })
  expect(container.querySelector('.hero-stats')?.textContent).toContain('Proxy paused')
  status = 'available'
  fireEvent(window, new Event('focus'))
  await waitFor(() => expect(screen.queryByRole('alert', { name: 'Monthly traffic limit reached' })).toBeNull())
  expect(container.querySelector('.hero-stats')?.textContent).toContain('Online')
})
