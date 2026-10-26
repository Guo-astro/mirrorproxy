import * as React from 'react'
import { Check, Clipboard } from 'lucide-react'
import { Action } from './controls'

type Token = { text: string; kind: 'text' | 'string' | 'url' | 'variable' | 'flag' | 'keyword' | 'operator' | 'comment' }
/** Highlight text as React nodes only; templates are never parsed as markup. */
export function commandTokens(value: string): Token[] {
  const pattern = /"(?:\\.|[^"\\])*"|'(?:''|[^'])*'|https?:\/\/[^\s"'<>]+|#[^\n]*|\$(?:env:)?[A-Za-z_][\w:]*|--?[A-Za-z][\w-]*|\b(?:curl|sh|bash|mirrorproxy|docker|npm|npx|yarn|pnpm|pip|cargo|go|irm|iex|Invoke-RestMethod|Invoke-WebRequest|FROM|RUN|ENV|COPY|Create|scriptblock)\b|[|;&=]/g
  const tokens: Token[] = []
  let end = 0
  for (const match of value.matchAll(pattern)) {
    const offset = match.index!
    if (offset > end) tokens.push({ text: value.slice(end, offset), kind: 'text' })
    const text = match[0]
    const kind = /^["']/.test(text) ? 'string' : text.startsWith('http') ? 'url' : text.startsWith('#') ? 'comment' : text.startsWith('$') ? 'variable' : text.startsWith('-') ? 'flag' : /^[|;&=]$/.test(text) ? 'operator' : 'keyword'
    tokens.push({ text, kind }); end = offset + text.length
  }
  if (end < value.length) tokens.push({ text: value.slice(end), kind: 'text' })
  return tokens
}
export function CommandBlock({ value, title, language = 'Shell', copied, copyLabel, copiedLabel, onCopy, className = '' }: {
  value: string; title?: string; language?: string; copied: boolean; copyLabel: string; copiedLabel: string; onCopy: () => void; className?: string
}) {
  return <div className={`consumer-command ${className}${copied ? ' is-copied' : ''}`}>
    <div className="consumer-command-toolbar"><span>{title ?? language}</span><Action type="button" className={copied ? 'command-copy copied' : 'command-copy'} onClick={onCopy}>{copied ? <Check size={15} aria-hidden="true" /> : <Clipboard size={15} aria-hidden="true" />}{copied ? copiedLabel : copyLabel}</Action></div>
    <pre className="consumer-command-scroll" tabIndex={0} aria-label={title ?? language}><code>{commandTokens(value).map((token, index) => <span className={`command-token-${token.kind}`} key={index}>{token.text}</span>)}</code></pre>
  </div>
}
