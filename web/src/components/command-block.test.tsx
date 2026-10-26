import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import { CommandBlock, commandTokens } from './command-block'
afterEach(cleanup)
it('preserves shell quoting, newlines, URLs and PowerShell literals exactly', () => {
  for (const value of ["curl -fsSL 'https://mirror.example/install.sh' | sh -s -- --setup --base-url 'https://mirror.example'", "& ([scriptblock]::Create((irm 'https://it''s.example/install.ps1'))) -Mirror 'https://it''s.example'", 'services:\n  app:\n    image: ghcr.io/org/image:latest\n# note']) {
    expect(commandTokens(value).map(token => token.text).join('')).toBe(value)
  }
})
it('renders untrusted command text without interpreting HTML and highlights copy feedback', () => {
  const value = 'npm config set registry https://mirror.example/npm/; echo "<img src=x onerror=alert(1)>"'
  const copy = vi.fn()
  const props = { value, copied: false, copyLabel: 'Copy', copiedLabel: 'Copied', onCopy: copy }
  const { container, rerender } = render(<CommandBlock {...props} />)
  expect(container.querySelector('code')?.textContent).toBe(value)
  expect(container.querySelector('img, script')).toBeNull()
  expect(container.querySelector('.command-token-keyword')?.textContent).toBe('npm')
  fireEvent.click(screen.getByRole('button', { name: 'Copy' }))
  expect(copy).toHaveBeenCalledOnce()
  rerender(<CommandBlock {...props} copied />)
  expect(screen.getByRole('button', { name: 'Copied' }).classList.contains('copied')).toBe(true)
})
