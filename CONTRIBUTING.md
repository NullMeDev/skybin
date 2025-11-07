# Contributing to SkyBin

Thank you for your interest in contributing to SkyBin! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

- Be respectful and constructive in all interactions
- No harassment, discrimination, or hostile behavior
- Focus on the code, not the person

## Getting Started

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Git with SSH configured for GitHub
- SQLite 3.x

### Setup

```bash
git clone git@github.com:NullMeDev/skybin.git
cd skybin
cargo build
cargo test --lib
```

### Running Locally

```bash
# Development mode with debug logging
RUST_LOG=debug cargo run

# Release mode (optimized)
cargo build --release
./target/release/paste-vault
```

## Development Workflow

1. Create a feature branch: `git checkout -b feature/description`
2. Make your changes
3. Run tests: `cargo test --lib`
4. Format code: `cargo fmt`
5. Check linting: `cargo clippy`
6. Commit with clear messages
7. Push and open a Pull Request

## Testing

### Run All Tests

```bash
cargo test --lib
```

### Run Specific Test

```bash
cargo test test_name -- --nocapture
```

### Test Coverage

Our current test coverage includes:
- Pattern detection (positive/negative cases)
- Database operations (insert, search, cleanup)
- Rate limiting behavior
- Hashing and deduplication
- Web server routes (basic responses)

### Adding Tests

Tests should be in the same file as the code they test, in a `#[cfg(test)] mod tests` section:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_feature() {
        let result = my_function();
        assert_eq!(result, expected_value);
    }
}
```

## Code Style

- Follow Rust naming conventions (snake_case for functions, CamelCase for types)
- Use meaningful variable names
- Write doc comments for public functions
- Maintain the existing code structure

### Format Code

```bash
cargo fmt
```

### Check Linting

```bash
cargo clippy
```

Fix clippy warnings before submitting PR (use `-D warnings` locally):

```bash
cargo clippy -- -D warnings
```

## Architecture Overview

### Key Modules

- **config**: TOML configuration parsing
- **db**: SQLite database layer with FTS5
- **hash**: SHA256 content deduplication
- **patterns**: Regex-based pattern detection
- **rate_limiter**: Per-source rate limiting
- **scrapers**: Individual paste source implementations
- **scheduler**: Scraping orchestration
- **web**: Axum web server and handlers
- **models**: Core data structures

### Adding a New Paste Source

1. Create `src/scrapers/{source}.rs`
2. Implement the `Scraper` trait with:
   - `name()` - Source identifier
   - `fetch_recent()` - Async function returning `Vec<DiscoveredPaste>`
3. Update `config.toml` to add source toggle in `[sources]`
4. Register in `src/scrapers/mod.rs`
5. Add tests for edge cases

Example:

```rust
pub struct MySource;

#[async_trait]
impl Scraper for MySource {
    fn name(&self) -> &str {
        "mysource"
    }

    async fn fetch_recent(&self) -> anyhow::Result<Vec<DiscoveredPaste>> {
        // Implementation
    }
}
```

### Adding a New Pattern

1. Add to `src/patterns/rules.rs` in the `BUILTIN_PATTERNS` constant
2. Define with: name, regex pattern, and severity level
3. Test with known data samples
4. Update config.toml if adding new category

## Pull Request Process

1. **Title**: Use clear, descriptive titles (e.g., "feat: add Gist scraper")
2. **Description**: Explain what and why, not just what
3. **Tests**: Include tests for new functionality
4. **Documentation**: Update README/WARP.md if needed
5. **CI/CD**: Ensure all GitHub Actions checks pass
6. **No merge conflicts**: Rebase if needed

### PR Labels

- `feature`: New functionality
- `bugfix`: Bug fixes
- `performance`: Performance improvements
- `docs`: Documentation changes
- `refactor`: Code refactoring

## Commit Messages

Follow conventional commits:

```
type(scope): subject

Body explaining the why and what (if needed).

Fixes #issue-number
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`

## Database Schema Changes

1. Create new migration in `migrations/` with incremented number
2. Update `db.rs` to run migration
3. Add tests for schema changes
4. Update WARP.md documentation

## Performance Considerations

- Concurrency is managed via `concurrent_scrapers` config
- Rate limiting respects source terms of service
- SQLite is single-writer; high write concurrency may cause locks
- FTS5 search is fast but adds overhead to inserts
- Memory usage grows with paste size and count

## Security Guidelines

- No hardcoded secrets or credentials
- Use environment variables for sensitive config
- Validate all user input
- HTML escape template output (Askama does this)
- Be cautious with regex (avoid ReDoS)

## Debugging

### Enable Tracing

```bash
RUST_LOG=paste_vault=trace cargo run
```

### Run with Backtrace

```bash
RUST_BACKTRACE=1 cargo run
```

### Database Inspection

```bash
sqlite3 pastevault.db
sqlite> SELECT * FROM pastes LIMIT 5;
sqlite> .schema pastes
```

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md` (if exists)
3. Commit: `chore: bump version to X.Y.Z`
4. Tag: `git tag vX.Y.Z`
5. Push: `git push origin main --tags`
6. GitHub Actions will build release binary

## Questions?

- Open an issue for bugs
- Start a discussion for design questions
- Check existing issues before creating duplicates
- Review WARP.md for project-specific context

## License

By contributing, you agree your contributions are under the same license as the project (check LICENSE file).

---

Happy coding! 🦀
