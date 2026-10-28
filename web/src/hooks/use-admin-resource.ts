import * as React from 'react'
import { request } from '../services/request'

/** Keeps cached data visible on refresh failure; only the latest request may commit. */
export function useAdminResource<T>(path: string, decode: (value: unknown) => T) {
  const [data, setData] = React.useState<T | null>(null)
  const [loading, setLoading] = React.useState(true)
  const [error, setError] = React.useState(false)
  const sequence = React.useRef(0)
  const reload = React.useCallback(async () => {
    const current = ++sequence.current
    setLoading(true)
    setError(false)
    try {
      const response = await request(path)
      if (!response.ok) throw new Error('resource unavailable')
      const next = decode(await response.json())
      if (current === sequence.current) setData(next)
    } catch {
      if (current === sequence.current) setError(true)
    } finally {
      if (current === sequence.current) setLoading(false)
    }
  }, [path, decode])
  React.useEffect(() => {
    void reload()
    return () => { sequence.current++ }
  }, [reload])
  return { data, setData, loading, error, reload }
}

export function decodeList<T>(value: unknown): T[] {
  if (!Array.isArray(value)) throw new Error('invalid list')
  return value as T[]
}
