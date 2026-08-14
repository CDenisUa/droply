// Core
import { test, expect } from '@playwright/test'

test('Droply shell loads with the paste-URL form', async ({ page }) => {
  await page.goto('/')

  await expect(page.getByRole('heading', { name: 'Droply' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Analyze' })).toBeVisible()
  await expect(page.getByLabel('Developed by Chepio')).toBeVisible()
})

test('typing a URL enables the Analyze button', async ({ page }) => {
  await page.goto('/')

  const analyzeButton = page.getByRole('button', { name: 'Analyze' })
  await expect(analyzeButton).toBeDisabled()

  await page.getByLabel('URL to analyze').fill('https://example.com/file.mp4')
  await expect(analyzeButton).toBeEnabled()
})

test('the Downloads page is reachable from the nav', async ({ page }) => {
  await page.goto('/')

  await page.getByRole('link', { name: 'Downloads' }).click()

  await expect(page.getByRole('heading', { name: 'Downloads' })).toBeVisible()
})
