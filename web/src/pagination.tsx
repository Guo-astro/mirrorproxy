type Locale = 'en' | 'zh'
export function Pagination({ page, total, pageSize, locale, onChange }: { page: number; total: number; pageSize: number; locale: Locale; onChange: (page: number) => void }) {
  const pageCount = Math.max(1, Math.ceil(total / pageSize))
  if (total <= pageSize) return null
  return <nav className="pagination" aria-label={locale === 'zh' ? '分页' : 'Pagination'}><span>{locale === 'zh' ? `第 ${page} / ${pageCount} 页，共 ${total} 条` : `Page ${page} of ${pageCount} · ${total} items`}</span><div><button disabled={page <= 1} onClick={() => onChange(page - 1)}>{locale === 'zh' ? '上一页' : 'Previous'}</button><button disabled={page >= pageCount} onClick={() => onChange(page + 1)}>{locale === 'zh' ? '下一页' : 'Next'}</button></div></nav>
}
