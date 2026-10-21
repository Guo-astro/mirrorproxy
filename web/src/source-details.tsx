export type SourceCapabilities = {
  cache: 'conditional' | 'bypass' | 'not_applicable'
  cache_enabled: boolean
  authentication: 'public_only' | 'operator_controlled' | 'not_applicable'
  client_authorization_forwarding: boolean
  clients: string[]
  limitations: string[]
}

type Locale = 'en' | 'zh'

function capabilityLabels(value: SourceCapabilities, locale: Locale) {
  const zh = locale === 'zh'
  return {
    cache: value.cache === 'conditional'
      ? value.cache_enabled ? (zh ? '按条件缓存' : 'Conditional cache') : (zh ? '缓存未启用' : 'Cache off')
      : value.cache === 'bypass' ? (zh ? '不缓存' : 'No server cache') : (zh ? '仅配置指导' : 'Setup guidance'),
    auth: value.authentication === 'public_only' ? (zh ? '仅公开资源' : 'Public resources only')
      : value.authentication === 'operator_controlled' ? (zh ? '认证由管理员控制' : 'Operator-controlled auth')
        : (zh ? '由客户端连接目标源' : 'Client connects to source'),
  }
}

export function SourceCapabilityBadges({ capabilities, locale }: { capabilities?: SourceCapabilities; locale: Locale }) {
  if (!capabilities) return null
  const labels = capabilityLabels(capabilities, locale)
  return <span className="source-capabilities" aria-label={locale === 'zh' ? '源能力' : 'Source capabilities'}>
    <span>{labels.cache}</span><span>{labels.auth}</span>
    <span className="source-clients">{capabilities.clients.join(' · ')}</span>
  </span>
}

export function SourceCapabilityDetails({ capabilities, locale }: { capabilities?: SourceCapabilities; locale: Locale }) {
  if (!capabilities) return null
  const zh = locale === 'zh'
  const labels = capabilityLabels(capabilities, locale)
  const auth = capabilities.authentication === 'public_only'
    ? (zh ? '不转发客户端 Authorization 或 Cookie，也不使用服务端上游凭据。' : 'Client Authorization and cookies are blocked; server upstream credentials are not used.')
    : capabilities.authentication === 'operator_controlled'
      ? capabilities.client_authorization_forwarding
        ? (zh ? '已开启客户端 Authorization 透传，适用于协议支持的请求；服务端上游凭据优先。Cookie 不转发。' : 'Client Authorization forwarding is enabled for supported requests; server upstream credentials take precedence. Cookies are not forwarded.')
        : (zh ? '客户端 Authorization 和 Cookie 不透传；管理员可单独配置服务端上游凭据。' : 'Client Authorization and cookies are not forwarded; administrators can configure server upstream credentials separately.')
      : (zh ? '此条目提供配置指导，认证由客户端与目标源处理。' : 'This entry provides setup guidance; the client and destination handle authentication.')
  return <dl className="source-capability-details">
    <div><dt>{zh ? '服务端缓存' : 'Server cache'}</dt><dd>{labels.cache}{capabilities.cache === 'conditional' ? (zh ? '；仅符合大小、响应头等缓存规则的请求可缓存，启用不代表每次命中。' : '; only requests meeting size, header, and other cache rules qualify. Enabled does not guarantee a hit.') : ''}</dd></div>
    <div><dt>{zh ? '上游认证' : 'Upstream authentication'}</dt><dd>{auth}</dd></div>
    <div><dt>{zh ? '客户端' : 'Clients'}</dt><dd>{capabilities.clients.join(' · ')}</dd></div>
    {capabilities.limitations.includes('no_native_xet') ? <div><dt>{zh ? '传输限制' : 'Transport limits'}</dt><dd>{zh ? '仅 HTTP 下载，不支持原生 Xet；设置 HF_HUB_DISABLE_XET=1。' : 'HTTP downloads only; native Xet is unsupported. Set HF_HUB_DISABLE_XET=1.'}</dd></div> : null}
  </dl>
}

export type SelectionReport = {
  strategy: string
  samples: {
    observed_at: number
    endpoints: {
      origin: string
      configured_position: number
      rank: number
      reason: string
      consecutive_failures: number
      cooldown_remaining_secs: number
      latency_ewma_ms: number | null
    }[]
  }[]
}

export function UpstreamSelectionDetails({ report, locale }: { report?: SelectionReport; locale: Locale }) {
  const zh = locale === 'zh'
  const reasons: Record<string, string> = zh
    ? { configured_order: '保持配置顺序', cooldown: '冷却期内降低优先级', failures: '连续失败降低优先级', latency: '根据平滑响应延迟排序', unobserved: '暂无请求结果，以配置位置评分' }
    : { configured_order: 'Configured order', cooldown: 'Deprioritized during cooldown', failures: 'Deprioritized after failures', latency: 'Ranked by smoothed response latency', unobserved: 'No request result; scored by configured position' }
  return <section className="selection-details" aria-labelledby="selection-details-title">
    <div className="selection-heading"><h4 id="selection-details-title">{zh ? '上游排序与熔断原因' : 'Upstream ranking and circuit state'}</h4>
      {report ? <span>{report.strategy === 'adaptive' ? (zh ? '自适应' : 'Adaptive') : (zh ? '顺序优先' : 'Ordered')}</span> : null}</div>
    <p>{zh ? '按最近请求样本和当前状态计算候选顺序，不代表最终成功的上游；不同路径可能有不同排序。冷却会降低优先级，不会完全禁用上游。' : 'Candidate order uses recent request samples and current state, not the final successful upstream; paths may rank differently. Cooldown lowers priority rather than disabling an endpoint.'}</p>
    {report?.samples.length ? report.samples.map((sample, index) => <details className="selection-sample" key={sample.endpoints.map((endpoint) => `${endpoint.configured_position}:${endpoint.origin}`).sort().join('|')} open={index === 0}>
      <summary>{sample.endpoints[0]?.origin} · {zh ? '请求样本' : 'Request sample'} {index + 1}<time dateTime={new Date(sample.observed_at * 1000).toISOString()}>{new Date(sample.observed_at * 1000).toLocaleString(zh ? 'zh-CN' : 'en-US')}</time></summary>
      <ol>{sample.endpoints.map((endpoint) => <li key={endpoint.configured_position}>
        <span className="selection-rank">#{endpoint.rank}</span><div><code>{endpoint.origin}</code><strong>{reasons[endpoint.reason] ?? endpoint.reason}</strong>
          <small>{zh ? '配置位置' : 'Configured position'} #{endpoint.configured_position} · {zh ? '连续失败' : 'Consecutive failures'} {endpoint.consecutive_failures} · {zh ? '平滑延迟' : 'Smoothed latency'} {endpoint.latency_ewma_ms === null ? '—' : `${Math.round(endpoint.latency_ewma_ms)} ms`}
            {report.strategy === 'adaptive' && endpoint.cooldown_remaining_secs > 0 ? ` · ${zh ? '刷新时冷却剩余约' : 'Cooldown remaining at refresh:'} ${endpoint.cooldown_remaining_secs}s` : ''}</small></div>
      </li>)}</ol>
    </details>) : <p className="selection-empty">{zh ? '尚无选源请求样本。通过本站发起下载后刷新查看；仅记录经过选源器的请求。' : 'No selection samples yet. Make a download through this instance, then refresh; only requests using the selector are recorded.'}</p>}
  </section>
}
