import * as React from 'react'
import { Button, Checkbox, Input, Select } from 'antd'
import { ProCard, ProForm } from '@ant-design/pro-components'

/** Shared controls preserve native form validation and the API-facing value types. */
export function Action({ className = '', type, children, color: _color, ...props }: React.ButtonHTMLAttributes<HTMLButtonElement>) {
  const primary = className.includes('primary-button') || type === 'submit'
  const danger = /danger-button|revoke-button/.test(className)
  return <Button {...props} autoInsertSpace={false} htmlType={type ?? 'submit'} type={primary || danger ? 'primary' : 'default'} danger={danger} className={className}>{children}</Button>
}

export function Field(props: React.InputHTMLAttributes<HTMLInputElement>) {
  if (props.type === 'checkbox') {
    const { type: _type, onChange, value: _value, ...rest } = props
    return <Checkbox {...rest} onChange={event => onChange?.(event as unknown as React.ChangeEvent<HTMLInputElement>)} />
  }
  return <ProForm.Item noStyle><Input {...props} size={undefined} /></ProForm.Item>
}
export function TextArea(props: React.TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <Input.TextArea {...props} />
}

type Option = { value: string; label: React.ReactNode; disabled?: boolean }
function options(children: React.ReactNode): Option[] {
  return React.Children.toArray(children).flatMap(child => {
    if (!React.isValidElement<{ value?: string | number; disabled?: boolean; children?: React.ReactNode }>(child)) return []
    if (child.type === 'option') return [{ value: String(child.props.value ?? ''), label: child.props.children, disabled: child.props.disabled }]
    return options(child.props.children)
  })
}
export function Choice({ children, onChange, value, defaultValue, required, className, ...props }: React.SelectHTMLAttributes<HTMLSelectElement>) {
  const id = React.useId()
  const controlId = props.id ?? id
  const [label, setLabel] = React.useState<string>()
  React.useLayoutEffect(() => {
    const element = document.getElementById(controlId)
    const parent = element?.closest('label')
    if (!parent || props['aria-label']) return
    const text = Array.from(parent.childNodes).filter(node => node.nodeType === Node.TEXT_NODE).map(node => node.textContent).join('').trim()
    if (text) setLabel(text)
  }, [controlId, props['aria-label']])
  const selected = value === undefined ? undefined : String(value)
  return <span className="choice-field">
    <Select virtual={false} id={controlId} aria-label={props['aria-label'] ?? label} aria-labelledby={props['aria-labelledby']} aria-required={required} disabled={props.disabled} className={className} value={selected} defaultValue={defaultValue === undefined ? undefined : String(defaultValue)} options={options(children)}
      onChange={next => onChange?.({ target: { value: next }, currentTarget: { value: next } } as React.ChangeEvent<HTMLSelectElement>)} />
    {required ? <input className="choice-validation" aria-hidden="true" tabIndex={-1} value={selected ?? ''} required onChange={() => undefined} onInvalid={event => { event.preventDefault(); document.getElementById(controlId)?.focus() }} /> : null}
  </span>
}
export function SettingsForm({ children, className = '', ...props }: React.FormHTMLAttributes<HTMLFormElement>) {
  const classes = className.split(' ')
  if (classes.includes('settings-card')) {
    const entries = React.Children.toArray(children)
    const first = entries[0]
    const hasHead = React.isValidElement<{ className?: string }>(first) && first.props.className === 'settings-card-head'
    return <Panel className="settings-card">{hasHead ? first : null}<form {...props} className={classes.filter(value => value !== 'settings-card').join(' ')}><ProForm component={false} submitter={false}>{hasHead ? entries.slice(1) : children}</ProForm></form></Panel>
  }
  return <form {...props} className={className}><ProForm component={false} submitter={false}>{children}</ProForm></form>
}
export function Panel({ children, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  const entries = React.Children.toArray(children)
  const head = entries[0]
  if (React.isValidElement<{ className?: string; children?: React.ReactNode }>(head) && head.props.className === 'settings-card-head') {
    const rawParts = React.Children.toArray(head.props.children)
    const parts = rawParts.flatMap(part => React.isValidElement<{ children?: React.ReactNode }>(part) && part.type === 'div' ? React.Children.toArray(part.props.children) : [part])
    const heading = parts.find(part => React.isValidElement(part) && part.type === 'h4')
    const description = parts.find(part => React.isValidElement(part) && part.type === 'p')
    if (React.isValidElement<{ children?: React.ReactNode }>(heading)) {
      return <ProCard {...props} title={<h4 className="pro-card-title">{heading.props.children}</h4>} extra={parts.filter(part => part !== heading && part !== description)}>
        {description ? <div className="pro-card-description">{description}</div> : null}{entries.slice(1)}
      </ProCard>
    }
  }
  return <ProCard {...props}>{children}</ProCard>
}
