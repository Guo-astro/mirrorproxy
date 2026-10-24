import * as React from 'react'
import { byteLabel } from './formatting'
import { useConfirmDialog } from './confirm-dialog'
type Locale = 'en' | 'zh'
type Config = { enabled: boolean; max_file_bytes: number; daily_user_bytes: number; max_concurrent: number; max_user_concurrent: number; requests_per_minute: number; max_duration_secs: number; bandwidth_bytes_per_sec: number }
type Usage = { domain?: string; user_id?: number; day?: string; entry_domain?: string; requests: number; errors: number; denied: number; bytes: number; upstream_bytes: number }
type Block = { domain: string; subdomains: boolean; reason: string; actor: string }
type Permission = { kind: 'user' | 'group'; subject_id: number; policy: 'allow' | 'deny' }
type Report = { config: Config; permissions: Permission[]; blocks: Block[]; active: { id: string; user_id: number; domains: string[] }[]; usage: { domains: Usage[]; users: Usage[]; daily: Usage[]; totals: Usage[]; rows: Usage[]; truncated: boolean } }
const text = (locale: Locale, zh: string, en: string) => locale === 'zh' ? zh : en
async function put(path: string, body: unknown) { const r = await fetch(path, { method: 'PUT', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) }); if (!r.ok) throw new Error('Request failed') }

export function AdminDownloads({ locale }: { locale: Locale }) {
  const [report, setReport] = React.useState<Report | null>(null)
  const [draft, setDraft] = React.useState<Config | null>(null)
  const [error, setError] = React.useState(false)
  const [busy, setBusy] = React.useState(false)
  const [days, setDays] = React.useState('7')
  const [filter, setFilter] = React.useState('')
  const [userFilter, setUserFilter] = React.useState('')
  const [domain, setDomain] = React.useState('')
  const [subdomains, setSubdomains] = React.useState(true)
  const [reason, setReason] = React.useState('')
  const [kind, setKind] = React.useState<'user' | 'group'>('user')
  const [subject, setSubject] = React.useState('')
  const [policy, setPolicy] = React.useState('allow')
  const [search, setSearch] = React.useState('')
  const [choices, setChoices] = React.useState<{ id: number; label: string }[]>([])
  const confirm = useConfirmDialog()
  const load = React.useCallback(async (signal?: AbortSignal, resetDraft = false) => {
    const params = new URLSearchParams({ days })
    if (filter.trim()) params.set('domain', filter.trim())
    if (userFilter) params.set('user_id', userFilter)
    const r = await fetch(`/admin/api/download?${params}`, { signal })
    if (!r.ok) throw new Error('Report failed')
    const data = await r.json() as Report
    if (signal?.aborted) return
    setReport(data); setDraft((previous) => resetDraft || !previous ? data.config : previous); setError(false)
  }, [days, filter, userFilter])
  React.useEffect(() => { const c = new AbortController(); load(c.signal).catch(() => { if (!c.signal.aborted) setError(true) }); return () => c.abort() }, [load])
  React.useEffect(() => {
    const c = new AbortController()
    const timer = setTimeout(() => {
      const path = kind === 'group' ? '/admin/api/groups' : `/admin/api/users/summary?page=1&per_page=20&search=${encodeURIComponent(search)}`
      fetch(path, { signal: c.signal }).then(async (r) => { if (!r.ok) throw new Error(); const data = await r.json(); if (!c.signal.aborted) setChoices((kind === 'group' ? data : data.items).map((v: { id: number; name?: string; display_name?: string; email?: string }) => ({ id: v.id, label: `${v.id} · ${v.name ?? v.display_name} ${v.email ?? ''}` }))) }).catch(() => { if (!c.signal.aborted) setError(true) })
    }, 250)
    return () => { clearTimeout(timer); c.abort() }
  }, [kind, search])
  const action = async (work: () => Promise<unknown>, resetDraft = false) => {
    setBusy(true); setError(false)
    try { await work(); await load(undefined, resetDraft) } catch { setError(true) } finally { setBusy(false) }
  }
  const block = async (host: string, blocked: boolean, include = subdomains, note = reason) => {
    if (!await confirm({ locale, title: text(locale, blocked ? '封禁下载域名' : '解除封禁', blocked ? 'Block download domain' : 'Unblock domain'), message: `${host}${include ? text(locale, '（含子域名）', ' (including subdomains)') : ''}`, confirmLabel: text(locale, '确认', 'Confirm'), tone: blocked ? 'danger' : 'primary' })) return
    await action(() => put('/admin/api/download/blocks', { domain: host, subdomains: include, reason: note, blocked }))
  }
  return <section className="admin-tab-panel settings-stack">
    {error ? <p role="alert">{text(locale, '操作失败。此页面需要超级管理员权限，请检查输入后重试。', 'Operation failed. This page requires super administrator access. Check the input and retry.')} <button onClick={() => { load().catch(() => setError(true)) }}>{text(locale, '重试', 'Retry')}</button></p> : null}
    {draft ? <form className="settings-card" onSubmit={(e) => { e.preventDefault(); action(() => put('/admin/api/download/config', draft), true) }}>
      <div className="settings-card-head"><h4>{text(locale, '自定义下载策略', 'Custom download policy')}</h4><p>{text(locale, '默认关闭。只允许已授权用户使用专用 token 下载公网 HTTPS 文件；私网、凭据转发及上传始终禁止。此通道不使用共享缓存或镜像上游代理。保存会终止当前下载。', 'Disabled by default. Authorized users need a download-only token for public HTTPS files. Private networks, credential forwarding and uploads are always forbidden. This channel uses no shared cache or mirror upstream proxy. Saving cancels active downloads.')}</p></div>
      <label className="toggle-field"><input type="checkbox" checked={draft.enabled} onChange={(e) => setDraft({ ...draft, enabled: e.target.checked })} />{text(locale, '启用自定义下载', 'Enable custom downloads')}</label>
      <div className="config-fields">{([
        ['max_file_bytes', text(locale, '单文件上限（GiB）', 'File limit (GiB)'), 1024 ** 3],
        ['daily_user_bytes', text(locale, '每用户每日上限（GiB，UTC）', 'Daily user limit (GiB, UTC)'), 1024 ** 3],
        ['max_concurrent', text(locale, '实例并发上限', 'Instance concurrency'), 1],
        ['max_user_concurrent', text(locale, '用户并发上限', 'User concurrency'), 1],
        ['requests_per_minute', text(locale, '用户每分钟请求数', 'Requests per user/minute'), 1],
        ['max_duration_secs', text(locale, '下载总时限（秒）', 'Download deadline (seconds)'), 1],
        ['bandwidth_bytes_per_sec', text(locale, '实例带宽上限（MiB/s）', 'Instance bandwidth (MiB/s)'), 1024 ** 2],
      ] as const).map(([key, label, unit]) => <label key={key}>{label}<input required type="number" min={unit === 1 ? 1 : 1 / unit} step="any" value={draft[key] / unit} onChange={(e) => setDraft({ ...draft, [key]: Math.round(Number(e.target.value) * unit) })} /></label>)}</div>
      <button className="primary-button" disabled={busy}>{text(locale, '保存下载策略', 'Save download policy')}</button>
    </form> : null}
    <div className="settings-card"><div className="settings-card-head"><h4>{text(locale, '用户与分组授权', 'User and group permissions')}</h4><p>{text(locale, '默认无权限。用户显式拒绝优先于组授权；继承会删除直接规则。组拒绝不覆盖其他组的授权。', 'Denied by default. Explicit user deny overrides group grants. Inherit removes the direct rule. A group deny does not override another group grant.')}</p></div>
      <form className="compact-form inline-form" onSubmit={(e) => { e.preventDefault(); action(() => put('/admin/api/download/permissions', { kind, subject_id: Number(subject), policy })) }}>
        <label>{text(locale, '类型', 'Subject type')}<select value={kind} onChange={(e) => { setKind(e.target.value as 'user' | 'group'); setSubject('') }}><option value="user">{text(locale, '用户', 'User')}</option><option value="group">{text(locale, '分组', 'Group')}</option></select></label>
        {kind === 'user' ? <label>{text(locale, '搜索用户', 'Search users')}<input value={search} onChange={(e) => setSearch(e.target.value)} /></label> : null}
        <label>{text(locale, '授权对象', 'Permission subject')}<select required value={subject} onChange={(e) => setSubject(e.target.value)}><option value="">{text(locale, '请选择', 'Select')}</option>{choices.map((v) => <option key={v.id} value={v.id}>{v.label}</option>)}</select></label>
        <label>{text(locale, '权限', 'Policy')}<select value={policy} onChange={(e) => setPolicy(e.target.value)}><option value="allow">{text(locale, '允许', 'Allow')}</option><option value="deny">{text(locale, '拒绝', 'Deny')}</option><option value="inherit">{text(locale, '继承', 'Inherit')}</option></select></label><button disabled={busy || !subject}>{text(locale, '保存授权', 'Save permission')}</button>
      </form>
      {report?.permissions.map((p) => <div className="stat-row" key={`${p.kind}-${p.subject_id}`}><span>{text(locale, p.kind === 'user' ? '用户' : '分组', p.kind)} #{p.subject_id}</span><strong>{text(locale, p.policy === 'allow' ? '允许' : '拒绝', p.policy)}</strong><button disabled={busy} onClick={() => action(() => put('/admin/api/download/permissions', { ...p, policy: 'inherit' }))}>{text(locale, '恢复继承', 'Reset to inherit')}</button></div>)}
    </div>
    <div className="settings-card"><div className="settings-card-head"><h4>{text(locale, '域名统计', 'Domain traffic')}</h4><p>{text(locale, '流量归于最终下载主机名，明细关联入口域名。按数据块计量，包含已计费但客户端断开前未接收的少量缓冲数据。每日限额使用 UTC，月配额沿用实例时区。', 'Bytes are attributed to the final hostname; details retain the entry hostname. Metering counts queued chunks, including a small buffer on disconnect. Daily limits use UTC; monthly quotas use the instance timezone.')}</p></div>
      <div className="compact-form inline-form"><label>{text(locale, '时间范围', 'Period')}<select value={days} onChange={(e) => setDays(e.target.value)}><option value="1">1 {text(locale, '天', 'day')}</option><option value="7">7 {text(locale, '天', 'days')}</option><option value="30">30 {text(locale, '天', 'days')}</option><option value="90">90 {text(locale, '天', 'days')}</option></select></label><label>{text(locale, '精确域名筛选', 'Exact hostname filter')}<input value={filter} onChange={(e) => setFilter(e.target.value)} /></label><label>{text(locale, '用户 ID 筛选', 'User ID filter')}<input type="number" min="1" value={userFilter} onChange={(e) => setUserFilter(e.target.value)} /></label><button onClick={() => action(async () => undefined)} disabled={busy}>{text(locale, '刷新', 'Refresh')}</button></div>
      {report?.usage.totals.map((v, i) => <p key={i}>{v.requests} {text(locale, '次请求', 'requests')} · {byteLabel(v.bytes)} · {v.errors} {text(locale, '次错误', 'errors')} · {v.denied} {text(locale, '次拒绝', 'denied')}</p>)}
      <div className="download-scroll"><table className="download-table"><thead><tr><th>{text(locale, '域名（前 200）', 'Hostname (top 200)')}</th><th>{text(locale, '请求', 'Requests')}</th><th>{text(locale, '下载 / 回源', 'Download / upstream')}</th><th>{text(locale, '错误 / 拒绝', 'Errors / denied')}</th><th>{text(locale, '操作', 'Action')}</th></tr></thead><tbody>{report?.usage.domains.map((v) => <tr key={v.domain}><td><button onClick={() => setFilter(v.domain!)}>{v.domain}</button></td><td>{v.requests}</td><td>{byteLabel(v.bytes)} / {byteLabel(v.upstream_bytes)}</td><td>{v.errors} / {v.denied}</td><td><button disabled={busy} onClick={() => { setDomain(v.domain!); }}>{text(locale, '设置封禁', 'Set block')}</button></td></tr>)}</tbody></table></div>
      <details><summary>{text(locale, '每日趋势、用户流量与入口关联', 'Daily trend, user traffic and entry hosts')}</summary>
        {report?.usage.daily.slice().sort((a, b) => (a.day ?? '').localeCompare(b.day ?? '')).map((v) => <div className="stat-row" key={v.day}><span>{v.day}</span><strong>{byteLabel(v.bytes)}</strong><small>{v.requests} req · {v.errors} err</small></div>)}
        {report?.usage.users.map((v) => <div className="stat-row" key={v.user_id}><button onClick={() => setUserFilter(String(v.user_id))}>User #{v.user_id}</button><strong>{byteLabel(v.bytes)}</strong><small>{v.requests} req</small></div>)}
        {report?.usage.truncated ? <p>{text(locale, '明细最多显示 2000 行，请缩小筛选范围；总计不受此限制。', 'Details are limited to 2,000 rows; narrow the filters. Totals include all matching rows.')}</p> : null}
        <div className="download-scroll"><table className="download-table"><thead><tr><th>{text(locale, '日期 / 用户', 'Date / user')}</th><th>{text(locale, '入口 → 下载域名', 'Entry → download host')}</th><th>{text(locale, '流量', 'Bytes')}</th></tr></thead><tbody>{report?.usage.rows.map((v) => <tr key={`${v.day}-${v.user_id}-${v.domain}-${v.entry_domain}`}><td>{v.day} / #{v.user_id}</td><td>{v.entry_domain} → {v.domain}</td><td>{byteLabel(v.bytes)}</td></tr>)}</tbody></table></div>
      </details>
    </div>
    <div className="settings-card"><div className="settings-card-head"><h4>{text(locale, '域名封禁', 'Blocked domains')}</h4><p>{text(locale, '域名按完整标签匹配。封禁会取消关联的活跃下载，所有跳转目标均执行封禁检查。', 'Rules match complete hostname labels. Blocking cancels matching active downloads; every redirect is checked.')}</p></div>
      <form className="compact-form inline-form" onSubmit={(e) => { e.preventDefault(); block(domain, true) }}><label>{text(locale, '域名', 'Domain')}<input required value={domain} onChange={(e) => setDomain(e.target.value)} placeholder="downloads.example.org" /></label><label className="toggle-field"><input type="checkbox" checked={subdomains} onChange={(e) => setSubdomains(e.target.checked)} />{text(locale, '包含子域名', 'Include subdomains')}</label><label>{text(locale, '原因', 'Reason')}<input maxLength={256} value={reason} onChange={(e) => setReason(e.target.value)} /></label><button disabled={busy}>{text(locale, '封禁', 'Block')}</button></form>
      {report?.blocks.map((v) => <div className="stat-row" key={v.domain}><span>{v.domain}{v.subdomains ? text(locale, '（含子域名）', ' (+ subdomains)') : ''}<small>{v.reason} · {v.actor}</small></span><button disabled={busy} onClick={() => block(v.domain, false, v.subdomains, v.reason)}>{text(locale, '解除封禁', 'Unblock')}</button></div>)}
    </div>
    <div className="settings-card"><h4>{text(locale, '活跃下载', 'Active downloads')}</h4>{report?.active.map((v) => <div className="stat-row" key={v.id}><span>#{v.user_id} · {v.domains.join(' → ')}</span><button disabled={busy} onClick={() => action(async () => { const r = await fetch(`/admin/api/download/active/${v.id}`, { method: 'DELETE' }); if (!r.ok) throw new Error() })}>{text(locale, '终止', 'Terminate')}</button></div>)}</div>
  </section>
}

type Token = { id: string; name: string; expires_at: number }
export function UserDownloads({ locale }: { locale: Locale }) {
  const [data, setData] = React.useState<{ enabled: boolean; allowed: boolean; tokens: Token[] } | null>(null)
  const [name, setName] = React.useState('Agent')
  const [days, setDays] = React.useState('30')
  const [token, setToken] = React.useState('')
  const [error, setError] = React.useState(false)
  const [busy, setBusy] = React.useState(false)
  const load = React.useCallback(async (signal?: AbortSignal) => { const r = await fetch('/api/account/download', { signal }); if (!r.ok) throw new Error(); const v = await r.json(); if (!signal?.aborted) setData(v) }, [])
  React.useEffect(() => { const c = new AbortController(); load(c.signal).catch(() => { if (!c.signal.aborted) setError(true) }); return () => c.abort() }, [load])
  const action = async (work: () => Promise<void>) => { setBusy(true); setError(false); try { await work(); await load() } catch { setError(true) } finally { setBusy(false) } }
  return <section className="identity-panel"><h3>{text(locale, '自定义下载 token', 'Custom download tokens')}</h3>
    <p className="account-help">{text(locale, '需要管理员单独授权。token 仅用于下载，不授予管理权限；请在本地配置，勿发给 Agent 或写入 Skill。', 'Separate administrator approval is required. Tokens grant downloads only. Configure locally; never send them to an Agent or put them in a Skill.')}</p>
    {error ? <p role="alert">{text(locale, '无法完成 token 操作，请重试。', 'Token operation failed. Please retry.')}<button onClick={() => action(async () => undefined)}>{text(locale, '重试', 'Retry')}</button></p> : null}
    {data?.enabled && data.allowed ? <form className="account-form" onSubmit={(e) => { e.preventDefault(); action(async () => { const r = await fetch('/api/account/download/tokens', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ name, days: Number(days) }) }); if (!r.ok) throw new Error(); setToken((await r.json()).token) }) }}><label>{text(locale, '名称', 'Name')}<input required maxLength={80} value={name} onChange={(e) => setName(e.target.value)} /></label><label>{text(locale, '有效天数（1–365）', 'Valid days (1–365)')}<input required type="number" min="1" max="365" value={days} onChange={(e) => setDays(e.target.value)} /></label><button disabled={busy}>{text(locale, '创建下载 token', 'Create download token')}</button></form> : data ? <p>{text(locale, '下载通道未启用或尚未获得授权。', 'Downloads are disabled or permission has not been granted.')}</p> : null}
    {token ? <div><label>{text(locale, '仅显示本次，请立即保存', 'Shown once; save it now')}<input aria-label="Download token" type="password" readOnly value={token} /></label><button onClick={() => { navigator.clipboard.writeText(token).catch(() => setError(true)) }}>{text(locale, '复制 token', 'Copy token')}</button><button onClick={() => setToken('')}>{text(locale, '隐藏并清除', 'Clear token')}</button><p><code>mirrorproxy download --base-url ORIGIN auth --token-stdin</code></p></div> : null}
    {data?.tokens.map((v) => <div className="identity-row" key={v.id}><span><strong>{v.name}</strong><small>{new Date(v.expires_at * 1000).toLocaleDateString()} · download:fetch</small></span><button disabled={busy} onClick={() => action(async () => { const r = await fetch(`/api/account/download/tokens/${v.id}`, { method: 'DELETE' }); if (!r.ok) throw new Error(); setToken('') })}>{text(locale, '撤销', 'Revoke')}</button></div>)}
  </section>
}
