# ErgoStream scoped structural transfer v0.2

**Implementation:** `src/scoped.rs`  
**Wire contract:** `schemas/scoped_transfer_proposal_v0.schema.json`  
**Tracking:** [ERGO-STREAM-ATLAS-001 #7193](https://github.com/Luminous-Dynamics/symtropy/issues/7193)

## Why scope must be first-class

The v0.1 structural witness correctly demanded a total one-to-one map over the complete supplied structures. That remains the strict isomorphism floor. It is too restrictive for many real transfer questions, however: a mechanism may be embedded in a larger system, or a reusable source subgraph may correspond to only one declared target region.

The v0.2 scoped witness adds explicit map scope while retaining the v0.1 structural validator over the selected substructures. It never silently discards the rest of either world.

## Implemented map kinds

### Isomorphism

Both source and target selected scopes must include every node and relation. Excluded ID lists must be empty. The existing total structural witness then checks the complete one-to-one mapping.

### Induced embedding

The candidate selects a source node region and a target node region. Every relation wholly contained in each selected node region must be included in that side's selected relation scope. The selected source and target induced substructures must then correspond bijectively.

This prevents a candidate from ignoring an extra internal target relation while still claiming an induced embedding.

### Subgraph embedding

The candidate selects a source subgraph and a target subgraph. Additional target relations may be excluded, but their IDs must appear in `excluded_relation_ids`. The selected subgraphs must correspond bijectively.

This is deliberately weaker than an induced embedding. It says that the declared source subgraph exists in the target; it does not say the target region has no additional internal relationships.

## Scope-accounting contract

For each side, selected and excluded node IDs must:
- be unique within their respective lists;
- be disjoint;
- refer to existing entities;
- partition the complete node set.

Selected and excluded relation IDs must satisfy the same rules for the complete relation set. Every selected relation must have all participants inside the selected node scope.

The candidate correspondence must be total and one-to-one over exactly the selected source and target substructures. The witness reports how many source and target nodes/relations were excluded, so the output carries an explicit boundary.

A machine-readable example is provided at [`examples/scoped_transfer_induced_embedding.example.json`](examples/scoped_transfer_induced_embedding.example.json). It illustrates a source substructure embedded into a target while retaining excluded source and target context and binding both sides to evidence-prefix digests.

## What remains unsupported

This module does not implement:
- many-to-one homomorphisms;
- quotient maps / coarse-graining;
- one-way transition simulation;
- bisimulation;
- approximate simulation or approximate bisimulation;
- continuous-time conjugacy / semiconjugacy.

Those are distinct mathematical contracts, not flags that can safely reuse an isomorphism validator. Each needs its own soundness conditions and tests. The v0.2 wire schema intentionally admits only the three implemented map kinds.

## Reproduction

```sh
cargo fmt --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml -- --check
cargo test --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml
cargo clippy --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml --all-targets -- -D warnings
```

The scope witness is a finite structural validator. It does not discover the correspondence, establish causal-mechanism transfer, or authenticate hidden-world custody.
