import { request } from '../services/request'
import { Panel, SettingsForm, Field, Choice, Action } from '../components/controls'
import { useConfirmDialog } from '../confirm-dialog'
import { OperationToast } from '../operation-toast'
import * as React from 'react'
import { Save } from 'lucide-react'
import { type Locale, type SmtpView, type InvitationView, type OperationNotice } from '../types'
import { emailAdminError, responseError } from '../components/shared'

export function AdminEmailSettings({ locale }: { locale: Locale }) {
  const confirmAction = useConfirmDialog()
  const [smtp, setSmtp] = React.useState<SmtpView | null>(null)
  const [password, setPassword] = React.useState('')
  const [testRecipient, setTestRecipient] = React.useState('')
  const [invitations, setInvitations] = React.useState<InvitationView[]>([])
  const [inviteEmail, setInviteEmail] = React.useState('')
  const [mailNotice, setMailNotice] = React.useState<OperationNotice | null>(null)
  const [inviteNotice, setInviteNotice] = React.useState<OperationNotice | null>(null)
  const [savingSmtp, setSavingSmtp] = React.useState(false)
  const [testingSmtp, setTestingSmtp] = React.useState(false)
  const [sendingInvite, setSendingInvite] = React.useState(false)
  const load = React.useCallback(async () => {
    const [smtpResponse, invitationsResponse] = await Promise.all([request('/admin/api/smtp'), request('/admin/api/invitations')])
    if (smtpResponse.ok) {
      const value = await smtpResponse.json() as Partial<SmtpView>
      if (typeof value.host === 'string' && typeof value.port === 'number') setSmtp(value as SmtpView)
    }
    if (invitationsResponse.ok) {
      const value = await invitationsResponse.json() as unknown
      if (Array.isArray(value)) setInvitations(value as InvitationView[])
    }
  }, [])
  React.useEffect(() => { load().catch(() => undefined) }, [load])
  const saveSmtp = async (event: React.FormEvent) => {
    event.preventDefault(); if (!smtp) return
    setMailNotice(null)
    setSavingSmtp(true)
    try {
      const response = await request('/admin/api/smtp', { method: 'PUT', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ ...smtp, password: password || null }) })
      if (!response.ok) {
        setMailNotice({ tone: 'error', message: emailAdminError(await responseError(response), response.status, locale, locale === 'zh' ? '无法保存 SMTP 设置。' : 'Unable to save SMTP settings.') })
        return
      }
      setMailNotice({ tone: 'success', message: locale === 'zh' ? 'SMTP 设置已保存。现在可以发送测试邮件。' : 'SMTP settings saved. You can now send a test email.' })
      setPassword(''); await load()
    } catch {
      setMailNotice({ tone: 'error', message: locale === 'zh' ? '无法连接本地 MirrorProxy 服务。' : 'Could not reach the local MirrorProxy service.' })
    } finally { setSavingSmtp(false) }
  }
  const invite = async (event: React.FormEvent) => {
    event.preventDefault()
    setSendingInvite(true); setInviteNotice(null)
    try {
      const response = await request('/admin/api/invitations', { method: 'POST', headers: { 'content-type': 'application/json', 'x-mirrorproxy-locale': locale }, body: JSON.stringify({ email: inviteEmail, display_name: inviteEmail.split('@')[0] }) })
      if (!response.ok) { setInviteNotice({ tone: 'error', message: emailAdminError(await responseError(response), response.status, locale, locale === 'zh' ? '无法创建邀请。' : 'Unable to create invitation.') }); return }
      setInviteNotice({ tone: 'success', message: locale === 'zh' ? '邀请邮件已加入发送队列。' : 'Invitation queued for delivery.' })
      setInviteEmail(''); await load()
    } finally { setSendingInvite(false) }
  }
  const revoke = async (id: number) => {
    if (!await confirmAction({ locale, title: locale === 'zh' ? '撤销用户邀请' : 'Revoke invitation', message: locale === 'zh' ? '确定撤销这条邀请吗？邀请链接会立即失效。' : 'Revoke this invitation? Its link will stop working immediately.', confirmLabel: locale === 'zh' ? '撤销邀请' : 'Revoke invitation', tone: 'danger' })) return
    await request(`/admin/api/invitations/${id}`, { method: 'DELETE' }); await load()
  }
  const resend = async (id: number) => {
    const response = await request(`/admin/api/invitations/${id}/resend`, { method: 'POST', headers: { 'x-mirrorproxy-locale': locale } })
    setInviteNotice({ tone: response.ok ? 'success' : 'error', message: response.ok ? (locale === 'zh' ? '邀请邮件已重新加入队列。' : 'Invitation queued again.') : emailAdminError(await responseError(response), response.status, locale, locale === 'zh' ? '无法重新发送邀请。' : 'Unable to resend invitation.') })
    if (response.ok) await load()
  }
  const testSmtp = async (event: React.FormEvent) => {
    event.preventDefault()
    setTestingSmtp(true); setMailNotice(null)
    try {
      const response = await request('/admin/api/smtp/test', {
        method: 'POST', headers: { 'content-type': 'application/json', 'x-mirrorproxy-locale': locale }, body: JSON.stringify({ recipient: testRecipient }),
      })
      setMailNotice({ tone: response.ok ? 'success' : 'error', message: response.ok ? (locale === 'zh' ? '测试邮件已加入发送队列，请检查收件箱。' : 'Test email queued; check the recipient inbox.') : emailAdminError(await responseError(response), response.status, locale, locale === 'zh' ? '无法发送测试邮件。' : 'Unable to queue a test email.') })
    } finally { setTestingSmtp(false) }
  }
  if (!smtp) return null
  return (
    <section className="admin-tab-panel settings-stack">
      <Panel className="settings-card mail-settings-card"><div className="settings-card-head"><h4>{locale === 'zh' ? '发件服务器' : 'Mail server'}</h4><p>{locale === 'zh' ? '用于发送 Magic Link、备用验证码和用户邀请。SMTP 密码将直接保存到本地数据库。' : 'Used for magic links, fallback codes, and user invitations. The SMTP password is stored directly in the local database.'}</p></div><SettingsForm className="compact-form" onSubmit={saveSmtp}>
        <label>SMTP {locale === 'zh' ? '主机' : 'host'}<Field required={smtp.enabled} value={smtp.host} onChange={(event) => setSmtp({ ...smtp, host: event.target.value })} /></label>
        <label>{locale === 'zh' ? '端口' : 'Port'}<Field type="number" min="1" max="65535" value={smtp.port} onChange={(event) => setSmtp({ ...smtp, port: Number(event.target.value) })} /></label>
        <label>{locale === 'zh' ? '加密方式' : 'Security'}<Choice value={smtp.security} onChange={(event) => setSmtp({ ...smtp, security: event.target.value })}><option value="starttls">STARTTLS</option><option value="smtps">SMTPS</option><option value="none">{locale === 'zh' ? '不加密' : 'None'}</option></Choice></label>
        <label>{locale === 'zh' ? '用户名' : 'Username'}<Field value={smtp.username ?? ''} onChange={(event) => setSmtp({ ...smtp, username: event.target.value || null })} /></label>
        <label>{locale === 'zh' ? '密码' : 'Password'}<Field type="password" placeholder={smtp.has_password ? (locale === 'zh' ? '已保存，留空表示不修改' : 'Saved; leave blank to keep') : ''} value={password} onChange={(event) => setPassword(event.target.value)} /></label>
        <label>{locale === 'zh' ? '发件人名称' : 'From name'}<Field required maxLength={100} value={smtp.from_name} onChange={(event) => setSmtp({ ...smtp, from_name: event.target.value })} /></label>
        <label>{locale === 'zh' ? '发件邮箱' : 'From address'}<Field required={smtp.enabled} type="email" value={smtp.from_address} onChange={(event) => setSmtp({ ...smtp, from_address: event.target.value })} /></label>
        <label className="toggle-field"><Field type="checkbox" checked={smtp.enabled} onChange={(event) => setSmtp({ ...smtp, enabled: event.target.checked })} />{locale === 'zh' ? '启用邮件发送' : 'Enable email delivery'}</label>
        <Action className="primary-button mail-save-button" disabled={savingSmtp} type="submit"><Save size={16} />{savingSmtp ? (locale === 'zh' ? '保存中…' : 'Saving…') : (locale === 'zh' ? '保存发件设置' : 'Save mail settings')}</Action>
      </SettingsForm><OperationToast locale={locale} notice={mailNotice} /><SettingsForm className="compact-form inline-form mail-test-form" onSubmit={testSmtp}><label>{locale === 'zh' ? '测试收件人' : 'Test recipient'}<Field required type="email" value={testRecipient} onChange={(event) => setTestRecipient(event.target.value)} /></label><Action className="secondary-button" disabled={testingSmtp} type="submit">{testingSmtp ? (locale === 'zh' ? '发送中…' : 'Sending…') : (locale === 'zh' ? '发送测试邮件' : 'Send test email')}</Action></SettingsForm></Panel>
      <Panel className="settings-card"><div className="settings-card-head"><h4>{locale === 'zh' ? '邀请用户' : 'Invite users'}</h4><p>{locale === 'zh' ? '填写邮箱即可发送邀请，用户点击邮件中的 Magic Link 完成首次登录。' : 'Enter an email address to invite a user. They complete their first sign-in through the email magic link.'}</p></div><SettingsForm className="compact-form invite-form" onSubmit={invite}><label>{locale === 'zh' ? '邀请邮箱' : 'Email'}<Field required type="email" value={inviteEmail} onChange={(event) => setInviteEmail(event.target.value)} placeholder="name@example.com" /></label><Action className="primary-button" disabled={sendingInvite} type="submit">{sendingInvite ? (locale === 'zh' ? '发送中…' : 'Sending…') : (locale === 'zh' ? '发送邀请' : 'Send invitation')}</Action></SettingsForm><OperationToast locale={locale} notice={inviteNotice} /></Panel>
      <div className="invitation-history-head"><h4>{locale === 'zh' ? '最近邀请' : 'Recent invitations'}</h4><small>{locale === 'zh' ? '仅展示最近 3 天，最多 10 条。' : 'Showing up to 10 invitations from the last 3 days.'}</small></div>
      <div className="admin-account-list">{invitations.map((invitation) => (
        <div className="admin-account-row" key={invitation.id}>
          <span><strong>{invitation.email}</strong><small>{invitation.status === 'pending' ? (locale === 'zh' ? '待接受' : 'pending') : invitation.status} · {new Date(invitation.expires_at * 1000).toLocaleString()}</small></span>
          {invitation.status === 'pending' ? <span><Action className="secondary-button compact-button" onClick={() => resend(invitation.id)}>{locale === 'zh' ? '重新发送' : 'Resend'}</Action><Action className="revoke-button" onClick={() => revoke(invitation.id)}>{locale === 'zh' ? '撤销' : 'Revoke'}</Action></span> : null}
        </div>
      ))}{invitations.length === 0 ? <p className="empty-stat">{locale === 'zh' ? '最近 3 天没有邀请记录。' : 'No invitations in the last 3 days.'}</p> : null}</div>
    </section>
  )
}
