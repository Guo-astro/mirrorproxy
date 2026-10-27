import { Tag, Typography } from 'antd'
import { ProCard, StatisticCard, ProTable } from '@ant-design/pro-components'
import { UpstreamSelectionDetails } from '../source-details'
import * as React from 'react'
import { Database } from 'lucide-react'
import { type SourceHealthReport, type Locale } from '../types'

export function AdminSourceHealthPanel({ report, locale }: { report: SourceHealthReport | null; locale: Locale }) {
  const labels = locale === 'zh'
    ? { healthy: '可用', degraded: '部分异常', unhealthy: '不可用', disabled: '未启用', unknown: '未检测', source: '镜像源', adapter: '适配器', result: '检测结果', latency: '耗时', checked: '检测时间', upstreams: '个上游', empty: '尚无检测结果，点击“立即检测”开始检查全部镜像源。', auto: '系统每 15 分钟逐个检测所有已配置上游；前台状态会同步更新。' }
    : { healthy: 'Available', degraded: 'Degraded', unhealthy: 'Unavailable', disabled: 'Disabled', unknown: 'Not checked', source: 'Source', adapter: 'Adapter', result: 'Result', latency: 'Latency', checked: 'Checked', upstreams: 'upstreams', empty: 'No results yet. Run a check to probe every configured upstream.', auto: 'Every configured upstream is checked individually every 15 minutes; the public catalog follows the latest result.' }
  const items = React.useMemo(() => [...(report?.items ?? [])].sort((left, right) => {
    const rank: Record<string, number> = { unhealthy: 0, degraded: 1, healthy: 2, disabled: 3 }
    return (rank[left.status] ?? 3) - (rank[right.status] ?? 3) || left.target_code.localeCompare(right.target_code)
  }), [report])
  const checkedAt = report?.last_checked_at ? new Date(report.last_checked_at * 1000).toLocaleString(locale === 'zh' ? 'zh-CN' : 'en-US') : labels.unknown
  return <section className="admin-tab-panel source-health-panel">
    <ProCard className="source-health-summary" title={locale === 'zh' ? '镜像可用性' : 'Mirror availability'}>
      <p>{labels.auto}</p><Typography.Text type="secondary">{labels.checked}: {checkedAt}</Typography.Text>
      <div className="source-health-counts">{(['healthy', 'degraded', 'unhealthy', 'disabled', 'unknown'] as const).map(status => <StatisticCard key={status} statistic={{ title: <Tag color={healthColor(status)}>{labels[status]}</Tag>, value: report?.[status] ?? 0 }} />)}</div>
    </ProCard>
    <ProCard><UpstreamSelectionDetails report={report?.selection} locale={locale} /></ProCard>
    {items.length ? <div className="source-health-table-wrap"><ProTable rowKey="target_code" search={false} options={false} pagination={false} dataSource={items} tableLayout="fixed" columns={[
      { title: labels.source, dataIndex: 'target_code', width: '42%', render: (_, item) => <div className="health-source"><strong>{item.target_code}</strong>{item.error ? <p className="health-source-error">{item.error}</p> : null}{item.endpoints.length ? <details className="source-endpoint-details"><summary>{item.endpoints.length} {labels.upstreams}</summary><div>{item.endpoints.map(endpoint => <article className={endpoint.status} key={`${endpoint.position}-${endpoint.endpoint}`}><Tag color={healthColor(endpoint.status)}>{labels[endpoint.status]}</Tag><code>{endpoint.endpoint}</code><small>HTTP {endpoint.http_status ?? '—'} · {endpoint.latency_ms ?? '—'} ms {endpoint.error}</small></article>)}</div></details> : null}</div> },
      { title: labels.adapter, dataIndex: 'adapter' },
      { title: labels.result, render: (_, item) => <Tag color={healthColor(item.status)}>{labels[item.status]}</Tag> },
      { title: labels.latency, render: (_, item) => item.latency_ms === null ? '—' : `${item.latency_ms} ms` },
      { title: labels.checked, render: (_, item) => new Date(item.checked_at * 1000).toLocaleTimeString(locale === 'zh' ? 'zh-CN' : 'en-US') },
    ]} /></div> : <div className="source-health-empty"><Database size={24} /><p>{labels.empty}</p></div>}
  </section>
}

function healthColor(status: string) { return ({ healthy: 'success', degraded: 'warning', unhealthy: 'error', disabled: 'default', unknown: 'default' } as Record<string, string>)[status] }
