import { expect, test } from '@playwright/test'

test('announces the global quota in the consumer design across languages, themes and mobile widths', async ({ page }) => {
  const errors: string[] = []
  page.on('pageerror', error => errors.push(error.message))
  await page.addInitScript(() => { localStorage.setItem('mirrorproxy.locale', 'en'); localStorage.setItem('mirrorproxy.theme', 'light') })
  await page.route('**/api/public-config', route => route.fulfill({ json: {
    public_base_url: 'https://mirror.example', enabled_proxies: ['npm'],
    quota: { enabled: true, monthly_gb: 500, bidirectional_accounting: true, timezone: 'Asia/Shanghai', on_exceeded: 'stop_proxy', month: '2026-10', status: 'exhausted' },
  } }))
  await page.route('**/api/sources', route => route.fulfill({ json: { providers: [], targets: [], sources: [], templates: [], container_registries: [] } }))
  await page.route('**/api/source-health', route => route.fulfill({ json: { items: [] } }))
  await page.route('**/api/account/profile', route => route.fulfill({ status: 401, json: {} }))
  await page.route('**/version', route => route.fulfill({ json: { version: '1.7.1' } }))
  await page.goto('/')
  const notice = page.getByRole('alert', { name: 'Monthly traffic limit reached' })
  await expect(notice).toBeVisible()
  await expect(page.locator('.hero-stats')).toContainText('Proxy paused')
  await expect(page.locator('.hero-stats')).not.toContainText('Online')
  await expect(notice).toContainText('500 GB / month')
  await expect(notice).toContainText('Asia/Shanghai')
  await expect(notice).toHaveCSS('border-radius', '18px')
  await expect(notice).toHaveCSS('box-shadow', 'none')
  await expect(notice).toHaveCSS('background-image', 'none')
  for (const dark of [false, true]) {
    if (dark) await page.getByTitle('Theme').click()
    for (const width of [1440, 375, 320]) {
      await page.setViewportSize({ width, height: 900 })
      await expect(notice).toBeInViewport()
      await expect(notice).toHaveCSS('background-color', dark ? 'rgb(22, 22, 23)' : 'rgb(245, 245, 247)')
      await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth)).toBe(width)
    }
  }
  await page.getByTitle('Language').click()
  const chinese = page.getByRole('alert', { name: '本月流量额度已用尽' })
  await expect(chinese).toContainText('本站代理下载已暂停')
  await expect(page.locator('.hero-stats')).toContainText('代理暂停')
  await expect(chinese).toBeInViewport()
  await page.screenshot({ path: test.info().outputPath('quota-mobile-dark-zh.png') })
  await page.setViewportSize({ width: 1440, height: 1000 })
  await page.getByTitle('Theme').click()
  await page.screenshot({ path: test.info().outputPath('quota-desktop-light-zh.png') })
  expect(errors).toEqual([])
})

test('refreshes quota state automatically and clears a recovered warning without reloading', async ({ page }) => {
  await page.clock.install()
  let status = 'available'
  await page.route('**/api/public-config', route => route.fulfill({ json: {
    public_base_url: 'https://mirror.example', enabled_proxies: [],
    quota: { enabled: true, monthly_gb: 500, timezone: 'local', on_exceeded: 'throttle', month: '2026-10', status },
  } }))
  await page.route('**/api/sources', route => route.fulfill({ json: { providers: [], targets: [], sources: [], templates: [] } }))
  await page.route('**/api/source-health', route => route.fulfill({ json: { items: [] } }))
  await page.route('**/api/account/profile', route => route.fulfill({ status: 401, json: {} }))
  await page.route('**/version', route => route.fulfill({ json: { version: '1.7.1' } }))
  await page.goto('/')
  await expect(page.locator('.hero-stats')).toContainText('Online')
  await expect(page.locator('.public-quota-notice')).toHaveCount(0)
  status = 'exhausted'
  await page.clock.fastForward(31_000)
  await expect(page.getByRole('alert', { name: 'Monthly traffic limit reached' })).toBeVisible()
  await expect(page.locator('.hero-stats')).toContainText('Rate limited')
  status = 'reserved'
  await page.clock.fastForward(31_000)
  await expect(page.getByRole('alert', { name: 'Proxy traffic allowance is temporarily unavailable' })).toContainText('Please retry shortly')
  status = 'available'
  await page.clock.fastForward(31_000)
  await expect(page.locator('.public-quota-notice')).toHaveCount(0)
  await expect(page.locator('.hero-stats')).toContainText('Online')
})
