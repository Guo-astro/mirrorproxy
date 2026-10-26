import * as React from 'react'
import { Modal } from 'antd'
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
  const resolverRef = React.useRef<((confirmed: boolean) => void) | null>(null)

  const settle = React.useCallback((confirmed: boolean) => {
    setRequest(null)
    resolverRef.current?.(confirmed)
    resolverRef.current = null
  }, [])

  const confirmAction = React.useCallback<ConfirmDialogContextValue>((nextRequest) => new Promise((resolve) => {
    resolverRef.current?.(false)
    resolverRef.current = resolve
    setRequest(nextRequest)
  }), [])

  React.useEffect(() => () => resolverRef.current?.(false), [])

  const locale = request?.locale ?? 'en'
  const danger = request?.tone === 'danger'
  return <ConfirmDialogContext.Provider value={confirmAction}>
    {children}
    <Modal open={Boolean(request)} title={request?.title ?? (danger ? (locale === 'zh' ? '确认敏感操作' : 'Confirm sensitive action') : (locale === 'zh' ? '确认操作' : 'Confirm action'))}
      onCancel={() => settle(false)} onOk={() => settle(true)}
      okText={request?.confirmLabel ?? (locale === 'zh' ? '继续' : 'Continue')}
      cancelText={locale === 'zh' ? '取消' : 'Cancel'} okButtonProps={{ danger }} focusable={{ focusTriggerAfterClose: true }} cancelButtonProps={{ autoFocus: true }} destroyOnHidden>
      <p>{request?.message}</p>
    </Modal>
  </ConfirmDialogContext.Provider>
}
