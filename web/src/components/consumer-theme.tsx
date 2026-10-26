import * as React from 'react'
import { ConfigProvider, theme } from 'antd'
import { AppearanceContext } from '../providers'
import { ConfirmDialogProvider } from '../confirm-dialog'
import { ToastProvider } from '../operation-toast'
import '../consumer.css'

/** User-facing appearance only. Administration retains its native Pro theme. */
export function ConsumerTheme({ children }: { children: React.ReactNode }) {
  const { dark } = React.useContext(AppearanceContext)
  return <ConfigProvider theme={{
    cssVar: {},
    algorithm: dark ? theme.darkAlgorithm : theme.defaultAlgorithm,
    token: {
      colorPrimary: '#0066cc',
      colorBgContainer: dark ? '#1d1d1f' : '#ffffff',
      colorBgElevated: dark ? '#1d1d1f' : '#ffffff',
      colorText: dark ? '#f5f5f7' : '#1d1d1f',
      colorTextSecondary: dark ? '#b6b6bd' : '#6e6e73',
      colorBorder: dark ? '#424245' : '#d2d2d7',
      colorFillAlter: dark ? '#272729' : '#f5f5f7',
      borderRadius: 12, controlHeight: 48, fontSize: 15,
      fontFamily: '"SF Pro Text", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
      boxShadow: 'none', boxShadowSecondary: 'none',
    },
    components: { Button: { borderRadius: 999, primaryShadow: 'none', defaultShadow: 'none', dangerShadow: 'none', fontWeight: 400 }, Input: { paddingInline: 16 } },
  }}>
    <div className="consumer-ui"><ToastProvider><ConfirmDialogProvider>{children}</ConfirmDialogProvider></ToastProvider></div>
  </ConfigProvider>
}
