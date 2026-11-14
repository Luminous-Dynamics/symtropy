# Connectome Evidence Contract V1

A dependency-free Rust boundary between source connectome edge tables and downstream graph / causal-mechanism benchmarks.

**Parent issue:** [#3534 — LOCO-EVIDENCE-001](https://github.com/Luminous-Dynamics/symtropy/issues/3534)  
**Related work:** [#3507](https://github.com/Luminous-Dynamics/symtropy/pull/3507), [#3508](https://github.com/Luminous-Dynamics/symtropy/issues/3508), [#3511](https://github.com/Luminous-Dynamics/symtropy/issues/3511), [#3518](https://github.com/Luminous-Dynamics/symtropy/issues/3518), [#3519](https://github.com/Luminous-Dynamics/symtropy/issues/3519), [#3520](https://github.com/Luminous-Dynamics/symtropy/pull/3520).

## The contract

Connectome edge records, endpoint identity, annotation state, and source coverage are different things. V1 preserves those distinctions rather than collapsing them into a convenient but misleading typed graph.

### 1. Source snapshot identity

Every import binds an edge table to:
- dataset identifier;
- public release and optional CAVE/materialization identifier;
- edge-table revision;
- exact source archive filename;
- SHA-256 digest of the acquired archive bytes;
- declared edge-table coverage scope.

Sha256Digest::parse accepts exactly 64 hexadecimal characters. This crate validates the supplied digest; it does not download files or hash an archive on behalf of an adapter.

### 2. Raw endpoint identity is not annotation identity

Each endpoint retains its source raw_endpoint_id. Annotation is separate:
- CellTyped: canonical/root neuron ID, cell-type label and independent proofreading flag;
- IdentifiedUnclassified: canonical/root neuron ID, proofreading flag and a reason for the missing type;
- Unresolved: raw endpoint ID and explicit reason; no canonical ID is invented.

Raw IDs are snapshot-local source identities. Never substitute an array index. Never join two snapshots solely because their row positions happen to match. An adapter must preserve the source's mapping and annotation lineage.

Repeated appearances of a raw endpoint must carry consistent annotation in the same frozen snapshot. Conflicts fail closed.

### 3. Coverage has explicit denominators

coverage_summary() counts presynaptic and postsynaptic endpoint occurrences independently, with an explicit denominator equal to the number of edge records. It separately counts edges with both endpoints identified, proofread, or cell-typed. These are edge-record coverage measures, not unique-neuron coverage.

This allows source-reported quantities such as presynaptic-link coverage, postsynaptic-link coverage and both-endpoint coverage to be represented without making one side stand in for the other.

### 4. Typed projection is lossy and must say how

project_typed() produces a typed graph only after the caller selects either:
- RequireCellType; or
- RequireCellTypeAndProofread.

Every source edge is accounted for as included or assigned one explicit, mutually exclusive exclusion category. The returned counters distinguish missing endpoint identities, missing cell types and insufficient proofreading. They are deliberately not silent filter_map operations.

A projected graph is not asserted to be complete, unbiased or representative of the source connectome.

### 5. Unknown is not zero

lookup_raw_edge() returns:
- Observed with aggregate observed synapse count when matching source endpoint IDs exist;
- NotObservedInCompleteSnapshotScope only when the manifest declares the edge table complete within a named snapshot scope;
- UnknownUnderPartialCoverage when the manifest declares partial coverage.

"Not observed in a complete snapshot scope" is a statement about the frozen source table and its declared scope. It is not a claim of biological impossibility or proof that no synapse exists outside the scope.

## Synthetic verification

The tests cover malformed digests/manifests, duplicate edge IDs, conflicting endpoint annotations, zero weights, denominator-explicit coverage, separate pre/post missingness, auditable typed projection, proofread-only exclusion, and the distinct lookup outcomes for observed / not-observed-in-complete-snapshot / unknown-under-partial-coverage pairs.

All fixtures are synthetic. Passing tests qualifies only this crate's software behavior; it does not reproduce the BANC coverage percentages, validate an adapter, or constitute biological evidence.

## Run

```sh
cargo fmt --manifest-path research/connectome-evidence-contract-v1/Cargo.toml -- --check
cargo test --manifest-path research/connectome-evidence-contract-v1/Cargo.toml --locked
cargo clippy --manifest-path research/connectome-evidence-contract-v1/Cargo.toml --locked --all-targets -- -D warnings
```

## Next research step

Build source-format adapters only after acquiring a bounded, source-pinned archive subset. Each adapter should retain exact raw IDs, source annotation lineage, release/materialization identity and SHA-256 of the acquired bytes; then compare independently computed coverage reports with the source publication before using projected graphs in null-model or intervention experiments.

Useful source anchors:
- Pugliese et al. 2026 DNg100/VNC connectome reproduction: https://doi.org/10.1038/s41586-026-11195-y
- Bates et al. 2026 brain-and-cord connectome: https://doi.org/10.1038/s41586-026-10735-w
- Dallmann et al. 2025 proprioceptive gating: https://doi.org/10.1038/s41586-025-09554-2
