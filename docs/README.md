# RUnits Documentation

This directory contains extended documentation beyond the rustdoc API reference.

## Contents

- **[roadmap.md](roadmap.md)** — "Source of truth" for the project status, next phases, and feature catalog
- **[gnu-units-parity.md](gnu-units-parity.md)** — Gap analysis vs GNU Units 2.25: what's covered, what's missing, standout opportunities

## Auto-Generated API Documentation

The main API documentation is auto-generated from rustdoc comments:

- **Local:** `cargo doc --open`
- **Online:** GitHub Pages (updated automatically by CI on every push to `main`)

## Getting Started

For usage examples see the crate-level docs in `src/lib.rs`, or run:

```bash
cargo doc --open
```
