import { Panel } from '../components/controls'
import { request } from '../services/request'
import { SettingsForm, Field, Action } from '../components/controls'
import { byteLabel } from '../formatting'
import { OperationToast } from '../operation-toast'
import * as React from 'react'
import { ChartNoAxesCombined, ChevronDown, Globe2, RefreshCw } from 'lucide-react'
import { type Locale, type GeoTrafficOverview } from '../types'
import { PROXY_ADAPTERS, ConsoleMetric } from '../components/shared'

const TrafficChart = React.lazy(() => import('../components/traffic-chart'))

export function AdminGeoTraffic({ locale }: { locale: Locale }) {
  const today = new Date().toISOString().slice(0, 10)
  const [from, setFrom] = React.useState(`${today.slice(0, 7)}-01`)
  const [to, setTo] = React.useState(today)
  const [target, setTarget] = React.useState('')
  const [data, setData] = React.useState<GeoTrafficOverview | null>(null)
  const [busy, setBusy] = React.useState(false)
  const [error, setError] = React.useState('')

  const load = React.useCallback(async () => {
    setBusy(true); setError('')
    const params = new URLSearchParams({ from, to }); if (target) params.set('target', target)
    try {
      const response = await request(`/admin/api/geo-traffic?${params}`)
      if (!response.ok) throw new Error(await response.text())
      const value = await response.json() as { overview: GeoTrafficOverview }
      setData(value.overview)
    } catch {
      setError(locale === 'zh' ? '地域流量读取失败，请检查日期范围。' : 'Regional traffic could not be loaded. Check the date range.')
    } finally { setBusy(false) }
  }, [from, locale, target, to])
  React.useEffect(() => { load() }, [load])

  const countries = React.useMemo(() => {
    const grouped = new Map<string, { code: string; name: string; billed: number; delivered: number; requests: number; children: GeoTrafficOverview['regions'] }>()
    for (const row of data?.regions ?? []) {
      const key = `${row.country_code}:${row.country}`
      const current = grouped.get(key) ?? { code: row.country_code, name: row.country, billed: 0, delivered: 0, requests: 0, children: [] }
      current.billed += row.billed_bytes; current.delivered += row.response_bytes; current.requests += row.request_count; current.children.push(row); grouped.set(key, current)
    }
    return [...grouped.values()].sort((a, b) => b.billed - a.billed)
  }, [data])
  const peak = Math.max(1, ...countries.map((country) => country.billed))
  const trendPeak = Math.max(1, ...(data?.daily ?? []).map((point) => point.billed_bytes))

  return <section className="admin-tab-panel geo-report-panel">
    <SettingsForm className="geo-filter-strip" onSubmit={(event) => { event.preventDefault(); load() }}>
      <label className="geo-filter-field"><span>{locale === 'zh' ? '开始日期' : 'From'}</span><Field type="date" value={from} onChange={(event) => setFrom(event.target.value)} /></label>
      <label className="geo-filter-field"><span>{locale === 'zh' ? '结束日期' : 'To'}</span><Field type="date" value={to} onChange={(event) => setTo(event.target.value)} /></label>
      <div className="geo-filter-field"><span>{locale === 'zh' ? '代理目标' : 'Proxy target'}</span><details className="geo-target-picker"><summary><span>{target || (locale === 'zh' ? '全部目标' : 'All targets')}</span><ChevronDown size={15} /></summary><div className="geo-target-options" role="listbox" aria-label={locale === 'zh' ? '选择代理目标' : 'Select proxy target'}><Action className={!target ? 'active' : ''} type="button" onClick={(event) => { setTarget(''); event.currentTarget.closest('details')?.removeAttribute('open') }}>{locale === 'zh' ? '全部目标' : 'All targets'}<small>ALL</small></Action>{PROXY_ADAPTERS.map((adapter) => <Action className={target === adapter ? 'active' : ''} key={adapter} type="button" onClick={(event) => { setTarget(adapter); event.currentTarget.closest('details')?.removeAttribute('open') }}>{adapter}<small>{adapter === 'os' ? 'OS' : 'PROXY'}</small></Action>)}</div></details></div>
      <Action className="primary-button" disabled={busy} type="submit"><RefreshCw className={busy ? 'spin' : ''} size={16} /> {locale === 'zh' ? '应用筛选' : 'Apply'}</Action>
    </SettingsForm>
    <OperationToast locale={locale} notice={error ? { tone: 'error', message: error } : null} />
    <div className="geo-metric-ribbon">
      <ConsoleMetric label={locale === 'zh' ? '实际下发' : 'Delivered'} value={byteLabel(data?.response_bytes ?? 0)} />
      <ConsoleMetric label={locale === 'zh' ? '计费流量' : 'Billed'} value={byteLabel(data?.billed_bytes ?? 0)} />
      <ConsoleMetric label={locale === 'zh' ? '请求数' : 'Requests'} value={(data?.request_count ?? 0).toLocaleString()} />
      <ConsoleMetric label={locale === 'zh' ? '错误数' : 'Errors'} value={(data?.error_count ?? 0).toLocaleString()} />
    </div>
    <div className="geo-report-grid">
      <Panel className="settings-card geo-atlas-card"><div className="settings-card-head"><div><h4>{locale === 'zh' ? '国家 / 省市排行' : 'Country / region ranking'}</h4></div><Globe2 size={22} /></div>
        {countries.length ? <div className="geo-country-list">{countries.map((country, index) => <details key={`${country.code}-${country.name}`} open={index < 3} className="geo-country-row"><summary><span className="geo-rank">{String(index + 1).padStart(2, '0')}</span><span><strong>{country.name}</strong><small>{country.code} · {country.requests.toLocaleString()} req</small></span><span className="geo-traffic-value">{byteLabel(country.billed)}</span><i style={{ '--geo-share': `${Math.max(2, country.billed / peak * 100)}%` } as React.CSSProperties} /></summary><div className="geo-city-list">{country.children.slice(0, 30).map((row) => <div key={`${row.province}-${row.city}`}><span>{row.province === 'Unknown' ? '—' : row.province} / {row.city === 'Unknown' ? '—' : row.city}</span><strong>{byteLabel(row.billed_bytes)}</strong><small>{byteLabel(row.response_bytes)} {locale === 'zh' ? '实际' : 'delivered'} · {row.request_count} req</small></div>)}</div></details>)}</div> : <p className="empty-stat">{locale === 'zh' ? '所选范围尚无地域流量。' : 'No regional traffic in this range.'}</p>}
      </Panel>
      <Panel className="settings-card geo-trend-card"><div className="settings-card-head"><div><h4>{locale === 'zh' ? '每日计费流量' : 'Daily billed traffic'}</h4></div><ChartNoAxesCombined size={22} /></div><div className="geo-trend-chart" role="img" aria-label={locale === 'zh' ? '每日计费流量趋势' : 'Daily billed traffic trend'}><React.Suspense fallback={<p role="status">{locale === 'zh' ? '加载图表…' : 'Loading chart…'}</p>}><TrafficChart data={data?.daily ?? []} locale={locale} /></React.Suspense></div></Panel>
    </div>
  </section>
}
