# Chromatic Setup Guide

Quick start guide for setting up Chromatic visual regression testing for the Sanctifier dashboard.

## Prerequisites

- GitHub repository access
- Admin access to repository settings
- Chromatic account (free tier available)

## Step-by-Step Setup

### 1. Create Chromatic Account

1. Visit [chromatic.com](https://www.chromatic.com/)
2. Click "Sign up with GitHub"
3. Authorize Chromatic to access your GitHub account
4. Select the Sanctifier repository

### 2. Get Project Token

1. In Chromatic dashboard, go to your project
2. Click "Manage" → "Configure"
3. Copy the "Project Token"
4. Keep this token secure (treat like a password)

### 3. Add GitHub Secret

1. Go to GitHub repository: `Settings` → `Secrets and variables` → `Actions`
2. Click "New repository secret"
3. Name: `CHROMATIC_PROJECT_TOKEN`
4. Value: Paste your Chromatic project token
5. Click "Add secret"

### 4. Update Configuration

Edit `frontend/chromatic.config.json`:

```json
{
  "projectId": "YOUR_PROJECT_ID_HERE",
  ...
}
```

Replace `PROJECT_ID_PLACEHOLDER` with your actual Chromatic project ID (found in Chromatic dashboard).

### 5. Test the Setup

#### Local Test (Optional)

```bash
cd frontend

# Set token temporarily
export CHROMATIC_PROJECT_TOKEN=your-token-here

# Run Chromatic
npm run chromatic
```

#### CI Test

1. Create a test branch:
```bash
git checkout -b test/chromatic-setup
```

2. Make a small UI change (e.g., update a component)

3. Commit and push:
```bash
git add .
git commit -m "test: Verify Chromatic setup"
git push origin test/chromatic-setup
```

4. Create a pull request

5. Check that:
   - ✅ Chromatic workflow runs
   - ✅ Build completes successfully
   - ✅ PR comment appears with results
   - ✅ Chromatic link works

### 6. Review First Build

1. Click the Chromatic link in the PR comment
2. Review the baseline snapshots
3. Accept all changes (this sets the baseline)
4. Future PRs will compare against this baseline

## Verification Checklist

- [ ] Chromatic account created
- [ ] Repository connected to Chromatic
- [ ] Project token obtained
- [ ] GitHub secret added
- [ ] Configuration file updated
- [ ] Test PR created
- [ ] Workflow runs successfully
- [ ] PR comment appears
- [ ] Chromatic dashboard accessible
- [ ] Baseline snapshots reviewed

## Troubleshooting

### Workflow Doesn't Run

**Check:**
- Workflow file exists: `.github/workflows/chromatic.yml`
- Changes are in `frontend/**` directory
- Branch is targeting `main` or `develop`

### Token Error

**Error**: `CHROMATIC_PROJECT_TOKEN not found`

**Fix:**
- Verify secret name is exactly `CHROMATIC_PROJECT_TOKEN`
- Check secret is added to repository (not organization)
- Ensure workflow has access to secrets

### Build Fails

**Check:**
- Storybook builds locally: `npm run build-storybook`
- Dependencies are installed: `npm ci`
- No TypeScript errors: `npm run lint`

### No PR Comment

**Possible Causes:**
- Workflow still running (wait for completion)
- No visual changes detected
- GitHub Actions permissions issue

**Fix:**
- Check workflow logs in Actions tab
- Verify `GITHUB_TOKEN` has write permissions
- Check if comment was collapsed

## Next Steps

1. **Add More Stories**: Create stories for all dashboard components
2. **Configure Viewports**: Test responsive designs
3. **Set Up Baselines**: Establish baselines for all components
4. **Train Team**: Share documentation with team members
5. **Monitor Usage**: Track snapshot usage and costs

## Quick Reference

### Commands

```bash
# Run Storybook locally
npm run storybook

# Build Storybook
npm run build-storybook

# Run Chromatic (requires token)
npm run chromatic

# Run Chromatic in CI mode
npm run chromatic:ci
```

### Links

- **Chromatic Dashboard**: https://www.chromatic.com/builds
- **Storybook Local**: http://localhost:6006
- **Documentation**: [VISUAL_REGRESSION_TESTING.md](./VISUAL_REGRESSION_TESTING.md)

### Support

- **Chromatic Docs**: https://www.chromatic.com/docs/
- **Storybook Docs**: https://storybook.js.org/docs/
- **GitHub Issues**: Report issues in repository

## Team Onboarding

Share this checklist with new team members:

1. Read [VISUAL_REGRESSION_TESTING.md](./VISUAL_REGRESSION_TESTING.md)
2. Access Chromatic dashboard (request access if needed)
3. Run Storybook locally: `npm run storybook`
4. Review existing stories and baselines
5. Practice creating a test PR with UI changes
6. Learn the review workflow in Chromatic

---

**Setup Complete!** 🎉 Visual regression testing is now active for the Sanctifier dashboard.