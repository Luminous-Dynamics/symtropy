# Evidence validity and scoped-invalidation benchmark

This standalone Rust tool is the first executable slice for Symtropy issue #1846. It tests a narrow but safety-relevant lifecycle contract: when a construction, survey, procurement, material, or commissioning input changes, exactly the explicitly dependent evidence becomes stale, while unrelated evidence remains usable.

## Run from the repository root

    cargo fmt --manifest-path tools/evidence-invalidation-benchmark/Cargo.toml -- --check
    cargo test --manifest-path tools/evidence-invalidation-benchmark/Cargo.toml
    SOURCE_REVISION="$(git rev-parse HEAD)" cargo run --manifest-path tools/evidence-invalidation-benchmark/Cargo.toml

The CLI defaults to the frozen fixture at tools/evidence-invalidation-benchmark/fixtures/evidence-validity-v0-1.json. An alternate manifest path can be passed as the first argument. Reports include the exact manifest-byte SHA-256 and a source revision. In CI, SOURCE_REVISION is set to the checked-out commit.

## What is checked

- Strict schema parsing, unique artifact IDs, resolved dependencies, acyclic dependency graph, exact claim/scope references, and valid digest encodings.
- SHA-256 of the exact source-content bytes for each evidence artifact.
- Trigger-specific invalidation on the first edge, followed by transitive invalidation of derived descendants.
- Separate superseded, stale, revoked, unknown, incomplete_coverage, contradictory, and out_of_scope states.
- A graph-search production selector checked against an independently structured repeated-scan oracle.
- Frozen expected outcomes, exact claim dispositions, duplicate-event idempotency, conflicting duplicate rejection, input-order invariance, and precision controls for unrelated furnishings.
- Claims are accepted only when source digest, evidence status, claim ID, spatial/scope ID, and coverage requirements pass.

## Claim ceiling

A PASS means only that the implementation conforms to this finite synthetic contract. It does not establish real survey validity, structural safety, material composition, certification, permit eligibility, legal authority, environmental impact, circularity, or suitability of any real site. No physical action is initiated. Queue, missing, skipped, or stale-head CI checks are not passes.

The benchmark is intentionally not a circularity scalar or lifecycle optimizer. Before ranking repair, reuse, deconstruction, recycling, or transformation options, those alternatives need compatible functional units, source-scoped material/energy flows, uncertainty, and domain-specific acceptance criteria. Symthaea can propose hypotheses; Symtropy owns scenario evaluation; Mycelix owns provenance and decision accountability; Sol-Atlas owns spatial projection. Each keeps its own authority boundary.
