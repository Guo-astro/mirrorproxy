// Testable application entry; Umi owns the browser entry and route chunks.
import * as React from 'react'
import { ApplicationProviders } from './providers'
import { PublicApp } from './pages/portal'
import { UserPage } from './pages/account'
import { AdminPage } from './pages/admin'
export { HttpSecurityBanner } from './components/shared'
export { agentSetupCommands, containerRegistryInputTemplate, normalizeContainerImage, rewriteContainerConfig, sourceManualCommand } from './pages/portal'
export function App() {
  const path = window.location.pathname
  const page = path === '/admin' || path.startsWith('/admin/') ? <AdminPage /> : path === '/login' || path === '/account' || path.startsWith('/account/') ? <UserPage /> : <PublicApp />
  return <ApplicationProviders>{page}</ApplicationProviders>
}
