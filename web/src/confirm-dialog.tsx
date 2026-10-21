import * as React from 'react'
import { CircleAlert } from 'lucide-react'
type Locale = 'en' | 'zh'
type ConfirmDialogRequest = {
  locale: Locale
  message: string
  title?: string
  confirmLabel?: string
  tone?: 'primary' | 'danger'
}
type ConfirmDialogContextValue = (request: ConfirmDialogRequest) => Promise<boolean>

const ConfirmDialogContext = React.createContext<ConfirmDialogContextValue | null>(null)

export function useConfirmDialog() {
  const confirmAction = React.useContext(ConfirmDialogContext)
  if (!confirmAction) throw new Error('useConfirmDialog must be used inside ConfirmDialogProvider')
  return confirmAction
}

export function ConfirmDialogProvider({ children }: { children: React.ReactNode }) {
  const [request, setRequest] = React.useState<ConfirmDialogRequest | null>(null)
  const dialogRef = React.useRef<HTMLDialogElement>(null)
  const resolverRef = React.useRef<((confirmed: boolean) => void) | null>(null)

  const settle = React.useCallback((confirmed: boolean) => {
    const dialog = dialogRef.current
    if (dialog?.open && typeof dialog.close === 'function') dialog.close()
    dialog?.removeAttribute('open')
    setRequest(null)
    resolverRef.current?.(confirmed)
    resolverRef.current = null
  }, [])

  const confirmAction = React.useCallback<ConfirmDialogContextValue>((nextRequest) => new Promise((resolve) => {
    resolverRef.current?.(false)
    resolverRef.current = resolve
    setRequest(nextRequest)
  }), [])

  React.useEffect(() => {
    const dialog = dialogRef.current
    if (!dialog || !request || dialog.open) return
    if (typeof dialog.showModal === 'function') {
      try { dialog.showModal() } catch { dialog.setAttribute('open', '') }
    }
    if (!dialog.open) dialog.setAttribute('open', '')
  }, [request])

  React.useEffect(() => () => resolverRef.current?.(false), [])

  const locale = request?.locale ?? 'en'
  const danger = request?.tone === 'danger'
  return <ConfirmDialogContext.Provider value={confirmAction}>
    {children}
    <dialog
      aria-labelledby="app-confirm-title"
      aria-describedby="app-confirm-message"
      className={`app-confirm-dialog${danger ? ' app-confirm-dialog-danger' : ''}`}
      ref={dialogRef}
      onCancel={(event) => { event.preventDefault(); settle(false) }}
      onMouseDown={(event) => { if (event.target === event.currentTarget) settle(false) }}
    >
      <div className="app-confirm-panel">
        <span className="app-confirm-icon" aria-hidden="true"><CircleAlert size={20} /></span>
        <div className="app-confirm-copy">
          <h2 id="app-confirm-title">{request?.title ?? (danger ? (locale === 'zh' ? '确认敏感操作' : 'Confirm sensitive action') : (locale === 'zh' ? '确认操作' : 'Confirm action'))}</h2>
          <p id="app-confirm-message">{request?.message}</p>
        </div>
        <div className="app-confirm-actions">
          <button autoFocus className="secondary-button" type="button" onClick={() => settle(false)}>{locale === 'zh' ? '取消' : 'Cancel'}</button>
          <button className={danger ? 'danger-button' : 'primary-button'} type="button" onClick={() => settle(true)}>{request?.confirmLabel ?? (locale === 'zh' ? '继续' : 'Continue')}</button>
        </div>
      </div>
    </dialog>
  </ConfirmDialogContext.Provider>
}
