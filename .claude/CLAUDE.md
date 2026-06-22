# CLAUDE.md

This file contains behavioral instructions for Claude when working in this repository. All user-facing documentation is in README.md.

## Core Behavioral Guidelines

`akd-watch` audits Auditable Key Directories (AKDs) for integrity, protecting against split-world attacks in key transparency systems. It is a Rust workspace with four crates:

- **`common`** — shared config, storage backends, crypto, and protobuf definitions
- **`auditor`** — core auditing logic; `namespace_auditor.rs` is the heart of the algorithm
- **`web`** — Axum-based REST API server that exposes audit results
- **`aio`** — combined binary that runs auditor + web concurrently (recommended for deployment)

## Commands

```sh
# Build
cargo build --release

# Test
cargo test --workspace --all-features

# Lint (all three must pass CI)
cargo clippy --all-features --all-targets
cargo +nightly fmt --check  # pinned nightly version in rust-toolchain.toml (nightly-channel field)
cargo sort --workspace --check
```

## Architecture notes

- **Storage is trait-based.** Backends (Azure Blob Storage or filesystem) are swapped via `config.toml` at runtime, not at compile time.
- **Protobuf codegen.** `crates/common/build.rs` runs `prost-build`. Edit `.proto` files under `crates/common/src/proto/specs/` to regenerate.
- **Testing feature.** The `common` crate exposes a `testing` feature used by tests across crates.
- **Clippy denies.** The workspace-level `Cargo.toml` denies `unused_async` and `unwrap_used`. Use `?` or `anyhow` context instead of `.unwrap()`.
