# ErgoStream microcosm implementation status

**Version:** 0.1 · **Date:** 2026-11-23

This directory contains a deliberately small, dependency-free Rust reference for the first discrete-stream milestone.

## Implemented source surface

- `src/lib.rs` generates deterministic streams of two-frame latent units over a hidden three-unit alphabet.
- A fixed prefix guarantees each unit class appears; a seeded SplitMix64 suffix adds controlled sequence variation.
- A symbol permutation changes the visible alphabet without changing evaluator-owned unit/motif classes.
- `policy_view(end)` returns only the requested available frame prefix, without latent labels or generator seed.
- `evaluate(snapshot)` separately scores exact unit boundaries, unit equivalence classes, adjacent-pair motif boundaries and motif equivalence classes.
- The evaluator uses exact span matches before scoring pairwise class equivalence, preserving segmentation and reuse as separate measures.
- Duplicate proposals, false merging, false splitting, merged spans, invalid prefixes, invalid profiles and the maximum event bound have regression tests.

## Important limits

- This is a **single-channel discrete synthetic microcosm**, not the full multimodal ErgoStream benchmark.
- It has no active query actions, process isolation, OS sandbox, JSONL protocol, cryptographic run receipts, noise model, continuous dynamics, physical simulation or real-world data.
- `WorldV0::evaluate` is owner/evaluator-side API. Do not link it into an untrusted learner process or mistake the Rust module boundary for a security boundary.
- The candidate interface is a reference scoring contract; no learning policy is implemented.

## Verification status

- Twelve Rust unit tests passed (12 passed, 0 failed) on exact PR head `01fe2a3c3987a79611096f8e3827616500232fec`.
- The focused GitHub Actions workflow passed `cargo fmt --check`, `cargo test --locked`, and strict `cargo clippy --all-targets -- -D warnings` using Rust 1.96.0.
- Verified workflow run: [#45232056205](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45232056205), triggered by push at the exact PR head SHA.
- The authoring environment itself has no `rustc` or `cargo`; all Rust qualification above is from the remote runner.
- The repository-wide CI is a separate gate; its current status must be checked independently. Queued/skipped workflows are not passes.

## Reproduction

```sh
cargo fmt --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml -- --check
cargo test --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml
cargo clippy --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml --all-targets -- -D warnings
```

## Structural-transfer witness extension

A follow-on implementation is staged in [ERGO-STREAM-TRANSFER-001 #7171](https://github.com/Luminous-Dynamics/symtropy/issues/7171) and its stacked pull request:

- `src/transfer.rs` validates a total one-to-one map of finite nodes, relation kinds and relation instances.
- Directed relations explicitly preserve or reverse participant order; undirected incidence is compared without order.
- Relation-kind mapping is one-to-one, preventing distinct source relation classes from being collapsed into one target class.
- Intervention-effect mapping is validated separately against supplied qualitative effect observations; the result only records whether all declared effects match the precommitted transform, not whether a causal claim is established.
- No intervention map returns an explicitly unqualified behavior result; missing or contradictory evidence cannot be promoted to support.
- Negative tests cover incidence mismatch, relation-kind collision, non-injective node mapping, incomplete relation mapping, invalid transforms, missing evidence and intervention counterexamples.
- `TRANSFER_WITNESS_V0_1.md` records the claim boundary and reproduction commands.

This is a finite witness validator, not an analogy-discovery learner, causal proof, authenticated observation system, hidden-world service or isolation boundary. It does not yet implement the multi-world generator, false-unification family, active discriminating interventions, or non-identifiability protocol.

**Verification for this extension:** pending exact-head Rust 1.96 formatting, tests and strict Clippy. The previous v0.1 focused pass and repository CI pass remain attached to the parent #7152 head, not this extension.

## Passive acquisition baseline extension

The same stacked research branch now adds `src/acquisition.rs`, exposed as `acquisition`:

- `PassiveGapPatternBaselineV0` is a simple batch baseline using only public sequence, channel, symbol and tick fields.
- It estimates a two-scale timing threshold from the observed smallest and largest inter-frame gaps, segments where a gap exceeds the midpoint, clusters exact observed event signatures, and forms adjacent-pair motifs.
- It explicitly omits an unfinished final segment unless the caller marks end-of-stream.
- It abstains with an empty snapshot when the stream has no separable timing scale.
- It rejects inconsistent prefixes, non-contiguous sequence numbers and non-monotonic timestamps.
- Tests compare its output against the existing hidden evaluator, include alphabet permutation, open-tail, no-gap abstention and malformed-time/sequence controls.

This is intentionally a transparent calibration baseline tailored to the current two-scale synthetic generator. Perfect recovery on this generator is not evidence of general structural discovery; the purpose is to establish a reproducible lower-complexity comparator and expose how much the first generator rewards a simple timing/pattern heuristic.

**Verification for this extension:** pending on its exact latest head; do not infer from the earlier witness-only pass.

## Conservative graph-correspondence baseline extension

The branch also adds `src/matching.rs`, exposed as `matching`:

- `UniqueIncidenceSignatureBaselineV0` proposes a source-target node mapping only when every directed/undirected incidence signature is unique.
- It matches relation instances under explicit preserve-order, reverse-order or unordered transforms.
- It requires a consistent one-to-one relation-kind mapping and validates its candidate through the separate structural-witness validator.
- It abstains on symmetric / ambiguous node signatures instead of making an arbitrary mapping.
- Tests cover a relabelled and reversed asymmetric fixture, ambiguous symmetric leaves, and inconsistent target relation kinds.
- `GRAPH_MATCHING_BASELINE_V0_1.md` documents the algorithm and its limitations.

This is a conservative degree/incidence-signature baseline, not a complete graph-isomorphism or graph-edit solver. A strong VF2-style / graph-edit comparator remains required before making claims against classical graph matching.

**Verification for the combined transfer, passive extractor and graph-matching branch:** pending on the exact latest head.

## Paired-world Transfer Atlas v0.1 extension

The branch now adds `src/atlas.rs`, exposed as `atlas`, and `TRANSFER_ATLAS_V0_1.md`:

- Positive family: a three-node structure under node/relation/kind relabeling and directed participant reversal.
- Negative family: K3,3 versus the triangular prism. Both are six-node, nine-edge, undirected 3-regular graphs with identical local degree signatures, but they are non-isomorphic (bipartite / triangle-free versus containing triangles).
- The owner-side fixture stores the expected transfer disposition outside the learner-facing pair view.
- The unique-incidence baseline must transfer the positive pair and abstain on the degree-matched negative pair.
- A local evaluator distinguishes accepted transfer, correct abstention, missed transfer and false unification.

This is still an in-process reference and **not a sealed hidden-world service or OS security boundary**. The three reference families do not yet cover matched higher-order marginals, active discriminating probes, non-identifiable map sets, multiscale structure or stream-to-graph learning. The intervention-divergent fixture uses supplied qualitative signs, not an authenticated intervention runner.

**Verification for the latest stacked branch, including the paired-world atlas:** pending exact-head Rust 1.96 format, tests and strict Clippy. The previous run on head `187c9621c5a5a947629628ab1994bcbcc4a1db40` passed formatting but had 32/33 tests pass; the sole failure was an ambiguous graph-matcher test fixture, now corrected with uniquely identifiable unary relation incidence. The combined atlas extension is newer and has not yet been qualified.


## Intervention-divergent Transfer Atlas extension

The paired-world Atlas now includes `StructurallySimilarInterventionDivergent`:

- The source and target structures are still structurally isomorphic under the declared relabeling / reversal.
- Two source actions share one action kind and map to one target action kind.
- The evaluator-owned qualitative outcomes cannot be explained by one consistent `Preserve` or `Negate` transform for that action kind.
- A candidate with one declared transform receives an intervention counterexample; changing the transform per example is rejected as `InconsistentActionKindTransform`.
- The positive relabelled pair has a separate qualitative-sign match control.

The action correspondence contract now includes source/target action-kind IDs and requires a one-to-one action-kind mapping with a single effect transform per source action kind. The result label remains `all_declared_effects_match`, not causal qualification.

This test still uses qualitative reference signs, not an authenticated intervention runner or a physical causal experiment. It establishes that the finite evaluator can separate structural correspondence from an inconsistent proposed effect transform; it does not establish transportable causal mechanisms.

**Verification for the latest combined branch:** pending exact-head Rust 1.96 format, tests and strict Clippy.

## Transfer Atlas v0.2 protocol

Added `TRANSFER_ATLAS_V0_2_PROTOCOL.md` to specify the next full benchmark phase:

- evaluator / learner process custody and serialized policy-view boundaries;
- freeze-before-reveal ordering for structural maps and action/effect transforms;
- nine paired-world generator families, including matched-marginal, non-identifiable, multiscale and cross-modal cases;
- acquisition, graph-matching, causal-discovery, direct-policy and world-model baseline ladders;
- separate matched-budget and best-available comparisons;
- capability, epistemic reliability, efficiency and prediction-to-action scorecards;
- family-held-out splitting, statistical power analysis, pre-registration, full failed-run denominators and replay artifacts;
- prospective go/no-go targets and explicit falsification conditions.

This is a protocol document, not evidence that the v0.2 benchmark is implemented or qualified.

## Finite action-conditioned dynamics-transfer witness

The branch now adds `src/dynamics.rs`, exposed as `dynamics`, and `DYNAMICS_TRANSFER_V0_1.md`:

- Represents a deterministic finite transition system as states, actions and transitions.
- Validates total one-to-one state and action correspondences.
- Requires a one-to-one, consistent source/target action-kind map.
- Checks exact transition commutation under the state/action maps: `phi(delta_source(x, a)) == delta_target(phi(x), psi(a))`.
- Rejects nondeterministic systems, incomplete or non-injective maps, action-kind collisions, missing target transitions and an explicit transition counterexample.
- Tests include a relabelled positive system with four commuting transitions and negative controls for state/action map integrity and transition mismatch.

This is a stronger finite dynamics test than graph correspondence or qualitative effect-sign matching, but it remains a validator for a proposed witness. It does not discover mappings from streams, infer causal mechanisms, authenticate interventions, estimate uncertainty or qualify physical-world transfer. It is not a continuous-time CfC/LTC equivalence test. A continuous extension needs explicit clock transformation, observation delay/sampling, uncertainty, error bounds and held-out rollout / intervention contracts.

**Verification for the combined transfer, acquisition, matching, atlas and dynamics branch:** pending exact-head Rust 1.96 format, tests and strict Clippy.

## Latest exact-head test finding

The focused run on head `2a68852ac613f625abf095d8a6f73f594963f17c` passed `cargo fmt --check` and all **50 unit tests**. Its doctest then failed because the indented mathematical transition equation in the module-level Rust documentation was parsed as a Rust code example. The same run also reported an unused import in `atlas.rs`. No Clippy result was produced because the test command stopped the workflow.

Both narrow issues have been corrected:
- the equation is now an inline code span, not an executable doctest;
- the test-only effect-transform import is now scoped to the test module.

Corrected head: `2315e78f5e2886f8367a4619743be5576c1acd5b`. A new exact-head Rust 1.96 run is required before qualification.

## Continuous-time / hybrid dynamics transfer protocol

Added `CONTINUOUS_DYNAMICS_TRANSFER_V0_2_PROTOCOL.md` as the next research extension. It specifies:
- explicit state, action, clock/time and hybrid-mode correspondences;
- trajectory-level approximation rather than frame-index matching;
- measurement-time versus arrival-time separation, clock uncertainty, irregular sampling and delayed/missing observations;
- hybrid event guards, resets, impacts and contact discontinuities;
- separation of initial-state, parameter, structural, process, measurement, clock and model-discrepancy uncertainty;
- local dynamics, long-horizon trajectory, event-time, intervention and downstream action-utility metrics;
- system-identification, discrete recurrent, Neural ODE / continuous-time, CfC-only, LTC-only, graph-only, HDC/CfC hybrid and action-conditioned transformer baselines;
- negative controls for false clock alignment, wrong vector fields, wrong reset maps and out-of-support action/state pairs.

The protocol explicitly does not claim a continuous-time learner or a continuous CfC/LTC equivalence result. The executable #7173 module remains a deterministic finite-transition correspondence witness.

## Exact-head Rust verification

The focused workflow run [#45268504387](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45268504387) passed on code head `bb11dc83015f63afbcddba6c00ee92fdce5fc23e` using Rust 1.96.0:
- `cargo fmt --check`: passed;
- `cargo test --locked`: passed, including 50 unit tests and the dynamics module doctest;
- strict `cargo clippy --all-targets -- -D warnings`: passed.

After this code head, the only branch change is the documentation-only continuous dynamics protocol at `579c93217cac34196e25a78d190417948d4e3528`. This verification applies to all executable source on the current branch. It is not an OS-sandbox, full blind evaluator, multi-family benchmark, or learner-performance result.
