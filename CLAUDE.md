# CLAUDE.md - vauchi-tui

Terminal user interface for vauchi.

## Rules

- Depends on `vauchi-core`
- TUI for power users and testing

## Commands

```bash
cargo run -p vauchi-tui                     # Run TUI
cargo test -p vauchi-tui                    # Run tests
```

## Local Development

Uses `.cargo/config.toml` to patch git dependency to local path.
Ensure `../core/vauchi-core` exists for local builds.
