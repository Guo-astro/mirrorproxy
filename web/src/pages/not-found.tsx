import { Button, Result } from 'antd'
export default function NotFound() {
  return <Result status="404" title="404" subTitle="Page not found / 页面不存在" extra={<Button type="primary" href="/">MirrorProxy</Button>} />
}
