import * as React from 'react'
import { createPortal } from 'react-dom'
import { X } from 'lucide-react'
import { Alert, Button } from 'antd'

type Locale = 'en' | 'zh'
export type OperationNotice = { tone: 'success' | 'error'; message: string; title?: string; action?: { label: string; run: () => void } }
type ToastEntry = { id: string; notice: OperationNotice; locale: Locale; onDismiss?: () => void }
const ToastContext = React.createContext<{ show: (entry: ToastEntry) => void; dismiss: (id: string) => void } | null>(null)

function ToastCard({ entry, onClose }: { entry: ToastEntry; onClose: () => void }) {
  const { notice, locale } = entry
  const [paused, setPaused] = React.useState(false)
  const success = notice.tone === 'success'
  React.useEffect(() => {
    if (paused || notice.action) return
    const timer = window.setTimeout(onClose, success ? 4500 : 9000)
    return () => window.clearTimeout(timer)
  }, [entry, onClose, paused, success, notice.action])
  return <div onFocus={() => setPaused(true)} onBlur={event => { if (!event.currentTarget.contains(event.relatedTarget)) setPaused(false) }}><Alert className={`admin-toast admin-toast-${notice.tone}`} role={success ? 'status' : 'alert'} type={notice.tone} showIcon
    onMouseEnter={() => setPaused(true)} onMouseLeave={() => setPaused(false)}
    title={notice.title || (locale === 'zh' ? (success ? '操作成功' : '操作失败') : (success ? 'Success' : 'Unable to complete operation'))}
    description={<span className="admin-toast-copy"><span>{notice.message}</span>{notice.action ? <Button type="link" className="toast-action" onClick={() => { notice.action!.run(); onClose() }}>{notice.action.label}</Button> : null}</span>}
    action={<Button type="text" onClick={onClose} aria-label={locale === 'zh' ? '关闭提示' : 'Dismiss notification'} icon={<X size={16} />} />} /></div>
}

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [entry, setEntry] = React.useState<ToastEntry | null>(null)
  const show = React.useCallback((next: ToastEntry) => setEntry(next), [])
  const dismiss = React.useCallback((id: string) => setEntry(current => current?.id === id ? null : current), [])
  const context = React.useMemo(() => ({ show, dismiss }), [show, dismiss])
  const close = React.useCallback(() => { if (entry) { dismiss(entry.id); entry.onDismiss?.() } }, [entry, dismiss])
  return <ToastContext.Provider value={context}>{children}{entry ? createPortal(<div className="toast-region"><ToastCard key={entry.id + entry.notice.message} entry={entry} onClose={close} /></div>, document.body) : null}</ToastContext.Provider>
}

/** Presents operation feedback outside the form without moving focus or blocking work. */
export function OperationToast({ notice, locale, onClose }: { notice: OperationNotice | null; locale: Locale; onClose?: () => void }) {
  const context = React.useContext(ToastContext)
  const id = React.useId()
  const [localEntry, setLocalEntry] = React.useState<ToastEntry | null>(null)
  const callback = React.useRef(onClose)
  callback.current = onClose
  const message = notice?.message
  const tone = notice?.tone
  const title = notice?.title
  const action = React.useRef(notice?.action)
  action.current = notice?.action
  const show = context?.show
  const dismiss = context?.dismiss
  React.useEffect(() => {
    if (!message || !tone) { dismiss?.(id); setLocalEntry(null); return }
    const entry: ToastEntry = { id, locale, onDismiss: () => callback.current?.(), notice: { message, tone, title, action: action.current ? { label: action.current.label, run: () => action.current?.run() } : undefined } }
    if (show) show(entry); else setLocalEntry(entry)
    return () => dismiss?.(id)
  }, [id, locale, message, tone, title, show, dismiss])
  const close = React.useCallback(() => { setLocalEntry(null); callback.current?.() }, [])
  if (context || !localEntry) return null
  return createPortal(<div className="toast-region"><ToastCard entry={localEntry} onClose={close} /></div>, document.body)
}
