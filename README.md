# ReleaseCheck Core

Rust matching and indexing core for ReleaseCheck.

This repository owns the parts of ReleaseCheck that need to be fast, deterministic, and explainable:

- normalization
- candidate matching
- confidence scoring
- evidence generation
- worker/indexing primitives
- benchmarks and evaluation metrics

The TypeScript product surface lives in the sibling `app` repository.

## Commands

```bash
cargo test --workspace
cargo check --workspace
```

## Repository Layout

```text
crates/rc-core      normalization, scoring, evidence primitives
crates/rc-worker    ingestion and indexing worker placeholder
```
