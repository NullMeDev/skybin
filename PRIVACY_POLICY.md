# Privacy Policy

**Effective Date**: 2025-01-09  
**Version**: 1.0

## Overview

SkyBin is committed to protecting your privacy and maintaining the anonymity of all data on our platform. This document outlines our data handling practices and privacy protections.

## Core Privacy Principles

### 1. Complete Anonymity

**All data stored on SkyBin is completely anonymous.** We do not collect, store, or retain:
- User identities or names
- Email addresses
- IP addresses or connection information
- User agent strings or browser information
- Location data
- Cookies or tracking data
- Account information
- Authentication credentials

### 2. Anonymization of Uploaded Pastes

When a user submits a paste to SkyBin:

1. **Author names are stripped** - Any author field is removed before storage
2. **Titles are sanitized** - Email addresses, URLs, and usernames are removed from titles
3. **URLs are removed** - Any URLs in metadata are stripped
4. **Content is stored as-is** - Paste content is stored unchanged for full transparency
5. **No metadata is retained** - Only content hash, source, and timestamp are kept

**Result**: A user uploads a paste, and we store only the content with no ability to identify the original uploader.

### 3. Anonymization of Scraped Data

When SkyBin scrapes data from public paste sites:

1. **Source is recorded** - We track which paste site the content came from (e.g., "Pastebin", "GitHub Gists")
2. **Author names are stripped** - Original author information is discarded
3. **URLs are anonymized** - Original source URLs are not retained
4. **Paste IDs are replaced** - We generate new anonymous internal IDs
5. **Content is preserved** - Full paste content is stored for transparency

**Result**: Scraped pastes are separated from their original context, preventing re-identification.

## Data Storage

### What We Store

- **Content**: Full paste content as submitted or scraped
- **Content Hash**: SHA256 hash for deduplication (non-reversible)
- **Source**: Which paste service it came from
- **Timestamp**: When the paste was created/discovered
- **Syntax**: Programming language identifier (optional)
- **Pattern Matches**: Detected security patterns (generic, not personally identifying)
- **Sensitivity Flag**: Whether pattern matched detected sensitive data

### What We Don't Store

- ❌ Author names
- ❌ User identities
- ❌ Email addresses
- ❌ IP addresses
- ❌ URLs or links (especially those containing usernames/IDs)
- ❌ Usernames or handles
- ❌ Personal information of any kind
- ❌ Account information
- ❌ Browse history or tracking data

## Data Retention

All pastes are automatically deleted after **7 days** regardless of content. We maintain:

- No archives
- No backups of deleted content
- No permanent storage
- Automatic purge on expiration

## Security & Detection

### Pattern Detection

SkyBin uses pattern detection to identify potentially sensitive information:
- API keys and credentials
- SSH and PGP private keys
- Credit card numbers
- Database connection strings
- Email/password combinations

**Important**: Pattern matching is used for classification only. Detected patterns are:
- Never used to identify users
- Never shared with third parties
- Used only for marking content sensitivity level
- Not extractable from the system

### Sensitive Content

Content identified as containing potential sensitive data:
- Is clearly marked with a "SENSITIVE" warning
- Is still stored and searchable (users have right to access their own data)
- Does not change our anonymization practices
- Does not result in any user identification

## Search and Discovery

### Full-Text Search

SkyBin provides full-text search across all paste content:
- No user tracking during searches
- No search logging or history
- No personalization
- Results are identical for all users searching the same query
- Search queries are not retained

### Filters

Available search filters include:
- Source (which paste site)
- Sensitivity level (safe/contains patterns)
- Results per page

These filters do not result in any tracking or identification.

## User Submissions

When you upload a paste via the web interface:

1. **No authentication required** - Upload anonymously without creating an account
2. **No personal information collected** - We don't ask for your name, email, or any identifier
3. **IP address not logged** - Your connection information is not retained
4. **Content is anonymized** - Title and content are processed through our anonymization layer
5. **Result is completely anonymous** - No way to trace the paste back to you

## Transparency

### Content Access

- All stored pastes are searchable by any user
- No content is hidden or private
- Paste content is displayed in full
- This is intentional - SkyBin is a public repository of anonymized pastes

### Metadata Access

Public metadata about pastes:
- Creation date
- Source (which service it came from)
- Syntax language
- Sensitivity classification
- View count

### No User Profiles

There are no user profiles on SkyBin because:
- Users are completely anonymous
- No accounts are created
- No personal pages exist
- No user-specific data is stored

## External Services

### What We Do NOT Do

- **No third-party tracking** - We don't use Google Analytics, Mixpanel, or similar
- **No ads** - No advertising networks or ad tracking
- **No external data brokers** - We don't sell or share data
- **No analytics** - We don't track individual behavior
- **No cookies** - No tracking cookies are set
- **No social media integration** - No Facebook Pixel, Twitter Tags, etc.

## Technical Privacy Measures

### Hashing

Content hashes (SHA256) are used for:
- Deduplication (preventing exact duplicates)
- Integrity verification
- NOT for identifying users (hashes are non-reversible)

### Database Security

- Passwords: No user passwords (no accounts)
- Encryption: Content not encrypted (not necessary since it's public)
- Access: Limited to authorized code paths only
- Backups: Follow same retention policies (7-day deletion)

## Legal Compliance

### GDPR

SkyBin complies with GDPR because:
- No personal data is collected
- No user tracking occurs
- No third-party data sharing
- Data is deleted automatically (7-day retention)
- Users can request deletion (though already anonymous)

### CCPA

SkyBin complies with CCPA because:
- No personal information is collected
- No sale of user data (no data exists)
- No tracking or profiling
- No opt-out required (nothing to opt out of)

## Important Notes

### Public Data

This platform stores publicly available paste content. By the nature of the sites we scrape (Pastebin, GitHub Gists, etc.), content is already public. We:
- Anonymize the context
- Remove identifying metadata
- Preserve the content transparency
- Enable security research and pattern detection

### Not a Data Recovery Service

If you posted sensitive data to Pastebin and it was scraped by SkyBin:
- We can't identify that it was you
- We can't remove specific pastes (we remove all pastes after 7 days)
- You should delete it from the original source immediately
- Consider this platform as evidence that your data may have been seen

### Responsible Disclosure

If you discover a privacy issue:
1. Do not publicly disclose it
2. Contact us via [add contact method]
3. Allow 30 days for response and fix
4. We'll acknowledge the issue and provide details of any fix

## Changes to This Policy

We may update this privacy policy. Changes will be:
- Communicated via this document
- Posted to GitHub
- Effective immediately upon posting
- Never retroactively weaken privacy protections

Last updated: **2025-01-09**

## Questions?

For questions about this privacy policy or our data practices:
- Check our source code: [GitHub link]
- Review our anonymization module: `src/anonymization.rs`
- Open an issue on GitHub with your questions

---

**Remember**: SkyBin is built with privacy as a core principle. All data is anonymous, all users are protected, and no personal information is retained.
