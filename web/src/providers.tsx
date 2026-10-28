import * as React from 'react'
import { App as AntApp, ConfigProvider, theme } from 'antd'
import enUS from 'antd/locale/en_US'
import zhCN from 'antd/locale/zh_CN'
import { ConfirmDialogProvider } from './confirm-dialog'
import { ToastProvider } from './operation-toast'
import { readStoredPreference } from './preferences'

export const AppearanceContext = React.createContext({ dark: false, locale: 'en' })

export function ApplicationProviders({ children }: { children: React.ReactNode }) {
  const read = () => ({ dark: document.documentElement.dataset.theme === 'dark', locale: readStoredPreference(localStorage, 'mirrorproxy.locale', 'en', ['en', 'zh']) })
  const [preferences, setPreferences] = React.useState(read)
  React.useEffect(() => {
    const sync = () => setPreferences(read())
    const observer = new MutationObserver(sync)
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme', 'lang', 'data-locale'] })
    window.addEventListener('storage', sync)
    return () => { observer.disconnect(); window.removeEventListener('storage', sync) }
  }, [])
  const rootStyle = getComputedStyle(document.documentElement)
  const token = (name: string, fallback: string) => rootStyle.getPropertyValue(name).trim() || fallback
  return <AppearanceContext.Provider value={preferences}><ConfigProvider locale={preferences.locale === 'zh' ? zhCN : enUS} theme={{ cssVar: {}, algorithm: preferences.dark ? theme.darkAlgorithm : theme.defaultAlgorithm, components: { Button: { colorError: token('--control-danger', theme.defaultSeed.colorError), colorErrorHover: token('--control-danger-hover', theme.defaultSeed.colorError), colorErrorActive: token('--control-danger-active', theme.defaultSeed.colorError) } }, token: { colorPrimary: token('--brand-primary', theme.defaultSeed.colorPrimary), fontFamily: token('--font-ui', theme.defaultSeed.fontFamily) } }}>
    <AntApp><ToastProvider><ConfirmDialogProvider>{children}</ConfirmDialogProvider></ToastProvider></AntApp>
  </ConfigProvider></AppearanceContext.Provider>
}
