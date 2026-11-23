# ErgoStream paired-world Transfer Atlas v0.1

**Implementation:** `src/atlas.rs`  
**Tracking:** [ERGO-STREAM-ATLAS-001 #7193](https://github.com/Luminous-Dynamics/symtropy/issues/7193)

## Purpose

The existing microcosm tests unit and motif recovery within one generated stream. This small paired-world reference adds the first direct test of structural transfer: can a simple learner map a source structure to a transformed target, and can it abstain when superficial local statistics are compatible with a false analogy?

## Included families


### Relabelled and direction-reversed positive pair

The source and target share the same small relational structure, but node IDs, relation IDs and relation-kind IDs are changed. One directed relation reverses participant order; an undirected relation preserves unordered incidence. The unique-incidence baseline should recover the complete correspondence and the independent validator checks it.

### Structurally similar but intervention-divergent pair

This pair uses the same structural correspondence as the positive relabeled/reversed case. Its policy view exposes two actions belonging to the same source action kind and corresponding target action kind. The evaluator retains qualitative outcomes in which one action preserves a positive sign while another maps a negative source effect to a positive target effect.

A single precommitted transformation for that action kind cannot explain both observations. A candidate that declares one transform receives a counterexample; a candidate that changes the transform between examples is invalid. This demonstrates the key distinction: structural transfer may pass while the proposed behavioral/effect transfer fails.

The effect-sign reference still does not authenticate interventions or prove a real causal mechanism.

### Isomorphic observed structure, different hidden mechanism

This negative pair exposes the same relabeled/reversed graph structure as the positive case through the policy view, but the evaluator's owner-only fixture state marks the underlying mechanism as non-transferable. The visible structural data is therefore insufficient to establish mechanism transfer. The unique-incidence matcher produces a valid structural witness, but the independent evaluator records `FalseUnification`.

This is intentional: a system that sees only the graph should not silently promote a structural isomorphism into a mechanism-transfer claim. The correct epistemic response before an additional discriminating observation is an unresolved / set-valued mechanism hypothesis. The simple graph matcher is expected to fail this higher-level test.

### Degree-matched non-isomorphic negative pair

The source is K3,3 and the target is the triangular prism graph. Both are simple undirected 3-regular graphs with six nodes and nine edges, so every node has the same local degree / incidence signature. They are not isomorphic: K3,3 is bipartite and has no triangles, while the triangular prism contains triangles.

The unique-incidence baseline must abstain because node signatures are ambiguous. It must not invent a unique map from identical degree signatures.

## Evaluator dispositions

The local evaluator distinguishes:
- `TransferAccepted`: a candidate map passed the declared finite structural validator on a positive pair;
- `CorrectAbstention`: no candidate map was submitted on a negative pair;
- `RejectedFalseProposal`: a candidate was submitted on a negative pair, but the structural validator rejected it;
- `MissedTransfer`: a candidate failed validation on a positive pair;
- `FalseUnification`: a structurally valid candidate was accepted on an evaluator-defined negative pair (the isomorphic-observation / different-hidden-mechanism control).

These dispositions are finite-reference diagnostics only. The current positive/negative fixture label is evaluator-owned inside the reference object, but this is **not an OS isolation boundary**. Do not run an untrusted learner in the same process and claim that this API is a sealed blind evaluator. A production evaluation service needs a separate process/container boundary, seed custody, serialized schema allowlists, withheld probe custody, replay receipts and independent verification.

## Important limitations

- Only four tiny paired-world families / controls are present, with hidden mechanism semantics represented by a local owner-side fixture flag rather than an authenticated scientific mechanism model.
- The negative control matches degree signatures, not every possible local statistic or motif statistic.
- The conservative baseline is not a complete VF2 / Nauty / Bliss graph-isomorphism solver.
- The intervention-divergent pair uses evaluator-supplied qualitative effect signs, not an active intervention runner, uncertainty-aware effect estimates, or authenticated physical interventions. Non-identifiability sets, multiscale coarse-graining, and a stream-to-graph learner remain future work.
- No claim of scientific discovery, causal transfer, broad intelligence or SOTA follows from these fixtures.

## Reproduction

```sh
cargo fmt --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml -- --check
cargo test --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml
cargo clippy --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml --all-targets -- -D warnings
```
