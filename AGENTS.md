# ReleaseCheck Core

This repository is the Rust core for ReleaseCheck.

It should contain:

- normalization logic
- fuzzy matching
- confidence scoring
- matching evidence generation
- ingestion and indexing worker code
- benchmarks for search and matching paths

Repository boundary:

1. Keep this repo Rust-only unless the user explicitly asks otherwise.
2. Do not add Bun, TypeScript app code, web UI, or SDK code here. Those belong in `../app`.
3. Public interfaces should be explicit enough for the app repo to call later through a stable boundary such as HTTP, CLI, FFI, or generated bindings.
4. Matching decisions must return evidence, not only a numeric confidence score.
5. Platform-specific code should sit behind adapters. The core matching engine should not depend on raw Spotify, Melon, YouTube, MusicBrainz, or Discogs response shapes.
6. Prefer deterministic rules and tests before adding heavier ranking or ML-like behavior.
7. Performance-sensitive code should be benchmarkable. Avoid clever code that cannot be measured or explained.

Performance priorities:

- Fast normalization for artist, title, album, and version strings.
- Low-allocation matching for hot paths.
- Batch-friendly APIs.
- Clear latency targets for search/index/match paths.
- Explainable score components such as title, artist, duration, date, and ISRC.

Commands:

- `cargo test --workspace`: run the Rust test suite.
- `cargo check --workspace`: fast compile check.

Current structure:

- `crates/rc-core`: normalization, scoring, and evidence primitives.
- `crates/rc-worker`: ingestion/indexing worker placeholder.
