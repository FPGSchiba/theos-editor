import { expect, test } from '@playwright/test';

import { replay } from './support/replay';

test('the app loads with the grid-noir theme on its root', async ({ page }) => {
  await replay(page, { os: 'windows', use: ['project_last-none'] });
  await page.goto('/');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'grid-noir');
  await expect(page.locator('#app .tu-shell')).toBeVisible();
});
