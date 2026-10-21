import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { SourceCapabilityBadges, SourceCapabilityDetails, UpstreamSelectionDetails, type SourceCapabilities } from './source-details'

afterEach(cleanup)

const conditional: SourceCapabilities = {
  cache: 'conditional', cache_enabled: false, authentication: 'operator_controlled',
  client_authorization_forwarding: false, clients: ['npm', 'Yarn', 'pnpm'], limitations: [],
}

describe('source capabilities', () => {
  it('distinguishes configured cache state and authorization opt-in', () => {
    const { rerender } = render(<SourceCapabilityDetails capabilities={conditional} locale="en" />)
    expect(screen.getByText(/Cache off/)).toBeTruthy()
    expect(screen.getByText(/Client Authorization and cookies are not forwarded/)).toBeTruthy()
    rerender(<SourceCapabilityDetails capabilities={{ ...conditional, cache_enabled: true, client_authorization_forwarding: true }} locale="en" />)
    expect(screen.getByText(/Conditional cache/)).toBeTruthy()
    expect(screen.getByText(/forwarding is enabled for supported requests/)).toBeTruthy()
    rerender(<SourceCapabilityBadges capabilities={{ ...conditional, cache: 'bypass', authentication: 'public_only' }} locale="zh" />)
    expect(screen.getByText('不缓存')).toBeTruthy()
    expect(screen.getByText('仅公开资源')).toBeTruthy()
  })

  it('keeps guidance separate from proxying and tolerates older catalog responses', () => {
    const { rerender, container } = render(<SourceCapabilityDetails locale="en" />)
    expect(container.textContent).toBe('')
    rerender(<SourceCapabilityDetails capabilities={{ ...conditional, cache: 'not_applicable', authentication: 'not_applicable' }} locale="en" />)
    expect(screen.getByText('Setup guidance')).toBeTruthy()
    expect(screen.getByText(/client and destination handle authentication/)).toBeTruthy()
  })
})

describe('upstream selection explanations', () => {
  it('renders empty state, preserves server ordering, and localizes circuit reasons', () => {
    const { rerender } = render(<UpstreamSelectionDetails locale="en" />)
    expect(screen.getByText(/No selection samples yet/)).toBeTruthy()
    const report = { strategy: 'adaptive', samples: [{ observed_at: 1792540800, endpoints: [
      { origin: 'https://fast.example', configured_position: 2, rank: 1, reason: 'latency', consecutive_failures: 0, cooldown_remaining_secs: 0, latency_ewma_ms: 20.4 },
      { origin: 'https://failed.example', configured_position: 1, rank: 2, reason: 'cooldown', consecutive_failures: 3, cooldown_remaining_secs: 45, latency_ewma_ms: null },
    ] }] }
    rerender(<UpstreamSelectionDetails report={report} locale="zh" />)
    expect(screen.getByText('冷却期内降低优先级')).toBeTruthy()
    expect(screen.getByText(/刷新时冷却剩余约 45s/)).toBeTruthy()
    expect(screen.getAllByRole('listitem')[0].textContent).toContain('https://fast.example')
    expect(screen.getByText(/不代表最终成功的上游/)).toBeTruthy()
    rerender(<UpstreamSelectionDetails report={{ ...report, strategy: 'ordered' }} locale="en" />)
    expect(screen.getByText('Ordered')).toBeTruthy()
    expect(screen.queryByText(/Cooldown remaining at refresh:/)).toBeNull()
  })
})
