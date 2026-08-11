// Core
import { test, expect } from '@playwright/test'

test('Droply shell loads with the paste-URL form', async ({ page }) => {
  await page.goto('/')

  await expect(page.getByRole('heading', { name: 'Droply' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Analyze' })).toBeVisible()
  await expect(page.getByLabel('Developed by Chepio')).toBeVisible()
})
