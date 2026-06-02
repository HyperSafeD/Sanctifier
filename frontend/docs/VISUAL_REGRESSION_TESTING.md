# Visual Regression Testing with Chromatic

This document explains the visual regression testing setup for the Sanctifier dashboard using Chromatic and Storybook.

## Overview

Visual regression testing helps catch unintended UI changes by comparing screenshots of components before and after code changes. This ensures that:

- UI changes are intentional and reviewed
- Styling bugs are caught early
- Component appearance remains consistent
- Cross-browser rendering issues are detected

## Technology Stack

### Chromatic
- **Purpose**: Visual regression testing platform
- **Features**: 
  - Automated screenshot comparison
  - Cross-browser testing
  - UI review workflow
  - Change detection and approval
  - Integration with Storybook

### Storybook
- **Purpose**: Component development and documentation
- **Features**:
  - Isolated component development
  - Interactive component playground
  - Accessibility testing integration
  - Visual testing foundation

## Setup and Configuration

### Prerequisites

1. **Chromatic Account**: Sign up at [chromatic.com](https://www.chromatic.com/)
2. **Project Token**: Obtain from Chromatic project settings
3. **GitHub Repository**: Connected to Chromatic

### Environment Configuration

#### GitHub Secrets

Add the following secret to your GitHub repository:

```
CHROMATIC_PROJECT_TOKEN=<your-project-token>
```

**How to add:**
1. Go to repository Settings → Secrets and variables → Actions
2. Click "New repository secret"
3. Name: `CHROMATIC_PROJECT_TOKEN`
4. Value: Your Chromatic project token
5. Click "Add secret"

#### Chromatic Configuration

The project uses `chromatic.config.json` for configuration:

```json
{
  "projectId": "PROJECT_ID_PLACEHOLDER",
  "buildScriptName": "build-storybook",
  "storybookBuildDir": "storybook-static",
  "exitZeroOnChanges": true,
  "exitOnceUploaded": true,
  "autoAcceptChanges": "main",
  "skip": "dependabot/**",
  "onlyChanged": true,
  "traceChanged": "expanded"
}
```

**Configuration Options:**
- `projectId`: Chromatic project identifier
- `autoAcceptChanges`: Auto-approve changes on main branch
- `onlyChanged`: Only test changed stories (faster builds)
- `skip`: Skip Chromatic for Dependabot PRs
- `traceChanged`: Track which files caused changes

## Usage

### Local Development

#### Run Storybook Locally
```bash
cd frontend
npm run storybook
```

Access at: http://localhost:6006

#### Build Storybook
```bash
npm run build-storybook
```

#### Run Chromatic Locally
```bash
# Requires CHROMATIC_PROJECT_TOKEN environment variable
export CHROMATIC_PROJECT_TOKEN=your-token
npm run chromatic
```

### CI/CD Integration

#### Automatic Triggers

Visual regression tests run automatically on:

1. **Pull Requests**: 
   - Targeting `main` or `develop` branches
   - With changes to `frontend/**` files

2. **Push to Main/Develop**:
   - Establishes baseline for future comparisons
   - Auto-accepts changes as new baseline

#### Workflow Steps

1. **Build Storybook**: Compile all component stories
2. **Upload to Chromatic**: Send build for visual testing
3. **Compare Screenshots**: Chromatic compares with baseline
4. **Report Results**: Comment on PR with findings
5. **Review Changes**: Manual review if changes detected

### Review Workflow

#### When Changes Are Detected

1. **PR Comment**: Automatic comment with Chromatic results
2. **Review Link**: Click "View Build Results" in comment
3. **Inspect Changes**: Review each visual change in Chromatic UI
4. **Accept or Reject**:
   - ✅ **Accept**: Approve intentional changes
   - ❌ **Deny**: Reject unintended changes, fix code

#### Chromatic UI Review

**Change Types:**
- 🟢 **New**: New component or story added
- 🟡 **Changed**: Visual differences detected
- 🔴 **Removed**: Component or story removed

**Review Actions:**
- **Accept**: Approve change as new baseline
- **Deny**: Reject change, requires code fix
- **Batch Accept**: Approve multiple changes at once

## Best Practices

### Writing Stories for Visual Testing

#### Component Coverage
```typescript
// Good: Multiple states covered
export const Default: Story = {};
export const Loading: Story = { args: { isLoading: true } };
export const Error: Story = { args: { error: 'Failed to load' } };
export const Empty: Story = { args: { data: [] } };
```

#### Responsive Testing
```typescript
export const Mobile: Story = {
  parameters: {
    viewport: { defaultViewport: 'mobile1' }
  }
};

export const Tablet: Story = {
  parameters: {
    viewport: { defaultViewport: 'tablet' }
  }
};
```

#### Dark Mode Testing
```typescript
export const DarkMode: Story = {
  parameters: {
    backgrounds: { default: 'dark' }
  }
};
```

### Reducing False Positives

#### Exclude Dynamic Content
```typescript
export const WithTimestamp: Story = {
  parameters: {
    chromatic: {
      // Ignore specific elements
      ignore: ['.timestamp', '.random-id']
    }
  }
};
```

#### Disable Animations
```typescript
export const Animated: Story = {
  parameters: {
    chromatic: {
      // Pause animations for consistent screenshots
      pauseAnimationAtEnd: true
    }
  }
};
```

#### Delay Capture
```typescript
export const AsyncContent: Story = {
  parameters: {
    chromatic: {
      // Wait for async content to load
      delay: 1000
    }
  }
};
```

### Performance Optimization

#### Skip Unchanged Stories
```typescript
// Skip stories that rarely change
export const Static: Story = {
  parameters: {
    chromatic: { disableSnapshot: true }
  }
};
```

#### Use TurboSnap
- Enabled by default with `onlyChanged: true`
- Only tests stories affected by code changes
- Significantly faster builds

## Troubleshooting

### Common Issues

#### 1. Chromatic Token Not Found

**Error**: `Error: CHROMATIC_PROJECT_TOKEN not found`

**Solution**:
- Verify secret is added to GitHub repository
- Check secret name matches exactly
- Ensure workflow has access to secrets

#### 2. Build Failures

**Error**: `Storybook build failed`

**Solution**:
```bash
# Test build locally
npm run build-storybook

# Check for errors
npm run storybook
```

#### 3. Too Many Changes Detected

**Causes**:
- Font rendering differences
- Dynamic content (timestamps, IDs)
- Animation timing
- Browser differences

**Solutions**:
- Use `chromatic.ignore` for dynamic elements
- Disable animations with `pauseAnimationAtEnd`
- Use consistent test data
- Add delays for async content

#### 4. Slow Builds

**Optimization**:
- Enable `onlyChanged` (already enabled)
- Use `skip` for non-UI changes
- Disable snapshots for static stories
- Reduce story count if excessive

### Debug Mode

Enable diagnostics for troubleshooting:

```json
{
  "diagnostics": true
}
```

Or via CLI:
```bash
npm run chromatic -- --diagnostics
```

## Workflow Integration

### Pull Request Process

1. **Create PR**: Open pull request with UI changes
2. **Automatic Test**: Chromatic runs automatically
3. **Review Comment**: Bot comments with results
4. **Visual Review**: Review changes in Chromatic
5. **Approve/Fix**: Accept changes or fix issues
6. **Merge**: Merge after approval

### Baseline Management

#### Main Branch Baseline
- Changes to `main` auto-accepted as baseline
- Establishes truth for future comparisons
- Updated with each merge

#### Branch Baselines
- Each branch compared against main baseline
- Changes must be approved before merge
- Prevents unintended visual regressions

## Monitoring and Metrics

### Chromatic Dashboard

Access at: https://www.chromatic.com/builds

**Metrics Available:**
- Build history and trends
- Change frequency
- Review time
- Component coverage
- Browser compatibility

### GitHub Integration

**PR Checks:**
- ✅ **Passed**: No visual changes or all approved
- ⚠️ **Changes**: Visual changes need review
- ❌ **Failed**: Build or upload failed

## Advanced Configuration

### Custom Viewports

```typescript
// .storybook/preview.ts
export const parameters = {
  viewport: {
    viewports: {
      mobile: { name: 'Mobile', styles: { width: '375px', height: '667px' } },
      tablet: { name: 'Tablet', styles: { width: '768px', height: '1024px' } },
      desktop: { name: 'Desktop', styles: { width: '1920px', height: '1080px' } }
    }
  }
};
```

### Threshold Configuration

```json
{
  "threshold": 0.2,
  "diffThreshold": 0.063
}
```

### Parallel Testing

```json
{
  "parallel": true,
  "parallelism": 4
}
```

## Cost Optimization

### Snapshot Limits

Chromatic has snapshot limits based on plan:
- **Free**: 5,000 snapshots/month
- **Team**: 35,000 snapshots/month
- **Enterprise**: Custom limits

### Optimization Strategies

1. **TurboSnap**: Only test changed stories
2. **Skip Patterns**: Skip non-UI PRs
3. **Batch Changes**: Group related changes
4. **Story Pruning**: Remove redundant stories
5. **Viewport Limits**: Test essential viewports only

## Resources

### Documentation
- [Chromatic Documentation](https://www.chromatic.com/docs/)
- [Storybook Documentation](https://storybook.js.org/docs/)
- [Visual Testing Guide](https://storybook.js.org/docs/react/writing-tests/visual-testing)

### Support
- [Chromatic Support](https://www.chromatic.com/support)
- [Storybook Discord](https://discord.gg/storybook)
- [GitHub Discussions](https://github.com/storybookjs/storybook/discussions)

### Best Practices
- [Visual Testing Handbook](https://storybook.js.org/tutorials/visual-testing-handbook/)
- [Component Driven Development](https://www.componentdriven.org/)
- [Chromatic Best Practices](https://www.chromatic.com/docs/best-practices)

## Maintenance

### Regular Tasks

**Weekly:**
- Review pending changes in Chromatic
- Check build success rates
- Monitor snapshot usage

**Monthly:**
- Review and prune unused stories
- Update baseline if needed
- Optimize slow builds

**Quarterly:**
- Review viewport coverage
- Update testing strategy
- Evaluate cost vs. value

### Updating Dependencies

```bash
# Update Chromatic
npm update chromatic

# Update Storybook
npx storybook@latest upgrade

# Test after updates
npm run build-storybook
npm run chromatic
```

## FAQ

**Q: How long do builds take?**
A: Typically 2-5 minutes with TurboSnap enabled.

**Q: Can I test locally before pushing?**
A: Yes, run `npm run chromatic` with your project token.

**Q: What browsers are tested?**
A: Chrome by default. Additional browsers available on paid plans.

**Q: How do I handle flaky tests?**
A: Use `chromatic.ignore`, disable animations, or add delays.

**Q: Can I skip Chromatic for specific PRs?**
A: Yes, add `[skip chromatic]` to commit message or use skip patterns.

**Q: How do I review changes?**
A: Click the Chromatic link in the PR comment to review in the web UI.

---

**Need Help?** Contact the team or check the [Chromatic documentation](https://www.chromatic.com/docs/).