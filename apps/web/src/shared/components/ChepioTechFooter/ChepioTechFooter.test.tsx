// Core
import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
// Components
import ChepioTechFooter from '@/shared/components/ChepioTechFooter/ChepioTechFooter'

describe('ChepioTechFooter', () => {
  it('links to chepio.tech and opens in a new tab', () => {
    render(<ChepioTechFooter />)

    const link = screen.getByRole('link', { name: /developed by chepio/i })
    expect(link).toHaveAttribute('href', 'https://chepio.tech')
    expect(link).toHaveAttribute('target', '_blank')
    expect(link).toHaveAttribute('rel', 'noopener noreferrer')
  })

  it('renders the chepio.tech logo', () => {
    render(<ChepioTechFooter />)

    const logo = screen.getByAltText('chepio.tech')
    expect(logo).toHaveAttribute('src', '/images/chepio-tech/logo_designed.svg')
  })
})
