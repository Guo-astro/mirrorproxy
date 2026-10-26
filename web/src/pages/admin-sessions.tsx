import { Panel } from '../components/controls'
import { request } from '../services/request'
import { Action } from '../components/controls'
import { useConfirmDialog } from '../confirm-dialog'
import * as React from 'react'
import { type Locale } from '../types'

export type AdminSessionView = { id: string; auth_method: string; created_at: number; expires_at: number; last_used_at: number; current: boolean }

export function AdminSessionManagement({ locale, onCurrentRevoked }: { locale: Locale; onCurrentRevoked: () => void }) {
  const confirmAction = useConfirmDialog()
  const [sessions, setSessions] = React.useState<AdminSessionView[]>([])
  const load = React.useCallback(async () => {
    const response = await request('/admin/api/auth/sessions')
    if (response.ok) {
      const value = await response.json() as unknown
      if (Array.isArray(value)) setSessions(value as AdminSessionView[])
    }
  }, [])
  React.useEffect(() => { load().catch(() => undefined) }, [load])
  const revoke = async (session: AdminSessionView) => {
    if (!await confirmAction({
      locale,
      title: locale === 'zh' ? '撤销管理员会话' : 'Revoke administrator session',
      message: locale === 'zh' ? `确定撤销${session.current ? '当前' : '这个'}管理员会话吗？${session.current ? '撤销后需要重新登录。' : ''}` : `Revoke ${session.current ? 'the current' : 'this'} administrator session?${session.current ? ' You will need to sign in again.' : ''}`,
      confirmLabel: locale === 'zh' ? '撤销会话' : 'Revoke session',
      tone: 'danger',
    })) return
    const response = await request(`/admin/api/auth/sessions/${session.id}`, { method: 'DELETE' })
    if (!response.ok) return
    if (session.current) onCurrentRevoked(); else await load()
  }
  return (
    <Panel className="settings-card session-card">
      <div className="settings-card-head">
        <h4>{locale === 'zh' ? '登录会话' : 'Administrator sessions'}</h4>
        <p>{locale === 'zh' ? '如果发现陌生设备，可立即撤销对应会话。' : 'Revoke any session you do not recognize.'}</p>
      </div>
      <div className="admin-account-list">{sessions.map((session) => (
        <div className="admin-account-row" key={session.id}>
          <span><strong>{session.auth_method === 'passkey' ? 'Passkey' : (locale === 'zh' ? '密码' : 'Password')}{session.current ? ` · ${locale === 'zh' ? '当前会话' : 'current'}` : ''}</strong><small>{locale === 'zh' ? '最近使用' : 'Last used'} {new Date(session.last_used_at * 1000).toLocaleString()} · {locale === 'zh' ? '过期时间' : 'expires'} {new Date(session.expires_at * 1000).toLocaleString()}</small></span>
          <Action className={session.current ? 'danger-button session-revoke' : 'revoke-button'} onClick={() => revoke(session)}>{locale === 'zh' ? '撤销' : 'Revoke'}</Action>
        </div>
      ))}</div>
    </Panel>
  )
}
