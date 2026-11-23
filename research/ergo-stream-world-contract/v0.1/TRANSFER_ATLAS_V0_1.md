# ErgoStream paired-world Transfer Atlas v0.1

**Implementation:** `src/atlas.rs`  
**Tracking:** [ERGO-STREAM-ATLAS-001 #7193](https://github.com/Luminous-Dynamics/symtropy/issues/7193)

## Purpose

The existing microcosm tests unit and motif recovery within one generated stream. This small paired-world reference adds the first direct test of structural transfer: can a simple learner map a source structure to a transformed target, and can it abstain when superficial local statistics are compatible with a false analogy?

## Included families

### Relabelled and direction-reversed positive pair

The source and target share the same small relational structure, but node IDs, relation IDs and relation-kind IDs are changed. One directed relation reverses participant order; an undirected relation preserves unordered incidence. The unique-incidence baseline should recover the complete correspondence and the independent validator checks it.

### Degree-matched non-isomorphic negative pair

The source is K3,3 and the target is the triangular prism graph. Both are simple undirected 3-regular graphs with six nodes and nine edges, so every node has the same local degree / incidence signature. They are not isomorphic: K3,3 is bipartite and has no triangles, while the triangular prism contains triangles.

The unique-incidence baseline must abstain because node signatures are ambiguous. It must not invent a unique map from identical degree signatures.

## Evaluator dispositions

The local evaluator distinguishes:
- `TransferAccepted`: a candidate map passed the declared finite structural validator on a positive pair;
- `CorrectAbstention`: the candidate abstained or failed validation on a negative pair;
- `MissedTransfer`: a candidate failed on a positive pair;
- `FalseUnification`: a structurally valid candidate was accepted on a negative pair.

These dispositions are finite-reference diagnostics only. The current positive/negative fixture label is evaluator-owned inside the reference object, but this is **not an OS isolation boundary**. Do not run an untrusted learner in the same process and claim that this API is a sealed blind evaluator. A production evaluation service needs a separate process/container boundary, seed custody, serialized schema allowlists, withheld probe custody, replay receipts and independent verification.

## Important limitations

- Only two tiny graph-pair families are present.
- The negative control matches degree signatures, not every possible local statistic or motif statistic.
- The conservative baseline is not a complete VF2 / Nauty / Bliss graph-isomorphism solver.
- The reference does not yet add active discriminating interventions, non-identifiability sets, multiscale coarse-graining, or a stream-to-graph learner.
- No claim of scientific discovery, causal transfer, broad intelligence or SOTA follows from these fixtures.

## Reproduction

```sh
cargo fmt --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml -- --check
cargo test --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml
cargo clippy --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml --all-targets -- -D warnings
```
