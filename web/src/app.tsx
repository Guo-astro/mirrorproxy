import * as React from 'react'
import { ApplicationProviders } from './providers'
import { initializeRequests, request } from './services/request'
initializeRequests()
export const rootContainer = (container: React.ReactNode) => <ApplicationProviders>{container}</ApplicationProviders>
export async function getInitialState() {
  const sessions = await Promise.allSettled(['/api/auth/session', '/admin/api/auth/session'].map(async path => {
    const response = await request(path)
    return response.ok ? response.json() : null
  }))
  return { user: sessions[0].status === 'fulfilled' ? sessions[0].value : null, admin: sessions[1].status === 'fulfilled' ? sessions[1].value : null }
}
