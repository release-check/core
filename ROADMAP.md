# ReleaseCheck Core Roadmap

This roadmap covers the Rust core repository.

`AGENTS.md` defines repo rules. This file defines implementation order.

## Purpose

Build the fast, explainable candidate matching engine behind ReleaseCheck.

The core repo owns:

- normalization
- fuzzy matching
- confidence scoring
- evidence generation
- indexing and worker primitives
- benchmarks
- evaluation metrics

## Non-Goals

- Do not add TypeScript app code here.
- Do not add Bun workspace files here.
- Do not depend on raw platform response shapes inside the matching engine.
- Do not start with ML/vector ranking before deterministic matching is strong.
- Do not optimize code that has no benchmark or correctness test.

## Matching Philosophy

Core should avoid overconfident merging.

Rules:

- Return candidates, not forced truth.
- Prefer `unknown` over false certainty.
- Treat strong platform false positives as severe failures.
- Keep same-name tracks, remixes, live versions, demos, remasters, and alternate versions separable.
- Every score should have evidence.

## Phase 0: Core Skeleton

Target: now to 2026-07-17.

Must-have:

- Keep `cargo test --workspace` green.
- Keep `crates/rc-core` focused on pure matching primitives.
- Keep `crates/rc-worker` as the ingestion/indexing boundary.
- Define candidate, platform status, confidence, and evidence structures.

Nice-to-have:

- Add `.gitignore` for Rust build output.
- Add benchmark harness.
- Add fixture format for demo candidates.

Cut scope:

- Platform-specific adapters before the candidate shape is stable.
- Persistent database integration.

## Phase 1: Candidate Matching Proof

Target: 2026-07-18 to 2026-08-10.

Must-have:

- Normalization rules for:
  - artist names
  - aliases
  - track titles
  - album titles
  - `feat` / `ft`
  - remix
  - live
  - demo
  - remaster
  - instrumental
  - sped up
- Candidate model independent of platform APIs.
- Confidence scoring with evidence fields.
- Unit tests for:
  - exact match
  - alias match
  - title casing and spacing
  - feature marker normalization
  - duration mismatch
  - URL/source evidence
  - version mismatch
  - false-positive platform availability
- Deterministic output for curated fixtures.

Nice-to-have:

- Trigram or edit-distance scorer.
- Batch scoring API.
- Worker pipeline for fixture ingestion.

Cut scope:

- Full crawler.
- OpenSearch/Elasticsearch integration.
- Vector reranking.

## Phase 2: Evaluation And Speed

Target: 2026-08-11 to 2026-08-27.

Must-have:

- Golden set support:
  - 5-10 hand-verified cases per scene.
  - Exact URLs.
  - Canonical artist/title.
  - Version distinction.
  - Matching rationale.
  - Acceptable top-3 candidates.
  - Target: 100% pass.
- Evaluation set support:
  - 20-50 cases per scene.
  - Platform-level `available`, `missing`, `unknown` labels.
  - Initial scene-level top-3 correct rate >= 80%.
- False positive tracking:
  - Strong platform false positives <= 2-5%.
  - Missing `unknown` on uncertain cases should be nearly zero.
- Benchmark report includes:
  - p50
  - p95
  - p99
  - dataset size
  - query count
  - cache/index mode
  - machine/context

Performance target:

- Normal indexed/cached cases should stay in the 1-second range from the product perspective.
- Core matching/index paths should be fast enough that app overhead and platform refresh do not dominate normal cases.

Nice-to-have:

- Criterion benchmarks.
- Lightweight local index.
- Worker refresh simulation.
- Serialized output for app demo ingestion.

Cut scope:

- Distributed worker system.
- Full public metadata graph.
- Production-grade platform sync.
- ML ranking.

## Verification

Required before handoff:

```bash
cargo test --workspace
```

When benchmark tooling exists, also run the benchmark command and store a short report in docs or artifacts.

## Open Questions

- What is the first stable boundary between `core` and `app`: CLI JSON, local HTTP, generated files, or bindings?
- Which matching rules are hard filters versus weighted score components?
- How strict should version markers be for live/remix/demo/remaster matches?
- Which concrete tracks enter the first golden set?
