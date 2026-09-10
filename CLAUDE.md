# CLAUDE.md

## What this is
A high performance share library in Rust with modular components for decentralized finance system engineering.
It contains basic primitives and components to build blockchains, decentralized exchanges and high performance
trading platforms.

## Commands
- Build everything:        cargo build --workspace
- Test everything:         cargo nextest run --workspace
- Lint (must pass in CI):  cargo clippy --workspace --all-targets -- -D warnings
- Format:                  cargo fmt --all
- Full CI check locally:   cargo xtask ci

## Workspace layout
- utilities      — basic utilities for the workspace.

## Hard constraints
- Cargo.lock is committed. Do not add it to .gitignore.
- New shared dependencies go in [workspace.dependencies] at the root, then
  referenced as `dep = { workspace = true }` in member Cargo.toml — don't
  pin versions ad hoc in individual crates.

## Conventions
- Errors: thiserror in crates/*, anyhow only in services/*/src/main.rs.
- Logging: tracing, never println!/eprintln! outside xtask.

## Before opening a PR
- cargo xtask ci passes locally
- cargo deny check passes (license/advisory check)
- run tests for all packages, not just the one you're working on