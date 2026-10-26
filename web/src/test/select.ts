import userEvent from '@testing-library/user-event'
import { screen } from '@testing-library/react'
/** Exercise the actual Ant Design popup, including keyboard-accessible options. */
export async function choose(control: HTMLElement, label: string) {
  await userEvent.click(control)
  await userEvent.click(await screen.findByRole('option', { name: label }))
}
