# Accessibility Audit Report

## Overview
This document outlines the structured accessibility audit implemented for the Sanctifier frontend application, focusing on:
- Tab order and keyboard navigation
- Contrast ratios
- ARIA labels and roles

## Audit Implementation

### 1. Automated Testing Tools
- **@axe-core/playwright**: WCAG 2.1/2.2 compliance testing
- **@storybook/addon-a11y**: Component-level accessibility checks
- **Custom Playwright tests**: Tab order, focus indicators, ARIA validation

### 2. Test Coverage

#### Comprehensive Accessibility Audit (Playwright)
- Homepage accessibility violations scan
- Dashboard page accessibility violations scan
- Playground page accessibility violations scan
- WCAG 2.0 A/AA, 2.1 AA, 2.2 AA standards

#### Tab Order and Keyboard Navigation
- Logical and sequential tab order verification
- Severity filter buttons keyboard navigation
- Tab navigation follows ARIA patterns
- Focus indicators visibility

#### ARIA Labels and Roles
- Interactive elements have accessible names
- Form inputs have associated labels
- Images have alt text or are decorative
- Landmark regions properly identified

#### Component Accessibility
- Call graph accessible title and description
- Sanctity score chart accessible label
- Severity bars have progress role

## Running the Audit

### Playwright E2E Tests
```bash
npm run test:e2e tests/e2e/accessibility.spec.ts
```

### Storybook Accessibility Checks
```bash
npm run storybook
```
Then navigate to the A11y addon panel in Storybook to view violations.

## Key Accessibility Standards

### WCAG 2.1 Level AA Compliance
- **1.4.3 Contrast (Minimum)**: Text contrast ratio of at least 4.5:1
- **1.4.6 Contrast (Enhanced)**: Large text contrast ratio of at least 3:1
- **2.4.3 Focus Order**: Focusable components receive focus in an order that preserves meaning and operability
- **2.4.7 Focus Visible**: Keyboard focus indicator is visible
- **4.1.2 Name, Role, Value**: All UI components have names, roles, and values

### ARIA Best Practices
- Use semantic HTML elements where possible
- Provide aria-label for icon-only buttons
- Use aria-labelledby for complex labels
- Implement proper landmark regions (main, nav, header, footer)
- Ensure dynamic content updates are announced (aria-live)

## Next Steps

1. Run the automated test suite
2. Review Storybook a11y addon results
3. Address any violations found
4. Implement manual keyboard navigation testing
5. Conduct screen reader testing
6. Regular regression testing

## Common Issues to Watch For

### Tab Order
- Custom tabindex values that disrupt natural flow
- Hidden elements that remain focusable
- Modal dialogs that don't trap focus
- Skip navigation links missing

### Contrast
- Gray text on dark backgrounds
- Placeholder text with insufficient contrast
- Disabled state indicators with poor contrast
- Focus indicators with low contrast

### ARIA Labels
- Icon buttons without aria-label
- Form fields without associated labels
- Images without alt text
- Dynamic content without aria-live regions
- Custom widgets without proper ARIA roles

## Resources
- [WCAG 2.1 Guidelines](https://www.w3.org/WAI/WCAG21/quickref/)
- [ARIA Authoring Practices](https://www.w3.org/WAI/ARIA/apg/)
- [WebAIM Contrast Checker](https://webaim.org/resources/contrastchecker/)
