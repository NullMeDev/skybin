# Quick Start: Enable GitHub Pages (5 Minutes)

## The Problem

The CI/CD workflow is ready, but GitHub Pages needs to be **manually enabled once** to authorize the automated deployment.

## The Solution (3 Steps)

### Step 1️⃣: Go to Settings → Pages
```
https://github.com/NullMeDev/skybin/settings/pages
```

### Step 2️⃣: Select "GitHub Actions" as Source
Under "Build and deployment":
- Find the dropdown that says "Deploy from a branch"
- Change it to: **"GitHub Actions"**
- Click "Save"

### Step 3️⃣: Wait 1-2 Minutes
The workflow will automatically trigger and deploy:
- ✅ Beautiful landing page
- ✅ API documentation
- ✅ Navigation to GitHub repo

## Done! 🎉

Your site will be live at:
```
https://nullmedev.github.io/skybin/
```

## Troubleshooting

**If you still see an error:**

1. ✅ Verify you have **Admin access** to the repo
2. ✅ Confirm source is set to **"GitHub Actions"** (not a branch)
3. ✅ Check Actions tab: https://github.com/NullMeDev/skybin/actions
4. ✅ Look for "Deploy to GitHub Pages" workflow
5. ✅ If it failed, click "Run workflow" to retry

## After Setup

- **Every push to main** → Auto-deploys updated site
- **Manual re-deploy**: Go to Actions tab, click "Run workflow"
- **Custom domain**: Can be added in Settings → Pages

## Still Having Issues?

See the full troubleshooting guide:
```
📄 GITHUB_PAGES_SETUP.md
```

---

**That's it!** GitHub Pages is now automatically configured for all future deployments.
