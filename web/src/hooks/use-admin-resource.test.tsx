import { act, cleanup, renderHook, waitFor } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import { decodeList, useAdminResource } from './use-admin-resource'

afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.unstubAllGlobals() })
const list = decodeList<string>
const response = (value: unknown) => new Response(JSON.stringify(value))

it('distinguishes invalid payloads and HTTP errors from an empty list, then retries', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(new Response('', { status: 503 })).mockResolvedValueOnce(response({})).mockResolvedValueOnce(response([])))
  const { result } = renderHook(() => useAdminResource('/admin/api/example', list))
  await waitFor(() => expect(result.current.error).toBe(true))
  expect(result.current.data).toBeNull()
  await act(() => result.current.reload())
  expect(result.current.error).toBe(true)
  expect(result.current.data).toBeNull()
  await act(() => result.current.reload())
  expect(result.current).toMatchObject({ data: [], error: false, loading: false })
})

it('retains loaded data and edits when a refresh fails over the network', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(response(['loaded'])).mockRejectedValueOnce(new TypeError('network failure')))
  const { result } = renderHook(() => useAdminResource('/admin/api/example', list))
  await waitFor(() => expect(result.current.data).toEqual(['loaded']))
  act(() => result.current.setData(['local edit']))
  await act(() => result.current.reload())
  expect(result.current).toMatchObject({ data: ['local edit'], error: true, loading: false })
})

it('prevents a slow previous request from overwriting the latest result', async () => {
  let finishFirst!: (value: Response) => void
  vi.stubGlobal('fetch', vi.fn().mockImplementationOnce(() => new Promise<Response>(resolve => { finishFirst = resolve })).mockResolvedValueOnce(response(['latest'])))
  const { result } = renderHook(() => useAdminResource('/admin/api/example', list))
  expect(result.current.loading).toBe(true)
  await act(() => result.current.reload())
  await act(async () => { finishFirst(response(['stale'])) })
  expect(result.current.data).toEqual(['latest'])
})
