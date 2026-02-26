<!-- SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me> -->
<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# CLAUDE.md - vauchi-tui

Terminal user interface for vauchi.

## Rules

- Depends on `vauchi-core`
- TUI for power users and testing

## Commands

```bash
cargo run -p vauchi-tui                     # Run TUI
just test tui                               # Run tests
just check tui                              # Format + lint + test
```

## Local Development

Uses `.cargo/config.toml` to patch git dependency to local path.
Ensure `../core/vauchi-core` exists for local builds.
