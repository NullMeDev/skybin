# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in SkyBin, please **do not** create a public GitHub issue. Instead, please report it responsibly to the maintainers.

### How to Report

1. **Email**: Contact the project maintainers at `dev@nullme.dev` with:
   - Description of the vulnerability
   - Steps to reproduce (if applicable)
   - Potential impact
   - Suggested fix (if you have one)

2. **Subject Line**: Start with `[SECURITY]` to indicate priority

3. **Response Time**: We aim to acknowledge security reports within 48 hours

### What to Expect

- We will investigate the vulnerability
- We will work on a fix in a private branch
- We will credit you for the discovery (unless you prefer anonymity)
- We will release a patched version with a security advisory

## Security Considerations

### Design

- **No Authentication by Design**: SkyBin is intentionally designed without authentication to preserve user anonymity
- **Data Retention**: Configure `retention_days` in `config.toml` to limit how long sensitive data is stored
- **Rate Limiting**: Built-in rate limiting prevents abuse of scraping endpoints

### Best Practices When Deploying

1. **Environment Isolation**: Run on isolated infrastructure
2. **Network Security**: Use firewalls to restrict access
3. **Database Protection**: Secure the SQLite database file with appropriate permissions
4. **Secrets Management**: Use environment variables (not config files) for API keys:
   ```bash
   export PASTEBIN_API_KEY="your_key_here"
   export GITHUB_TOKEN="your_token_here"
   ```

5. **Regular Updates**: Keep Rust and dependencies updated:
   ```bash
   rustup update
   cargo update
   ```

6. **Monitoring**: Enable debug logging to monitor suspicious activity:
   ```bash
   RUST_LOG=debug ./paste-vault
   ```

### Known Security Limitations

- **Single-Writer Database**: SQLite limits write concurrency; under extreme load, consider database migration
- **No Encryption at Rest**: Database files are not encrypted; use filesystem-level encryption if needed
- **In-Memory Buffers**: Large pastes are held in memory during processing; consider memory constraints
- **Pattern Detection**: Complex regex patterns could be susceptible to ReDoS attacks; patterns are validated before use

## Security Features

### Built-In

1. **Input Validation**: All web inputs are validated before processing
2. **HTML Escaping**: Template engine (Askama) auto-escapes output
3. **Size Limits**: Configurable paste size limits prevent DoS
4. **Rate Limiting**: Per-source rate limiting with jitter prevents hammering
5. **Auto-Purge**: Automatic data retention and cleanup
6. **Hash Deduplication**: SHA256 hashing prevents duplicate storage

### Recommended Additions

- WAF (Web Application Firewall) in front of deployment
- Regular security audits of new scrapers
- Dependency vulnerability scanning (via `cargo audit`)
- Intrusion detection monitoring

## Dependencies

SkyBin's security posture depends on its dependencies. Key dependencies include:

- **tokio**: Async runtime (widely audited)
- **axum**: Web framework (Tokio team maintained)
- **rusqlite**: SQLite bindings (simple, well-tested)
- **regex**: Pattern matching (maintained by Rust team)
- **reqwest**: HTTP client (widely used)

We recommend regularly running:

```bash
cargo audit
```

To check for known vulnerabilities in dependencies.

## Compliance

SkyBin is designed to help detect leaked credentials and sensitive data. However, operators must ensure compliance with:

- **GDPR**: For EU users' data
- **CCPA**: For California residents' data
- **Local Privacy Laws**: Based on deployment location
- **Terms of Service**: Of scraped paste services
- **CFAA (US)**: Computer Fraud and Abuse Act implications

## Future Security Enhancements

- [ ] TLS/SSL support for web server
- [ ] Database encryption at rest
- [ ] Audit logging for all operations
- [ ] API key-based authentication
- [ ] Rate limiting per IP address
- [ ] OWASP Top 10 security assessment

## Security Disclosure Timeline

When we receive a security report:

1. **Day 0**: Acknowledge receipt and begin investigation
2. **Days 1-3**: Develop and test fix
3. **Day 4**: Release patched version and advisory
4. **Day 5+**: Monitor for exploitation in the wild

---

**Thank you for helping keep SkyBin secure!**
