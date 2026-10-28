import { Alert, Button, Skeleton } from 'antd'
import { type Locale } from '../types'

export function ResourceStatus({ loading, error, hasData, retry, locale, message }: {
  loading: boolean; error: boolean; hasData: boolean; retry: () => Promise<void>; locale: Locale; message: string
}) {
  return <>
    {loading && !hasData ? <div className="resource-loading" role="status" aria-label={locale === 'zh' ? '加载中' : 'Loading'}><Skeleton active paragraph={{ rows: 2 }} /></div> : null}
    {error ? <Alert className="resource-error" type="error" role="alert" showIcon title={message} action={<Button onClick={() => void retry()}>{locale === 'zh' ? '重试' : 'Retry'}</Button>} /> : null}
  </>
}
