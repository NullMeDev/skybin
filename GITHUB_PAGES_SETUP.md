# GitHub Pages Setup Instructions

GitHub Pages deployment failed because it hasn't been enabled yet. Follow these steps to enable it:

## Manual Setup (Required First Time)

### Step 1: Enable GitHub Pages in Repository Settings

1. Go to: https://github.com/NullMeDev/skybin/settings/pages
2. Under "Build and deployment":
   - **Source**: Select "GitHub Actions"
   - This tells GitHub to use our automated workflow
3. Click "Save"

### Step 2: Wait for Workflow

After enabling:
1. Go to: https://github.com/NullMeDev/skybin/actions
2. The "Deploy to GitHub Pages" workflow should automatically trigger
3. Wait for it to complete (usually 1-2 minutes)

### Step 3: Verify Deployment

Once complete:
1. Your GitHub Pages site will be available at:
   - https://nullmedev.github.io/skybin/
2. Check the Actions tab to confirm the workflow passed

## What Will Be Deployed

The workflow deploys:
- **Landing Page**: Beautiful intro page with project features
- **API Documentation**: Auto-generated Rust documentation
- **Navigation**: Links to GitHub, docs, and API reference

## If Still Having Issues

### Issue: Still getting 404 error

**Solution:**
1. Check that you're logged in to GitHub
2. Verify you have Admin access to the repository
3. Go to Settings → Pages again and confirm:
   - Source is set to "GitHub Actions"
   - Not set to "Deploy from a branch"

### Issue: Workflow not running

**Solution:**
1. Navigate to: https://github.com/NullMeDev/skybin/settings/actions
2. Under "Actions permissions":
   - Ensure "Allow all actions and reusable workflows" is selected
3. Click "Save"

### Issue: Artifact upload failing

**Solution:**
If the artifact upload step fails:
1. Go to Settings → Actions → General
2. Under "Artifact and log retention":
   - Set to at least 5 days
3. Verify you have storage quota remaining

## Automatic Deployment on Future Pushes

Once enabled, GitHub Pages will automatically:
1. Trigger on every push to `main` branch
2. Generate updated documentation
3. Deploy new content
4. Update your site (usually within 1-2 minutes)

## Manual Trigger (Optional)

To manually re-deploy without pushing code:
1. Go to: https://github.com/NullMeDev/skybin/actions
2. Find "Deploy to GitHub Pages" workflow
3. Click "Run workflow"
4. Select branch: `main`
5. Click "Run workflow"

## Accessing Your Site

Once deployed, you can access:
- **Main Site**: https://nullmedev.github.io/skybin/
- **API Docs**: https://nullmedev.github.io/skybin/docs/paste_vault/
- **GitHub Repo**: https://github.com/NullMeDev/skybin

## Setting Custom Domain (Optional)

If you have a custom domain, you can configure it:
1. Go to Settings → Pages
2. Under "Custom domain":
   - Enter your domain (e.g., `skybin.example.com`)
   - Add DNS CNAME record pointing to `nullmedev.github.io`
3. Enable "Enforce HTTPS" (recommended)

## Troubleshooting Checklist

- [ ] GitHub Pages enabled (Settings → Pages)
- [ ] Source set to "GitHub Actions"
- [ ] Actions permissions allowed (Settings → Actions)
- [ ] Artifact retention set to 5+ days
- [ ] Workflow completed successfully (Actions tab)
- [ ] Site accessible at https://nullmedev.github.io/skybin/

---

For more info, see GitHub's official guide:
https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site
