import { Pagination } from './pagination'
import * as React from 'react'
import { AdminLocalAccounts } from './local-accounts'
import { TeamTargetAccess } from './v13-operations'
import { useConfirmDialog } from './confirm-dialog'
import { byteLabel } from './formatting'
type Locale = 'en' | 'zh'
type LinkedIdentity = { id: number; provider_name: string; provider_subject: string; email: string | null }
type UserSummary = AdminUserView & { billing: UserBillingView; response_bytes: number }
type UserSummaryPage = { items: UserSummary[]; total: number; page: number; per_page: number }
const quotaGb = (billing: UserBillingView) => billing.user_monthly_limit_bytes === null ? '' : String(billing.user_monthly_limit_bytes / (1024 ** 3))

type BillingGroupView = { id: number; name: string; monthly_limit_bytes: number | null; member_count: number }
type AdminUserView = { username?: string | null; id: number; email: string; display_name: string; disabled: boolean; routing_id: string }
type UserBillingView = { group_id: number | null; quota_mode: 'default' | 'unlimited' | 'custom'; user_monthly_limit_bytes: number | null }

function BillingGroupRow({ group, locale, reload, targets }: { group: BillingGroupView; locale: Locale; reload: () => Promise<void>; targets: readonly string[] }) {
  const [name, setName] = React.useState(group.name)
  const [quota, setQuota] = React.useState(group.monthly_limit_bytes === null ? '' : String(group.monthly_limit_bytes / (1024 ** 3)))
  const save = async () => {
    const response = await fetch(`/admin/api/groups/${group.id}`, { method: 'PUT', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ name, monthly_gb: quota === '' ? null : Number(quota) }) })
    if (response.ok) await reload()
  }
  return <div className="admin-account-row team-account-row"><span><strong>{group.name}</strong><small>{group.member_count} {locale === 'zh' ? '位成员' : 'members'} · {group.monthly_limit_bytes === null ? (locale === 'zh' ? '不限量' : 'unlimited') : byteLabel(group.monthly_limit_bytes)}</small></span><span><input aria-label={`${group.name} name`} value={name} onChange={(event) => setName(event.target.value)} /><input aria-label={`${group.name} quota`} min="0" type="number" placeholder={locale === 'zh' ? '不限量 GB' : 'Unlimited GB'} value={quota} onChange={(event) => setQuota(event.target.value)} /><button onClick={save}>{locale === 'zh' ? '保存' : 'Save'}</button></span><TeamTargetAccess groupId={group.id} targets={targets} locale={locale} /></div>
}

function BillingUserRow({ initialUser, groups, locale, reloadUsers }: { initialUser: UserSummary; groups: BillingGroupView[]; locale: Locale; reloadUsers: () => Promise<void> }) {
  const confirmAction = useConfirmDialog()
  const [user, setUser] = React.useState(initialUser)
  const [billing, setBilling] = React.useState<UserBillingView>(initialUser.billing)
  const [usage, setUsage] = React.useState({ response_bytes: initialUser.response_bytes })
  const [customGb, setCustomGb] = React.useState(quotaGb(initialUser.billing))
  const [identities, setIdentities] = React.useState<LinkedIdentity[] | null>(null)
  React.useEffect(() => {
    setUser(initialUser)
    setBilling(initialUser.billing)
    setUsage({ response_bytes: initialUser.response_bytes })
    setCustomGb(quotaGb(initialUser.billing))
  }, [initialUser])
  const save = async () => {
    if (!billing) return
    const response = await fetch(`/admin/api/users/${user.id}/billing`, { method: 'PUT', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ group_id: billing.group_id, quota_mode: billing.quota_mode, monthly_gb: billing.quota_mode === 'custom' ? Number(customGb) : null }) })
    if (response.ok) await reloadUsers()
  }
  const toggle = async () => {
    if (!await confirmAction({
      locale,
      title: user.disabled ? (locale === 'zh' ? '启用用户' : 'Enable user') : (locale === 'zh' ? '禁用用户' : 'Disable user'),
      message: user.disabled ? (locale === 'zh' ? `确定启用用户 ${user.username ?? user.email} 吗？` : `Enable ${user.username ?? user.email}?`) : (locale === 'zh' ? `确定禁用用户 ${user.username ?? user.email} 吗？该用户的登录会话会立即失效。` : `Disable ${user.username ?? user.email}? Their active sessions will be revoked immediately.`),
      confirmLabel: user.disabled ? (locale === 'zh' ? '启用用户' : 'Enable user') : (locale === 'zh' ? '禁用用户' : 'Disable user'),
      tone: user.disabled ? 'primary' : 'danger',
    })) return
    const response = await fetch(`/admin/api/users/${user.id}/status`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ disabled: !user.disabled }) })
    if (response.ok) setUser({ ...user, disabled: !user.disabled })
  }
  const rotate = async () => {
    if (!await confirmAction({ locale, title: locale === 'zh' ? '更换用户代理地址' : 'Rotate user routing address', message: locale === 'zh' ? `确定更换 ${user.username ?? user.email} 的专属代理地址吗？当前地址会立即失效。` : `Rotate the dedicated proxy address for ${user.username ?? user.email}? The current address will stop working immediately.`, confirmLabel: locale === 'zh' ? '确认更换' : 'Rotate address', tone: 'danger' })) return
    await fetch(`/admin/api/users/${user.id}/routing-id/rotate`, { method: 'POST' })
  }
  const loadIdentities = async () => { const response = await fetch(`/admin/api/users/${user.id}/identities`); if (response.ok) setIdentities(await response.json() as LinkedIdentity[]) }
  const unlink = async (identity: LinkedIdentity) => {
    if (!await confirmAction({ locale, title: locale === 'zh' ? '解除登录身份绑定' : 'Unlink identity', message: locale === 'zh' ? `确定解除 ${user.username ?? user.email} 与 ${identity.provider_name} 的绑定吗？` : `Unlink ${identity.provider_name} from ${user.username ?? user.email}?`, confirmLabel: locale === 'zh' ? '解除绑定' : 'Unlink identity', tone: 'danger' })) return
    const response = await fetch(`/admin/api/users/${user.id}/identities/${identity.id}`, { method: 'DELETE' }); if (response.ok) await loadIdentities()
  }
  const remove = async () => { if (!await confirmAction({ locale, title: locale === 'zh' ? '删除用户' : 'Delete user', message: locale === 'zh' ? `确定删除 ${user.username ?? user.email}？历史流量和审计记录会保留。` : `Soft-delete ${user.username ?? user.email}? Existing traffic and audit history will be retained.`, confirmLabel: locale === 'zh' ? '删除用户' : 'Delete user', tone: 'danger' })) return; const response = await fetch(`/admin/api/users/${user.id}`, { method: 'DELETE' }); if (response.ok) await reloadUsers() }
  return (
    <div className="admin-user-record">
      <div className="admin-user-summary">
        <div className="admin-user-identity">
          <strong>{user.display_name}</strong>
          <small>{user.username ?? user.email}</small>
          <code>{user.disabled ? (locale === 'zh' ? '已禁用' : 'disabled') : user.routing_id}{usage ? ` · ${byteLabel(usage.response_bytes)} ${locale === 'zh' ? '本月' : 'this month'}` : ''}</code>
        </div>
        {billing ? <div className="admin-user-controls">
          <div className="admin-user-fields">
            <label>{locale === 'zh' ? '计费组' : 'Billing group'}<select aria-label={`${user.username ?? user.email} billing group`} value={billing.group_id ?? ''} onChange={(event) => setBilling({ ...billing, group_id: event.target.value ? Number(event.target.value) : null })}><option value="">{locale === 'zh' ? '无计费组' : 'No billing group'}</option>{groups.map((group) => <option key={group.id} value={group.id}>{group.name}</option>)}</select></label>
            <label>{locale === 'zh' ? '用户配额' : 'User quota'}<select aria-label={`${user.username ?? user.email} quota mode`} value={billing.quota_mode} onChange={(event) => setBilling({ ...billing, quota_mode: event.target.value as UserBillingView['quota_mode'] })}><option value="default">{locale === 'zh' ? '默认配额' : 'Default quota'}</option><option value="unlimited">{locale === 'zh' ? '不限量' : 'Unlimited'}</option><option value="custom">{locale === 'zh' ? '自定义' : 'Custom'}</option></select></label>
            {billing.quota_mode === 'custom' ? <label>{locale === 'zh' ? '自定义 GB' : 'Custom GB'}<input required aria-label={`${user.username ?? user.email} custom quota`} min="0" type="number" value={customGb} onChange={(event) => setCustomGb(event.target.value)} /></label> : null}
          </div>
          <div className="admin-user-actions">
            <button className="primary-button compact-button" disabled={billing.quota_mode === 'custom' && customGb === ''} onClick={save}>{locale === 'zh' ? '保存配额' : 'Save billing'}</button>
            <button onClick={rotate}>{locale === 'zh' ? '更换子域名' : 'Rotate address'}</button>
            <button onClick={toggle}>{user.disabled ? (locale === 'zh' ? '启用' : 'Enable') : (locale === 'zh' ? '禁用' : 'Disable')}</button>
            <button onClick={loadIdentities}>{locale === 'zh' ? '登录身份' : 'Identities'}</button>
            <button className="danger-button compact-button" onClick={remove}>{locale === 'zh' ? '删除' : 'Delete'}</button>
          </div>
        </div> : null}
      </div>
      {identities ? <div className="admin-identity-detail">{identities.length ? identities.map((identity) => <span key={identity.id}><strong>{identity.provider_name}</strong><small>{identity.email ?? identity.provider_subject}</small><button className="revoke-button" onClick={() => unlink(identity)}>{locale === 'zh' ? '解除绑定' : 'Unlink'}</button></span>) : <small>{locale === 'zh' ? '未绑定第三方登录身份。' : 'No linked external identities.'}</small>}</div> : null}
    </div>
  )
}

export default function AdminBillingManagement({ locale, targets }: { locale: Locale; targets: readonly string[] }) {
  const [groups, setGroups] = React.useState<BillingGroupView[]>([])
  const [users, setUsers] = React.useState<UserSummary[]>([])
  const [name, setName] = React.useState('')
  const [quota, setQuota] = React.useState('')
  const [search, setSearch] = React.useState('')
  const [page, setPage] = React.useState(1)
  const [total, setTotal] = React.useState(0)
  const [error, setError] = React.useState(false)
  const pageSize = 10
  const load = React.useCallback(async (signal?: AbortSignal) => {
    const params = new URLSearchParams({ page: String(page), per_page: String(pageSize), search })
    try {
      const [groupResponse, userResponse] = await Promise.all([fetch('/admin/api/groups', { signal }), fetch(`/admin/api/users/summary?${params}`, { signal })])
      if (!groupResponse.ok || !userResponse.ok) throw new Error('Failed to load users')
      const [groupData, userData] = await Promise.all([groupResponse.json(), userResponse.json() as Promise<UserSummaryPage>])
      if (signal?.aborted) return
      setGroups(groupData as BillingGroupView[])
      setUsers(userData.items)
      setTotal(userData.total)
      setError(false)
      setPage((current) => Math.min(current, Math.max(1, Math.ceil(userData.total / pageSize))))
    } catch (error) {
      if (signal?.aborted) return
      setError(true)
      throw error
    }
  }, [page, search])
  React.useEffect(() => {
    const controller = new AbortController()
    load(controller.signal).catch(() => undefined)
    return () => controller.abort()
  }, [load])
  const create = async (event: React.FormEvent) => {
    event.preventDefault()
    const response = await fetch('/admin/api/groups', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ name, monthly_gb: quota === '' ? null : Number(quota) }) })
    if (response.ok) { setName(''); setQuota(''); await load() }
  }
  return <section className="admin-tab-panel settings-stack">{error ? <p role="alert">{locale === 'zh' ? '无法加载用户，请重试。' : 'Unable to load users. Please retry.'}<button onClick={() => { load().catch(() => undefined) }}>{locale === 'zh' ? '重试' : 'Retry'}</button></p> : null}<AdminLocalAccounts locale={locale} groups={groups} reload={load} /><div className="settings-card"><div className="settings-card-head"><h4>{locale === 'zh' ? '计费组' : 'Billing groups'}</h4><p>{locale === 'zh' ? '组内用户共享每月流量配额，每个用户只能属于一个计费组。' : 'Members share a monthly traffic quota; each user can belong to one billing group.'}</p></div><form className="compact-form inline-form" onSubmit={create}><label>{locale === 'zh' ? '组名称' : 'Group name'}<input required maxLength={80} value={name} onChange={(event) => setName(event.target.value)} /></label><label>{locale === 'zh' ? '共享月配额（GB）' : 'Shared monthly quota (GB)'}<input min="0" type="number" placeholder={locale === 'zh' ? '不限量' : 'Unlimited'} value={quota} onChange={(event) => setQuota(event.target.value)} /></label><button className="primary-button" type="submit">{locale === 'zh' ? '创建计费组' : 'Create billing group'}</button></form><div className="admin-account-list">{groups.map((group) => <BillingGroupRow key={group.id} group={group} locale={locale} reload={load} targets={targets} />)}</div></div><div className="settings-card"><div className="settings-card-head"><h4>{locale === 'zh' ? '用户' : 'Users'}</h4><label className="user-search">{locale === 'zh' ? '搜索用户' : 'Search users'}<input type="search" value={search} onChange={(event) => { setSearch(event.target.value); setPage(1) }} placeholder={locale === 'zh' ? '账号、邮箱、姓名或子域名' : 'Account, email, name, or routing ID'} /></label></div><div className="admin-account-list">{users.map((user) => <BillingUserRow key={user.id} initialUser={user} groups={groups} locale={locale} reloadUsers={load} />)}</div><Pagination page={page} total={total} pageSize={pageSize} locale={locale} onChange={setPage} /></div></section>
}
