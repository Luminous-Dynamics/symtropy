# ErgoStream conservative graph-correspondence baseline v0.1

**Implementation:** `src/matching.rs`  
**Policy name:** `unique-incidence-signature-v0`

## Algorithm

This baseline proposes a finite source-to-target graph correspondence without seeing evaluator-owned labels:

1. For every node, count its participation in directed and undirected relations by relation arity.
2. Group nodes by this incidence signature.
3. Map nodes only when each source signature and its target counterpart occur exactly once. If a signature is shared by multiple nodes, abstain rather than pick an arbitrary permutation.
4. For each source relation, map its participants through the node mapping and find a unique target relation with matching arity and directedness.
5. Infer preserve-order, reverse-order or unordered incidence transformation.
6. Require one consistent target relation kind and participant transform for every instance of a source relation kind.
7. Pass the proposed correspondence through the independent finite structural-witness validator.

## Role in the baseline ladder

This is a weak, interpretable graph-matching control between local pattern matching and more sophisticated graph-isomorphism / graph-edit / embedding methods. It is expected to solve some asymmetric cases and abstain on symmetric or otherwise ambiguous structures.

The baseline is not a complete graph-isomorphism algorithm. It cannot resolve repeated degree/incidence signatures, partial subgraph mappings, or cases where node roles are only distinguishable by higher-order neighborhoods. A strong VF2-style or other admitted graph matcher remains a necessary comparator before claiming an advantage over classical graph matching.

## What it does not establish

- It does not discover causal mechanisms.
- It does not test interventions.
- It does not solve non-identifiable pairs by itself.
- It does not authenticate source/target structures or their provenance.
- It does not provide a security boundary.
- It does not demonstrate structural transfer beyond its declared finite graph input.

The matcher is a candidate proposer only. The separate validator checks the exact witness, and a future independent evaluator must own hidden paired-world truth and intervention outcomes.

## Reproduction

```sh
cargo fmt --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml -- --check
cargo test --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml
cargo clippy --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml --all-targets -- -D warnings
```
