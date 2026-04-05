# RUnits Documentation

This directory contains extended documentation beyond the rustdoc API reference.

## Contents

- **[roadmap.md](roadmap.md)** — Project status, next phases, and feature catalog (also embedded into rustdoc as the [`roadmap`](../src/lib.rs) module)
- **[learning-notes.md](learning-notes.md)** — Key Rust concepts learned during development

## Auto-Generated API Documentation

The main API documentation is auto-generated from rustdoc comments:

- **Local:** `cargo doc --open`
- **Online:** GitHub Pages (updated automatically by CI on every push to `main`)

## Getting Started

For usage examples see the crate-level docs in `src/lib.rs`, or run:

```bash
cargo doc --open
```
