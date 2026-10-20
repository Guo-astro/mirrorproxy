import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import { OperationToast, ToastProvider } from './operation-toast'

afterEach(() => { cleanup(); vi.useRealTimers() })

it('floats feedback outside the form, replaces earlier notices, and supports dismissal', () => {
  const onClose = vi.fn()
  const { container, rerender } = render(<ToastProvider><form><OperationToast locale="zh" notice={{ tone: 'success', message: '用户已创建' }} /></form></ToastProvider>)
  expect(screen.getByRole('status').textContent).toContain('用户已创建')
  expect(container.querySelector('.admin-toast')).toBeNull()
  rerender(<ToastProvider><form><OperationToast locale="zh" notice={{ tone: 'error', message: '保存失败' }} onClose={onClose} /></form></ToastProvider>)
  expect(screen.queryByRole('status')).toBeNull()
  expect(screen.getByRole('alert').textContent).toContain('保存失败')
  fireEvent.click(screen.getByRole('button', { name: '关闭提示' }))
  expect(screen.queryByRole('alert')).toBeNull()
  expect(onClose).toHaveBeenCalledOnce()
})

it('auto-dismisses and pauses while hovered', () => {
  vi.useFakeTimers()
  render(<ToastProvider><OperationToast locale="en" notice={{ tone: 'success', message: 'Saved' }} /></ToastProvider>)
  fireEvent.mouseEnter(screen.getByRole('status'))
  act(() => vi.advanceTimersByTime(5000))
  expect(screen.getByRole('status')).toBeTruthy()
  fireEvent.mouseLeave(screen.getByRole('status'))
  act(() => vi.advanceTimersByTime(4500))
  expect(screen.queryByRole('status')).toBeNull()
})

it('keeps undo available and invokes the latest action after a rerender', () => {
  vi.useFakeTimers()
  const previous = vi.fn()
  const latest = vi.fn()
  const { rerender } = render(<ToastProvider><OperationToast locale="en" notice={{ tone: 'success', message: 'Source removed', action: { label: 'Undo', run: previous } }} /></ToastProvider>)
  act(() => vi.advanceTimersByTime(60000))
  expect(screen.getByRole('button', { name: 'Undo' })).toBeTruthy()
  rerender(<ToastProvider><OperationToast locale="en" notice={{ tone: 'success', message: 'Source removed', action: { label: 'Undo', run: latest } }} /></ToastProvider>)
  fireEvent.click(screen.getByRole('button', { name: 'Undo' }))
  expect(latest).toHaveBeenCalledOnce()
  expect(previous).not.toHaveBeenCalled()
  expect(screen.queryByRole('status')).toBeNull()
})
