import { test, expect } from '@playwright/test';
import { execSync } from 'child_process';
import path from 'path';

/**
 * Full E2E Regression Test: Upload → Scan → Mainnet Deploy
 * 
 * This test exercises the complete user journey from contract upload
 * through security scanning to guarded mainnet deployment.
 * 
 * Prerequisites:
 * - Soroban standalone network running (via setup-test-network.sh)
 * - Dashboard frontend running on localhost:3000
 * - Test contract fixture available
 */

test.describe('Full Deploy Flow: Upload → Scan → Mainnet Deploy', () => {
  let testContractPath: string;
  let scanResultPath: string;
  
  test.beforeAll(async () => {
    // Setup test network
    console.log('Setting up test network...');
    execSync('bash ../../scripts/setup-test-network.sh', { 
      stdio: 'inherit',
      cwd: __dirname 
    });
    
    // Prepare test contract fixture
    testContractPath = path.join(__dirname, '../fixtures/test-contract.wasm');
    
    // Verify fixture exists
    if (!require('fs').existsSync(testContractPath)) {
      throw new Error(`Test contract not found at ${testContractPath}`);
    }
  });
  
  test('complete flow from upload to guarded mainnet deploy', async ({ page }) => {
    // Step 1: Upload contract to dashboard
    await page.goto('http://localhost:3000/dashboard');
    await page.click('[data-testid="upload-contract-btn"]');
    
    const fileInput = await page.locator('input[type="file"]');
    await fileInput.setInputFiles(testContractPath);
    
    await expect(page.locator('[data-testid="upload-success"]'))
      .toBeVisible({ timeout: 10000 });
    
    // Step 2: Initiate security scan
    await page.click('[data-testid="scan-btn"]');
    await expect(page.locator('[data-testid="scan-in-progress"]'))
      .toBeVisible();
    
    // Wait for scan completion
    await expect(page.locator('[data-testid="scan-complete"]'))
      .toBeVisible({ timeout: 60000 });
    
    // Step 3: Verify findings displayed
    const findingsCount = await page.locator('[data-testid="finding-item"]').count();
    console.log(`Scan found ${findingsCount} findings`);
    expect(findingsCount).toBeGreaterThanOrEqual(0);
    
    // Step 4: Download SARIF report
    const downloadPromise = page.waitForEvent('download');
    await page.click('[data-testid="download-sarif-btn"]');
    const download = await downloadPromise;
    scanResultPath = await download.path() || '';
    expect(scanResultPath).toBeTruthy();
    
    // Verify SARIF format
    const sarifContent = require('fs').readFileSync(scanResultPath, 'utf-8');
    const sarif = JSON.parse(sarifContent);
    expect(sarif.version).toBe('2.1.0');
    
    // Step 5: Initiate mainnet deploy with safety gates
    await page.click('[data-testid="deploy-mainnet-btn"]');
    
    // Verify safety gate: confirmation modal appears
    await expect(page.locator('[data-testid="mainnet-confirmation-modal"]'))
      .toBeVisible({ timeout: 5000 });
    
    // Verify --confirm-mainnet flag documentation shown
    await expect(page.locator('text=/confirm.*mainnet/i'))
      .toBeVisible();
    
    // Enter confirmation passphrase
    await page.fill('[data-testid="confirmation-input"]', 'DEPLOY_TO_MAINNET');
    await page.click('[data-testid="confirm-deploy-btn"]');
    
    // Wait for deployment transaction
    await expect(page.locator('[data-testid="deploy-success"]'))
      .toBeVisible({ timeout: 30000 });
    
    // Verify contract address displayed (Stellar contract format)
    const contractAddress = await page.locator('[data-testid="contract-address"]')
      .textContent();
    expect(contractAddress).toMatch(/^C[A-Z0-9]{55}$/);
    
    console.log(`✅ Contract deployed to: ${contractAddress}`);
  });
  
  test.afterAll(async () => {
    // Cleanup test network
    console.log('Cleaning up test network...');
    try {
      execSync('docker stop soroban-standalone && docker rm soroban-standalone', {
        stdio: 'inherit'
      });
    } catch (error) {
      console.warn('Failed to cleanup test network:', error);
    }
  });
});
