import { Alert } from 'antd'
import { CirclePause } from 'lucide-react'
import { type Locale, type PublicConfig } from '../types'

export function isPublicQuotaBlocked(quota: PublicConfig['quota']) {
  return quota.enabled && (quota.status === 'exhausted' || quota.status === 'reserved')
}

export function PublicQuotaNotice({ quota, locale }: { quota: PublicConfig['quota']; locale: Locale }) {
  if (!isPublicQuotaBlocked(quota)) return null
  const zh = locale === 'zh'
  const exhausted = quota.status === 'exhausted'
  const throttled = quota.on_exceeded === 'throttle'
  const timezone = quota.timezone === 'local' ? (zh ? '服务器本地时间' : 'server local time') : quota.timezone
  return <Alert
    className="public-quota-notice"
    type="error"
    role="alert"
    aria-labelledby="public-quota-title"
    showIcon
    icon={<CirclePause size={28} aria-hidden="true" />}
    title={<><span className="public-quota-eyebrow">{zh ? '服务提示' : 'Service notice'}</span><h2 id="public-quota-title">{exhausted ? (zh ? '本月流量额度已用尽' : 'Monthly traffic limit reached') : (zh ? '代理流量额度暂不可用' : 'Proxy traffic allowance is temporarily unavailable')}</h2></>}
    description={<>
      <p>{exhausted
        ? (zh ? `本站代理${throttled ? '请求正在限流' : '下载已暂停'}。请于下月再试，或联系站点管理员。` : `Proxy ${throttled ? 'requests are rate limited' : 'downloads are paused'}. Please try again next month or contact the site operator.`)
        : (zh ? '本站代理请求暂时受限，请稍后重试或联系站点管理员。' : 'Proxy requests are temporarily limited. Please retry shortly or contact the site operator.')}</p>
      <p className="public-quota-detail">{quota.month ? `${quota.month} · ` : ''}{quota.monthly_gb.toLocaleString(zh ? 'zh-CN' : 'en-US')} GB{zh ? '/月' : ' / month'}<span aria-hidden="true"> · </span>{zh ? `每月 1 日按${timezone}启用新额度` : `New allowance on the 1st of each month (${timezone})`}</p>
    </>}
  />
}
