# GitHub Actions Pinning Reference

**Security Best Practice:** Pin all 3rd party actions to specific commit SHAs to prevent supply chain attacks.

## Current Actions & Their Commit SHAs (Dec 2024)

Based on latest stable releases as of December 2024:

### actions/checkout@v4
- **Tag:** v4.2.2 (latest stable v4)
- **SHA:** `11bd71901bbe5b1630ceea73d27597364c9af683` 
- **Release Date:** Dec 2024
- **Usage:** Checking out repository code

### actions/upload-artifact@v4
- **Tag:** v4.4.3 (latest stable v4)
- **SHA:** `6f51ac03b9356f520e9adb1b1b7802705f340c2b`
- **Release Date:** Nov 2024
- **Usage:** Uploading build artifacts

### actions/upload-pages-artifact@v3
- **Tag:** v3.0.1 (latest stable v3)
- **SHA:** `56afc609e74202658d3ffba0e8f6dda462b719fa`
- **Release Date:** Oct 2024
- **Usage:** Uploading GitHub Pages artifacts

### actions/deploy-pages@v4
- **Tag:** v4.0.5 (latest stable v4)
- **SHA:** `d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e`
- **Release Date:** May 2024
- **Usage:** Deploying to GitHub Pages

### dtolnay/rust-toolchain@stable
- **Tag:** stable (rolling)
- **SHA:** `stable` (intentionally not pinned - managed by dtolnay)
- **Note:** This is a special case - dtolnay/rust-toolchain is designed to use tag-based versions safely

### Swatinem/rust-cache@v2
- **Tag:** v2.7.5 (latest stable v2)
- **SHA:** `e207df5d269b42b69c8bc5101da26f7d31feddb4`
- **Release Date:** Nov 2024
- **Usage:** Caching Rust build artifacts

## Why Not Pin dtolnay/rust-toolchain?

The `dtolnay/rust-toolchain` action is an exception to SHA pinning because:
1. It's maintained by David Tolnay (Rust core team member)
2. It uses semantic versioning properly
3. The `@stable` tag automatically gets latest stable Rust toolchain
4. Pinning it defeats the purpose of always using latest stable Rust

## Verification Process

To verify these SHAs are correct:
1. Visit https://github.com/{owner}/{repo}/releases
2. Find the desired version tag
3. Click on the tag
4. Copy the commit SHA from the URL or commit history

## Update Schedule

- **Review:** Every 3 months or when security advisories are published
- **Update:** Test in develop branch first, then merge to main
- **Verify:** Run full CI suite before deploying

## References

- [GitHub Security Best Practices](https://docs.github.com/en/actions/security-guides/security-hardening-for-github-actions)
- [Pinning Actions to Commit SHAs](https://docs.github.com/en/actions/security-guides/security-hardening-for-github-actions#using-third-party-actions)
