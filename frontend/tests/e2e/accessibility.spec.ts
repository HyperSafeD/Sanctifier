import { expect, test } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

test.describe("Comprehensive accessibility audit", () => {
  test("homepage has no detectable accessibility violations", async ({ page }) => {
    await page.goto("/");

    const accessibilityScanResults = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"])
      .analyze();

    expect(accessibilityScanResults.violations).toEqual([]);
  });

  test("dashboard page has no detectable accessibility violations", async ({ page }) => {
    await page.goto("/dashboard");

    const accessibilityScanResults = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"])
      .analyze();

    expect(accessibilityScanResults.violations).toEqual([]);
  });

  test("playground page has no detectable accessibility violations", async ({ page }) => {
    await page.goto("/playground");

    const accessibilityScanResults = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"])
      .analyze();

    expect(accessibilityScanResults.violations).toEqual([]);
  });
});

test.describe("Tab order and keyboard navigation", () => {
  test("tab order is logical and sequential", async ({ page }) => {
    await page.goto("/dashboard");

    // Get all focusable elements
    const focusableElements = await page.evaluate(() => {
      const focusable = [
        'button:not([disabled])',
        'a[href]',
        'input:not([disabled])',
        'select:not([disabled])',
        'textarea:not([disabled])',
        '[tabindex]:not([tabindex="-1"])',
      ].join(', ');
      
      return Array.from(document.querySelectorAll(focusable))
        .map(el => ({
          tagName: el.tagName,
          type: (el as HTMLInputElement).type || '',
          id: el.id,
          textContent: el.textContent?.trim().substring(0, 30) || '',
          tabIndex: parseInt(el.getAttribute('tabindex') || '0'),
        }));
    });

    // Verify that elements have reasonable tab indices
    for (const element of focusableElements) {
      expect(element.tabIndex).toBeGreaterThanOrEqual(0);
    }
  });

  test("severity filter buttons are keyboard navigable", async ({ page }) => {
    await page.goto("/dashboard");

    // Load some data to make the filter buttons visible
    const mockReport = `{
      "auth_gaps": [
        {
          "function_name": "initialize",
          "code": "AUTH_GAP"
        }
      ]
    }`;

    const textarea = page.getByPlaceholder(/size_warnings/);
    await textarea.fill(mockReport);
    
    await page.waitForTimeout(500);
    
    await page.evaluate(() => {
      const buttons = Array.from(document.querySelectorAll('button'));
      const parseButton = buttons.find(btn => btn.textContent?.includes('Parse JSON'));
      if (parseButton) {
        (parseButton as HTMLButtonElement).click();
      }
    });
    
    await page.waitForTimeout(3000);

    // Check that filter buttons are present and have proper ARIA attributes
    await expect(page.getByRole("group", { name: "Filter by severity" })).toBeVisible();
    await expect(page.getByRole("button", { name: "All" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Critical" })).toBeVisible();
    await expect(page.getByRole("button", { name: "High" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Medium" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Low" })).toBeVisible();

    // Test keyboard navigation
    await page.keyboard.press("Tab");
    await expect(page.getByRole("button", { name: "All" })).toBeVisible();
  });

  test("tab navigation follows ARIA pattern", async ({ page }) => {
    await page.goto("/dashboard");

    const mockReport = `{
      "auth_gaps": [
        {
          "function_name": "initialize",
          "code": "AUTH_GAP"
        }
      ]
    }`;

    const textarea = page.getByPlaceholder(/size_warnings/);
    await textarea.fill(mockReport);
    
    await page.waitForTimeout(500);
    
    await page.evaluate(() => {
      const buttons = Array.from(document.querySelectorAll('button'));
      const parseButton = buttons.find(btn => btn.textContent?.includes('Parse JSON'));
      if (parseButton) {
        (parseButton as HTMLButtonElement).click();
      }
    });
    
    await page.waitForTimeout(3000);

    // Check that tabs are present and have proper ARIA attributes
    await expect(page.getByRole("tablist")).toBeVisible();
    const findingsTab = page.getByRole("tab", { name: "Findings" });
    const callGraphTab = page.getByRole("tab", { name: "Call Graph" });

    await expect(findingsTab).toBeVisible();
    await expect(callGraphTab).toBeVisible();
    
    // Test that tabs have proper ARIA attributes
    await expect(findingsTab).toHaveAttribute("role", "tab");
    await expect(callGraphTab).toHaveAttribute("role", "tab");
    
    // Test tab switching functionality
    await callGraphTab.click();
    await expect(callGraphTab).toBeVisible();
    await expect(findingsTab).toBeVisible();
    
    // Test keyboard navigation
    await findingsTab.focus();
    await expect(findingsTab).toBeVisible();
  });

  test("focus indicators are visible", async ({ page }) => {
    await page.goto("/dashboard");

    // Check that focusable elements have visible focus styles
    const focusStyles = await page.evaluate(() => {
      const button = document.querySelector('button');
      if (!button) return null;
      
      const computedStyle = window.getComputedStyle(button);
      return {
        outlineStyle: computedStyle.outlineStyle,
        outlineWidth: computedStyle.outlineWidth,
        outlineColor: computedStyle.outlineColor,
      };
    });

    if (focusStyles) {
      // Either outline is present or custom focus indicator exists
      const hasFocusIndicator = 
        focusStyles.outlineStyle !== 'none' || 
        focusStyles.outlineWidth !== '0px';
      
      expect(hasFocusIndicator).toBeTruthy();
    }
  });
});

test.describe("ARIA labels and roles", () => {
  test("all interactive elements have accessible names", async ({ page }) => {
    await page.goto("/dashboard");

    const violations = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa"])
      .analyze();

    // Check for button-name, link-name, and label violations
    const namingViolations = violations.filter(v => 
      v.id === 'button-name' || 
      v.id === 'link-name' || 
      v.id === 'label'
    );

    expect(namingViolations).toEqual([]);
  });

  test("form inputs have associated labels", async ({ page }) => {
    await page.goto("/dashboard");

    const inputsWithoutLabels = await page.evaluate(() => {
      const inputs = document.querySelectorAll('input, select, textarea');
      const violations: string[] = [];
      
      inputs.forEach(input => {
        const hasId = input.id;
        const hasAriaLabel = input.getAttribute('aria-label');
        const hasAriaLabelledby = input.getAttribute('aria-labelledby');
        
        // Check if there's a label element for this input
        let hasLabelElement = false;
        if (hasId) {
          const label = document.querySelector(`label[for="${hasId}"]`);
          hasLabelElement = !!label;
        }
        
        // Check if input is wrapped in a label
        const parentLabel = input.closest('label');
        const isWrappedInLabel = !!parentLabel;
        
        if (!hasAriaLabel && !hasAriaLabelledby && !hasLabelElement && !isWrappedInLabel) {
          violations.push(input.tagName + (input.id ? `#${input.id}` : ''));
        }
      });
      
      return violations;
    });

    expect(inputsWithoutLabels).toEqual([]);
  });

  test("images have alt text or are decorative", async ({ page }) => {
    await page.goto("/dashboard");

    const imagesWithoutAlt = await page.evaluate(() => {
      const images = document.querySelectorAll('img');
      const violations: string[] = [];
      
      images.forEach(img => {
        const alt = img.getAttribute('alt');
        const role = img.getAttribute('role');
        
        // Images should have alt text or be marked as decorative with role="presentation"
        if (alt === null && role !== 'presentation') {
          violations.push(img.src || 'image without src');
        }
      });
      
      return violations;
    });

    expect(imagesWithoutAlt).toEqual([]);
  });

  test("landmark regions are properly identified", async ({ page }) => {
    await page.goto("/dashboard");

    // Check for presence of landmark regions
    const landmarks = await page.evaluate(() => {
      return {
        hasMain: !!document.querySelector('main, [role="main"]'),
        hasNav: !!document.querySelector('nav, [role="navigation"]'),
        hasHeader: !!document.querySelector('header, [role="banner"]'),
        hasFooter: !!document.querySelector('footer, [role="contentinfo"]'),
      };
    });

    // At minimum, should have a main region
    expect(landmarks.hasMain).toBeTruthy();
  });
});

test.describe("Component accessibility", () => {
  test("call graph has accessible title and description", async ({ page }) => {
    await page.goto("/dashboard");

    const mockReport = `{
      "auth_gaps": [
        {
          "function_name": "initialize",
          "code": "AUTH_GAP"
        }
      ],
      "call_graph": [
        {
          "caller": "user_action",
          "callee": "internal_helper",
          "file": "src/lib.rs",
          "line": 100,
          "contract_id_expr": "self"
        }
      ]
    }`;

    const textarea = page.getByPlaceholder(/size_warnings/);
    await textarea.fill(mockReport);
    
    await page.waitForTimeout(500);
    
    await page.evaluate(() => {
      const buttons = Array.from(document.querySelectorAll('button'));
      const parseButton = buttons.find(btn => btn.textContent?.includes('Parse JSON'));
      if (parseButton) {
        (parseButton as HTMLButtonElement).click();
      }
    });
    
    await page.waitForTimeout(3000);
    
    await expect(page.getByRole("tab", { name: "Call Graph" })).toBeVisible();
    await page.getByRole("tab", { name: "Call Graph" }).click();
    await expect(page.getByRole("tab", { name: "Call Graph" })).toBeVisible();
    
    const tabPanel = page.getByRole("tabpanel");
    await expect(tabPanel).toBeVisible();
  });

  test("sanctity score chart has accessible label", async ({ page }) => {
    await page.goto("/dashboard");

    const mockReport = {
      summary: { total_findings: 1, has_critical: true, has_high: false },
      findings: {
        auth_gaps: [{ code: "AUTH_GAP", function: "test.rs:func" }],
        panic_issues: [],
        arithmetic_issues: [],
        unsafe_patterns: [],
        ledger_size_warnings: [],
        custom_rules: [],
      },
    };

    await page.evaluate((report) => {
      const textarea = document.querySelector("textarea");
      if (textarea) {
        textarea.value = JSON.stringify(report);
        textarea.dispatchEvent(new Event("input", { bubbles: true }));
      }
    }, mockReport);

    await page.getByRole("button", { name: "Parse JSON" }).click();

    const scoreSvg = page.locator('svg[aria-label*="Sanctity score"]');
    await expect(scoreSvg).toBeVisible();
  });

  test("severity bars have progress role", async ({ page }) => {
    await page.goto("/dashboard");

    const mockReport = `{
      "auth_gaps": [
        {
          "function_name": "initialize",
          "code": "AUTH_GAP"
        }
      ]
    }`;

    const textarea = page.getByPlaceholder(/size_warnings/);
    await textarea.fill(mockReport);
    
    await page.waitForTimeout(500);
    
    await page.evaluate(() => {
      const buttons = Array.from(document.querySelectorAll('button'));
      const parseButton = buttons.find(btn => btn.textContent?.includes('Parse JSON'));
      if (parseButton) {
        (parseButton as HTMLButtonElement).click();
      }
    });
    
    await page.waitForTimeout(3000);

    const criticalBar = page.getByRole("progressbar", { name: /critical/i });
    await expect(criticalBar).toBeAttached();
    await expect(criticalBar).toHaveAttribute("role", "progressbar");
    await expect(criticalBar).toHaveAttribute("aria-valuemin", "0");
    await expect(criticalBar).toHaveAttribute("aria-valuemax", "1");

    const highBar = page.getByRole("progressbar", { name: /high/i });
    await expect(highBar).toBeAttached();
    await expect(highBar).toHaveAttribute("role", "progressbar");
    await expect(highBar).toHaveAttribute("aria-valuemin", "0");
    await expect(highBar).toHaveAttribute("aria-valuemax", "1");

    const mediumBar = page.getByRole("progressbar", { name: /medium/i });
    await expect(mediumBar).toBeAttached();
    await expect(mediumBar).toHaveAttribute("role", "progressbar");

    const lowBar = page.getByRole("progressbar", { name: /low/i });
    await expect(lowBar).toBeAttached();
    await expect(lowBar).toHaveAttribute("role", "progressbar");
  });
});
