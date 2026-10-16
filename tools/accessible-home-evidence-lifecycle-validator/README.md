# Accessible Home Evidence Lifecycle Validator

Standalone Rust replay and semantic validator for the **synthetic-only** lifecycle contract in
\`docs/built-environment/accessible-home-evidence-lifecycle-v0.1.example.json\`.

## Run locally

From the repository root:

\`\`\`sh
cargo fmt --manifest-path tools/accessible-home-evidence-lifecycle-validator/Cargo.toml -- --check
cargo test --manifest-path tools/accessible-home-evidence-lifecycle-validator/Cargo.toml
cargo run --manifest-path tools/accessible-home-evidence-lifecycle-validator/Cargo.toml -- \
  --input docs/built-environment/accessible-home-evidence-lifecycle-v0.1.example.json \
  --output /tmp/accessible-home-lifecycle-receipt.json
\`\`\`

Invalid JSON, unknown fields, unknown references, unsupported versions, conflicting current claims,
supersession cycles, invalid event order, stale revision starts, expectation mismatches and expired
required evidence fail closed with exit code 2.

## Semantic rules

- Each change event must start from the current revision of the named facet and advance it.
- Events must be time-ordered and must not occur after the fixture evaluation time.
- Claims bind to explicit facet revisions. A change invalidates only claims bound to the changed facet.
- Review-due evidence is non-current at or before its deadline.
- Supersession cycles fail; incompatible current claims for one scope become an explicit conflict.
- A current declared hard barrier blocks a path even if another required claim is stale.
- A stale barrier applies only to the old configuration; the new configuration remains not evaluable until revalidation.
- Every claim and path declares an expected outcome; the evaluator independently derives it and rejects disagreement.
- The receipt includes a digest of exact input bytes. That digest establishes byte identity only, never truth,
  anonymity, safety or accessibility.

## Test corpus and near-term gates

The unit tests are synthetic negative controls for the valid fixture, unrelated-facet changes, stale
revision starts, future events, expiry, barrier precedence, stale barriers, conflicting claims, cycles,
duplicate event IDs, dangling facet/path references, real-home fixture rejection, deterministic replay,
exact-byte receipts and unknown revisions.

Proposed correctness targets (not yet achieved or independently audited):

- 100% of defined mutation controls rejected or assigned the expected fail-closed state.
- 0 unrelated claims invalidated solely by a change outside their declared facet bindings.
- 100% of invalidation findings identify the changed facet(s).
- 100% of expired, stale, superseded or conflicted required claims remain non-passing.
- Identical input bytes and pinned validator build yield identical receipts.
- 0 real resident payloads, actual floorplans, addresses or raw plan digests in public fixtures or receipts.

Proposed scale target for a future benchmark, not a measured result: replay 10,000 claims, 1,000 change
events and 1,000 paths in under 1 second and under 256 MiB peak RSS on a documented reference machine.
Measure a baseline before treating the target as credible.

## Limits

This validates internal consistency of declared synthetic evidence. It does not inspect real geometry,
measure clearances, simulate a resident, validate evacuation, assess assistive technology, establish code
compliance, or qualify any home or safety system. Real projects still require the resident's informed
direction, field measurement, appropriate professional review, jurisdiction-specific requirements and
privacy safeguards. A successful receipt is never a certificate.
