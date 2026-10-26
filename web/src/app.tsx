import * as React from 'react'
import { ApplicationProviders } from './providers'
import { initializeRequests } from './services/request'
initializeRequests()
export const rootContainer = (container: React.ReactNode) => <ApplicationProviders>{container}</ApplicationProviders>
