import { test, expect } from '@playwright/test';
import * as path from 'path';
import * as fs from 'fs';

test.describe('Full Scan Flow', () => {
  test('upload contract, view findings, and download SARIF', async ({ page }) => {
    await page.goto('/');

    // Upload contract
    const contractPath = path.join(__dirname, '../../../../contracts/fixtures/auth_gap_contract.rs');
    // Ensure the fixture exists
    expect(fs.existsSync(contractPath)).toBeTruthy();
    
    // Upload the file
    const fileChooserPromise = page.waitForEvent('filechooser');
    await page.click('text=Upload Contract'); // Replace with actual selector if different
    const fileChooser = await fileChooserPromise;
    await fileChooser.setFiles(contractPath);

    // Wait for findings to appear
    await expect(page.locator('text=S001')).toBeVisible({ timeout: 15000 });
    
    // Download SARIF
    const downloadPromise = page.waitForEvent('download');
    await page.click('text=Download SARIF'); // Replace with actual selector if different
    const download = await downloadPromise;
    
    // Wait for the download process to complete and save the downloaded file somewhere
    const downloadPath = await download.path();
    expect(downloadPath).toBeTruthy();
    
    if (downloadPath) {
      const sarifContent = fs.readFileSync(downloadPath, 'utf8');
      const sarifData = JSON.parse(sarifContent);
      
      // Verify S001 finding is in SARIF
      const hasS001 = sarifData.runs.some((run: any) => 
        run.results.some((result: any) => result.ruleId === 'S001')
      );
      expect(hasS001).toBeTruthy();
    }
  });
});
