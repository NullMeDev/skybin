# PasteVault API Usage Guide

## URL Submission Feature

PasteVault now includes a powerful **External URL Submission** feature that allows you to monitor paste URLs from ANY paste site, not just those with public APIs.

### How It Works

The External URL Scraper maintains a queue of submitted URLs. When URLs are submitted via the API, they are:
1. Added to a processing queue
2. Fetched by the scraper on its next cycle (every 5 minutes by default)
3. Content is extracted and stored in the database
4. Pattern detection runs automatically
5. Pastes appear in the web interface

### API Endpoint

**POST** `/api/submit-url`

Submit one or more paste URLs for monitoring.

### Request Format

#### Single URL
```json
{
  "url": "https://pastebin.com/AbCd1234",
  "urls": []
}
```

#### Multiple URLs
```json
{
  "url": "",
  "urls": [
    "https://pastebin.com/AbCd1234",
    "https://gist.github.com/user/xyz789",
    "https://rentry.co/example",
    "https://dpaste.org/sample"
  ]
}
```

### Response Format

```json
{
  "success": true,
  "data": {
    "queued": 4,
    "message": "Queued 4 URL(s) for scraping"
  },
  "error": null
}
```

### Supported Paste Sites

The scraper automatically detects the source from the URL and supports:
- Pastebin (`pastebin.com`)
- GitHub Gists (`gist.github.com`)
- Paste.ee (`paste.ee`)
- DPaste (`dpaste.com`, `dpaste.org`)
- Rentry (`rentry.co`, `rentry.org`)
- Hastebin (`hastebin.com`, `toptal.com/developers/hastebin`)
- Any other paste site (tagged as "external")

### Usage Examples

#### Using curl
```bash
# Submit a single URL
curl -X POST http://localhost:8081/api/submit-url \
  -H "Content-Type: application/json" \
  -d '{"url": "https://pastebin.com/raw/AbCd1234", "urls": []}'

# Submit multiple URLs
curl -X POST http://localhost:8081/api/submit-url \
  -H "Content-Type: application/json" \
  -d '{
    "url": "",
    "urls": [
      "https://pastebin.com/AbCd1234",
      "https://gist.github.com/username/abc123"
    ]
  }'
```

#### Using Python
```python
import requests

# Submit URLs
response = requests.post(
    "http://localhost:8081/api/submit-url",
    json={
        "url": "",
        "urls": [
            "https://pastebin.com/AbCd1234",
            "https://gist.github.com/user/xyz789"
        ]
    }
)

print(response.json())
# Output: {'success': True, 'data': {'queued': 2, 'message': 'Queued 2 URL(s) for scraping'}, 'error': None}
```

#### Using JavaScript/Node.js
```javascript
fetch('http://localhost:8081/api/submit-url', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    url: 'https://pastebin.com/AbCd1234',
    urls: []
  })
})
  .then(response => response.json())
  .then(data => console.log(data));
```

### Integration Ideas

#### 1. Social Media Monitor
Monitor Twitter/Reddit/Discord for paste links and automatically submit them:
```bash
#!/bin/bash
# Example: Extract paste URLs from a log file and submit them
grep -Eo 'https?://pastebin\.com/[A-Za-z0-9]+' social_media.log | \
  jq -Rs 'split("\n") | map(select(length > 0)) | {url: "", urls: .}' | \
  curl -X POST http://localhost:8081/api/submit-url \
    -H "Content-Type: application/json" \
    -d @-
```

#### 2. Browser Extension
Create a browser extension that:
- Detects when you visit a paste site
- Adds a "Submit to PasteVault" button
- Sends the current URL to your PasteVault instance

#### 3. Slack/Discord Bot
```python
# Example Discord bot command
@bot.command()
async def monitor(ctx, url: str):
    """Submit a paste URL to PasteVault"""
    response = requests.post(
        "http://pastevault.local:8081/api/submit-url",
        json={"url": url, "urls": []}
    )
    if response.json()["success"]:
        await ctx.send(f"✅ Queued URL for monitoring")
    else:
        await ctx.send(f"❌ Failed to queue URL")
```

#### 4. CI/CD Integration
Monitor your own organization's paste submissions:
```yaml
# GitHub Actions example
- name: Submit internal paste to monitoring
  run: |
    curl -X POST https://pastevault.company.com/api/submit-url \
      -H "Content-Type: application/json" \
      -d '{"url": "${{ env.PASTE_URL }}", "urls": []}'
```

### Error Handling

#### No valid URLs provided
```json
{
  "success": false,
  "data": null,
  "error": "No valid URLs provided"
}
```

URLs must start with `http://` or `https://` to be considered valid.

#### Empty request
```json
{
  "success": false,
  "data": null,
  "error": "No valid URLs provided"
}
```

### Queue Management

- The queue processes up to 10 URLs per scrape cycle
- Duplicate URLs are automatically filtered
- URLs are processed in FIFO order
- Failed URLs are not re-queued (submit again if needed)

### Monitoring Queue Status

Currently, there's no API endpoint to check queue status. This is planned for a future release.

To monitor if your submissions are being processed:
1. Check the logs: `tail -f logs/pastevault.log`
2. Watch the API: `GET /api/pastes` to see if new pastes appear
3. Check the web interface at `http://localhost:8081`

### Rate Limiting

Currently, there are no rate limits on URL submission. However, best practices:
- Don't submit more than 100 URLs per minute
- Allow time for the queue to process (5-minute default cycle)
- Consider the impact on your network and the target paste sites

### Privacy & Legal Notes

**Important Considerations:**
- Only submit URLs to publicly accessible pastes
- Respect paste site Terms of Service
- Don't submit URLs containing sensitive personal data
- This tool is for security research and legitimate monitoring
- Be aware of data retention laws in your jurisdiction

### Other API Endpoints

#### GET /api/pastes
Get recent pastes (includes submitted URLs after processing)

#### GET /api/paste/:id
Get specific paste details

#### GET /api/search?query=keyword
Search pastes (full-text search)

#### GET /api/stats
Get statistics about monitored pastes

#### POST /api/upload
Submit a new paste directly (not a URL)

---

## Example Workflow

### Scenario: Monitoring a Security Research Project

1. **Discover a paste URL** on Twitter mentioning your company
   ```
   https://pastebin.com/SuspiciousData
   ```

2. **Submit it to PasteVault**
   ```bash
   curl -X POST http://localhost:8081/api/submit-url \
     -H "Content-Type: application/json" \
     -d '{"url": "https://pastebin.com/SuspiciousData", "urls": []}'
   ```

3. **Wait for processing** (up to 5 minutes)

4. **Check results** via web interface or API
   ```bash
   curl http://localhost:8081/api/pastes | jq '.data[] | select(.source=="pastebin")'
   ```

5. **Review detected patterns**
   - Pattern detection runs automatically
   - Check `is_sensitive` flag
   - View matched patterns in paste details

---

## Future Enhancements

Planned features for the URL submission system:
- [ ] GET /api/queue-status - Check current queue size and status
- [ ] POST /api/bulk-submit - Submit large batches (1000+ URLs) efficiently
- [ ] Webhook notifications when submitted URLs are processed
- [ ] Priority queue for urgent submissions
- [ ] API authentication and rate limiting per user
- [ ] Historical tracking of submitted URLs

---

For more information, see the main README.md or visit the project repository.
