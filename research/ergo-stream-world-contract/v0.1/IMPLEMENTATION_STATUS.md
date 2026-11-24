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

**Historical note:** this status line was superseded by the later exact-head verification recorded below.

## Latest exact-head test finding

The focused run on head `2a68852ac613f625abf095d8a6f73f594963f17c` passed `cargo fmt --check` and all **50 unit tests**, but its doctest interpreted the indented mathematical transition equation in the module-level Rust documentation as executable Rust. That run also exposed an unused library-scope import in `atlas.rs`, and Clippy did not run.

Both narrow issues have been corrected:
- the formula was removed from the Rust module comment and retained in the standalone Markdown dynamics contract;
- the test-only effect-transform import was scoped to the test module.

These corrections were subsequently verified by the exact-source workflow recorded below.

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

At the time of that run, changes after the verified code head were documentation-only. Subsequent Atlas code changes described below are newer and are **not** covered by that pass. The prior result remains valid for the earlier executable code snapshot only.

## False-proposal versus abstention scoring correction

The Atlas evaluator now distinguishes three different negative-pair outcomes:
- `CorrectAbstention`: the learner submitted no candidate;
- `RejectedFalseProposal`: the learner submitted a candidate mapping, but the independent structural validator rejected it;
- `FalseUnification`: the proposed structure is a valid correspondence, but owner-side mechanism truth says transfer does not hold.

This prevents an invalid candidate map from being misreported as an abstention.

A fourth Atlas control, `IsomorphicObservationDifferentMechanism`, exposes the same policy-visible relabeled structure as the positive isomorphism case while keeping the non-transferable mechanism disposition evaluator-side. The unique-incidence baseline finds a valid graph map and is scored as `FalseUnification`. This shows why structural isomorphism must not automatically authorize mechanism transfer. In a real sealed episode, the correct result must remain unresolved until a permitted discriminating observation or intervention separates the hypotheses.

The visible fixture flag is only a local reference oracle, not a process-isolated hidden evaluator or authenticated physical mechanism model. The simple matcher is intentionally expected to fail this semantic negative control.

**Verification:** the above source/tests are newer than code head `bb11dc83015f63afbcddba6c00ee92fdce5fc23e`; their exact-head formatting, tests and strict Clippy are pending.

## Latest Atlas scoring / false-unification extension

The Atlas has since been extended in `src/atlas.rs`:

- `CorrectAbstention` now means exactly that the learner submitted no candidate.
- `RejectedFalseProposal` means a candidate was submitted but the independent structural validator rejected it.
- `FalseUnification` means a structurally valid correspondence is accepted even though evaluator-owned mechanism truth says transfer does not hold.
- The new `IsomorphicObservationDifferentMechanism` control presents the same policy-visible relabeled structure as the positive control, but the hidden owner-side mechanism label is non-transferable. The unique-incidence matcher therefore produces a valid graph correspondence and is scored as a false unification.
- A second negative test submits a complete degree-signature mapping on K3,3 versus the triangular prism; the structural validator rejects it with an incidence counterexample, producing `RejectedFalseProposal`, not abstention.

This separates graph correspondence from mechanism transportability and prevents a rejected proposal from being scored as a genuine abstention. The isomorphic-surface negative is deliberately evaluator-labelled and local; it is not a production secret-store or isolated blind-evaluator service.

The Atlas source changes are after the earlier verified code head `bb11dc83015f63afbcddba6c00ee92fdce5fc23e`. The new executable Atlas code is at commit `6532c3da90658a09ff2ca67f377a1077d16bd6f4`; subsequent branch commits are documentation-only. Workflow [#45269716596](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45269716596), associated with later head `4fbf1254365037184e366a458536a9c1a1c06b69`, was cancelled by branch concurrency before qualification. It is not a pass. The extended Atlas code remains unqualified until Rust 1.96 formatting, tests/doctest and strict Clippy pass on the same code snapshot.

## Scoped transfer-map semantics for v0.2

The v0.2 protocol now requires transfer proposals to declare whether they are an isomorphism, embedding, homomorphism, quotient/coarse-graining, simulation, bisimulation, approximate simulation/bisimulation, or continuous-time conjugacy/semiconjugacy. Each type carries explicit source/target scope, unmapped entities, assumptions, transformations, uncertainty and counterexamples. The v0.1 executable validator remains a total finite structural isomorphism witness; it does not implement partial embeddings, quotients, simulations or approximate/continuous equivalence.

## Scope-aware structural transfer v0.2

The branch now adds `src/scoped.rs`, exposed as `scoped`, plus:

- `SCOPED_STRUCTURAL_TRANSFER_V0_2.md` — finite scope and map-kind contract.
- `schemas/scoped_transfer_proposal_v0.schema.json` — standalone sidecar schema pinned to source/target episode IDs and evidence-prefix digests.

The executable validator supports three map kinds:
- `Isomorphism`: complete structures; excluded entity sets must be empty.
- `InducedEmbedding`: selected source/target induced substructures map bijectively; every relation wholly inside the selected node sets must be represented.
- `SubgraphEmbedding`: selected source/target subgraphs map bijectively; extra target relations may be outside the mapping but must be explicitly listed as excluded.

For both source and target, selected and excluded node/relation IDs must be unique, disjoint, valid and form a complete partition of the supplied structure. Selected relations cannot reference nodes outside the selected node scope. The returned witness reports excluded context counts; nothing outside the map is silently treated as equivalent.

The current scope contract deliberately does not implement many-to-one homomorphism, quotient/coarse-graining, simulation, bisimulation, approximate maps or continuous-time conjugacy. Those remain distinct validators. The wire schema admits only the three currently implemented map kinds.

Seven tests cover an induced embedding, full-isomorphism scope enforcement, omitted internal target relation, explicit extra target relation for subgraph embedding, incomplete scope partition, relation escaping the scope, and incomplete correspondence coverage.

**Verification for the scope-aware extension:** pending exact-head Rust 1.96 formatting, tests/doctest and strict Clippy. Earlier successful workflow coverage does not extend to this new module.


## Scoped transfer proposal example and workflow note

Added `examples/scoped_transfer_induced_embedding.example.json` as a machine-readable example of the new v0 proposal sidecar. The JSON Schema and example have both been retrieved and parsed as JSON. The example shows two selected nodes / one selected relation on each side, plus one excluded node and one excluded relation on each side, all bound to separate episode IDs and evidence-prefix hashes.

The first exact-head workflow after adding `scoped.rs` reached only `cargo fmt --check` and failed on three formatting differences in that file. Those exact rustfmt differences were corrected. The example and docs were then added; the latest branch head must still receive an exact-head Rust 1.96 format/test/doctest/strict-Clippy result. No validation result from the pre-scope code is inherited.

## Transfer bundle sidecar

Added `AGENT_TRANSFER_BUNDLE_V0_1.md`, `schemas/agent_transfer_bundle_v0.schema.json` and `examples/agent_transfer_bundle_v0.example.json`. The bundle attaches to an existing `AgentSubmission` by the exact Submit-frame digest, and binds both source and target episode IDs and evidence-prefix SHA-256 digests.

The scoped structural schema's `$id` now matches its repository filename, and the bundle references it by a relative schema URI. Each structural node/relation snapshot has evidence IDs. The optional deterministic-dynamics proposal now also carries source/target state and action snapshots with evidence IDs, plus source-transition evidence IDs and predicted target successors. Hidden target transitions remain evaluator-owned.

This remains a sidecar specification/example, not an integrated #11171 runner or a claim that the JSON Schema has been executed by a full Draft 2020-12 validator. Static JSON parse and scope-partition checks are reported separately.

## Decision-relevant expected value of sample information

Added `src/decision_value.rs`, exposed as `decision_value`, with `DECISION_RELEVANT_EVSI_V0_1.md`, a JSON Schema and a machine-readable known-answer fixture.

The evaluator computes an exact finite Bayes-risk reduction:
- baseline minimum expected loss before an experiment;
- expected minimum loss after observing each possible experiment outcome;
- exact EVSI numerator over a common denominator;
- net EVSI after cost in the same declared loss units;
- floating-point mutual information as a separate diagnostic only.

The known-answer pair contains a higher-information but decision-irrelevant probe and a lower-information, decision-changing probe:
- Experiment 100: mutual information = 0.8 bits; EVSI = 0; cost = 1; net EVSI = -1.
- Experiment 101: mutual information ≈ 0.721928 bits; EVSI = 2; cost = 1; net EVSI = +1.

Five tests currently encode the core exact arithmetic, information/decision divergence, cost dominance, invalid likelihood normalization, incomplete loss table and impossible-outcome semantics. (The current test module has six tests total; future tests should add the duplicate-ID and unknown-reference paths.)

This is an exact one-step finite decision oracle, not an adaptive scheduler, a learned VoC predictor, a causal experiment selector, or an action authority. It is designed to serve as a known-answer baseline for the #11171 research-policy adapter and to complement Symthaea #5476/#5478/#8721 and #13418.

## Decision-value oracle test expansion

The decision-value module now has twelve tests, including adversarial checks for duplicate hypothesis/action/outcome IDs, duplicate likelihood pairs, unknown outcome references, likelihood weights above scale, non-normalized rows, incomplete loss tables, impossible outcomes and cost dominance. The finite EVSI arithmetic remains exact; only the mutual-information diagnostic uses floating point.

**Verification for the decision-value source and its latest tests:** pending exact-head Rust 1.96 formatting, tests/doctest and strict Clippy. This code is newer than the earlier scoped-validator checks.

## Exact finite decision-value oracle

The branch now adds `src/decision_value.rs`, exposed as `decision_value`, and:
- `DECISION_RELEVANT_EVSI_V0_1.md`
- `schemas/decision_value_problem_v0.schema.json`
- `examples/decision_relevant_evsi_v0.example.json`

The exact finite evaluator separates mutual information from expected value of sample information and net decision value after cost. Known-answer experiment 100 carries 0.8 bits but has EVSI 0 / net -1; experiment 101 carries about 0.721928 bits but has EVSI 2 / net +1 because it changes the optimal action. EVSI uses exact integer numerators; mutual information is a descriptive floating-point metric only.

The module is a one-step evaluator/baseline, not a scheduler or research-policy authority. It is intended to be composed with #5476/#5478/#8721 and the #11171 adapter as a reference check, not a duplicate value-of-computation framework.

**Verification:** this decision-value code and its adversarial tests are newer than the last passing #7173 run and remain pending exact-head Rust 1.96 format/tests/doctest/strict Clippy.


## Finite quotient / coarse-graining implementation

The branch now also adds `src/quotient.rs`, exposed as `quotient`, with `FINITE_QUOTIENT_V0_1.md`.

The validator accepts a total state-to-class partition of a deterministic finite transition system only when it is an action congruence:
- all members of a class agree on action enablement;
- enabled members transition into the same destination class;
- the source transition system is deterministic;
- every source state is accounted for exactly once.

It builds a quotient state/action transition table and rejects incomplete partitions, successor-class mismatch, action-enabledness mismatch, unknown states, nondeterminism and duplicate transition rows. It does not yet match the quotient to a separate target system or preserve task-specific observables/rewards.

This is a finite one-system abstraction witness, not simulation/bisimulation or physical coarse-graining. Exact-head Rust verification is pending.

## Finite simulation and bisimulation

The branch now adds `src/behaviour.rs`, exposed as `behaviour`, and `FINITE_SIMULATION_BISIMULATION_V0_1.md`.

The validator uses finite labelled transition systems with possibly nondeterministic outcomes and a many-to-many state relation.

- **Simulation:** for each related state pair, every scoped source transition must have a matching target transition under the mapped action and related successor pair. Additional target transitions are permitted.
- **Bisimulation:** adds the reverse check. Every scoped target transition must have a matching source response, and the action map must cover the full target action scope.
- Scopes must be valid and closed under their declared state/action transitions. State relation pairs are unique and cover every scoped state. Action mappings and action-kind mappings are separately checked.

Seven tests cover target-extra-behaviour acceptance for simulation, rejection for bisimulation, positive bisimulation when extras are removed, missing target response, many-to-many state relations, duplicate edges and missing relation coverage.

This is finite proposed-relation validation, not learned mapping discovery, continuous-time behavioural equivalence, probabilistic bisimulation or causal-mechanism qualification.

**Historical note:** this pending status was superseded by the exact-head qualification recorded below. The current executable source passed formatting, 86 unit tests plus doctests, and strict Clippy on Rust 1.96.0.

## Latest exact-head qualification — 24 November 2026

The current executable source is at exact commit `9735f9f7ddaabce19b75a96a0e97783d3cc04a9d`. Focused workflow [#45296666519](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45296666519) ran with Rust 1.96.0 and passed every step:

- `cargo fmt --check`: passed;
- `cargo test --locked`: passed — **86 unit tests, 0 failures**, plus doctests;
- `cargo clippy --locked --all-targets -- -D warnings`: passed.

This current head includes the structural/dynamics witness, Atlas, scoped isomorphism/induced/subgraph embedding, finite quotient congruence, many-to-many simulation/bisimulation, finite EVSI decision-value oracle, and the sidecar schema/example documents. The earlier failed Clippy/test attempts described above are historical and superseded by this successful exact-head run.

Qualification boundary: this is the focused ErgoStream microcosm crate, not repository-wide Linux/Windows/macOS CI. It does not qualify the #11171 production learner adapter, the sealed evaluator process boundary, continuous-time transfer, physical deployment, or external benchmark performance.

## Sidecar consistency audit

The scoped structural, agent transfer bundle and decision-value JSON documents and examples were retrieved and parsed as JSON. Static cross-file checks passed for:
- the bundle's relative scoped-schema reference;
- repeated source/target episode and evidence-prefix bindings;
- complete selected/excluded node and relation partitions for both structural snapshots;
- behavioural state-scope and action-map coverage against the evidence-backed snapshots;
- simulation map-kind and response prediction fields;
- absence of evaluator-owned actual hidden target transitions from the candidate sidecar;
- two decision-value known-answer experiments.

This was a static consistency audit, not a full Draft 2020-12 validator invocation or a live #11171 episode submission.

## Transfer calibration and selective-risk metrics

The branch now adds `src/calibration.rs`, exposed as `calibration`, with `TRANSFER_CALIBRATION_V0_1.md`.

The evaluator-side metric accepts candidate probabilities for exact scoped transfer claims and owner-side binary outcomes. It reports:
- exact fixed-point Brier score;
- mean binary log loss (with explicit one-ppm clipping count at probabilities 0/1);
- fixed-width reliability-bin sufficient statistics and exact ECE numerator/denominator;
- selective risk/coverage at evaluator-specified confidence thresholds.

The decision confidence is derived from the predicted class: `p_transfer` for an assertion and `1-p_transfer` for a rejection. Explicit abstention is excluded from selective coverage but its probability remains in Brier/log-loss/ECE scoring. Candidate-authored probabilities and decisions are optional `transfer_forecasts` in the AgentTransferBundle; evaluator labels, bins and thresholds are not included in the learner sidecar.

The pooled scorer tests cover exact scores, overconfident false transfer, abstention/coverage trade-offs, rejection confidence, boundary binning, malformed and duplicate IDs, out-of-range values and unmatched outcomes. Its initial pooled-only exact head passed 95 tests in workflow #45356575117; the later stratified addition has its own qualification recorded below.

## Transfer forecast calibration and selective risk — current addition

New implementation on the branch:
- `src/calibration.rs`, exported as `calibration`;
- `TRANSFER_CALIBRATION_V0_1.md`;
- optional top-level `transfer_forecasts` in the AgentTransferBundle schema and example.

It computes exact fixed-point Brier score and ECE numerators/denominators, mean log loss with a reported one-ppm boundary clip count, and selective risk/coverage across evaluator-supplied confidence thresholds. Assert/reject confidence is derived from the selected class; abstention is excluded from selective coverage but its probability remains in proper-score calculations. Thresholds, bins and outcome labels are evaluator-owned. The example forecast list includes structural, simulation and mechanism claims with assert/abstain/reject decisions.

The pooled calibration implementation and the later evaluator-stratified extension are now qualified together on exact head `e9a5fbf30b69931f54f387329df31af210ee717e`: Rust 1.96.0 formatting, 96 unit tests, doctests and strict Clippy all passed in workflow [#45357214975](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45357214975). The earlier 86-test qualification is historical and superseded for this branch.

## Evaluator-stratified transfer calibration

The calibration module now also includes `evaluate_transfer_calibration_by_stratum`. Each owner-side outcome has an `evaluation_stratum_id` (not visible in the learner's forecast schema). The helper first validates the complete pooled cohort, then emits separate Brier/log-loss/ECE and selective-risk reports per stratum in deterministic ID order.

This allows preregistered comparisons across representation-only, semantic/mechanism, combined shift and claim-family strata without allowing candidate-authored labels or silent omission of malformed records. A new known-answer test demonstrates a pooled evaluation that could hide a stratum with 100% selective risk.

**Current source head `e9a5fbf30b69931f54f387329df31af210ee717e` is qualified:** workflow [#45357214975](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45357214975), Rust 1.96.0, format passed, 96 unit tests passed, doctests passed, strict Clippy passed.

## Latest exact-head qualification including stratified calibration — 24 November 2026

The current executable source head is `e9a5fbf30b69931f54f387329df31af210ee717e`. Focused workflow [#45357214975](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45357214975) ran with Rust 1.96.0 and passed all three gates:
- `cargo fmt --check`: passed;
- `cargo test --locked`: **96 unit tests passed, 0 failed**, plus doctests;
- `cargo clippy --locked --all-targets -- -D warnings`: passed.

The 96 tests include the evaluator-stratified calibration regression, which demonstrates why pooled scores must not hide a failing shift stratum. This remains focused microcosm qualification only. It is not full repository-wide CI, a formal conformal guarantee, production #11171 integration, a sealed evaluator, or evidence of external SOTA performance.

## Wilson uncertainty intervals for selective risk and coverage

`SelectiveRiskPointV0` now exposes two-sided 95% Wilson intervals for both coverage and selective risk. Coverage uses all scored claims as its denominator; selective risk uses only selected claims at that confidence threshold. An empty selected set has no selective-risk estimate/interval. Three tests cover zero observed errors at small n, separate denominators and empty selective sets.

These are explicitly IID binomial reference intervals only. They are not valid under repeated measures, campaign/world clustering, overlapping frames or temporal dependence. The confirmatory benchmark must declare the independent unit and use block/campaign bootstrap or another justified dependence-aware procedure when observations are clustered.

**This source addition is newer than exact head e9a5… and requires its own Rust 1.96 formatting, tests/doctests and strict-Clippy qualification.**

## Split-conformal transfer prediction sets — new implementation

The calibration module now includes a binary split-conformal reference. It calibrates nonconformity score 1-p_transfer for true transfer examples and p_transfer for false transfer examples. The finite-sample quantile rank is ceil((n+1)(1-alpha)); when the rank is n+1, it uses the conservative full-scale quantile. At prediction time it returns a set over {transfer, non-transfer}; singleton sets map to assert/reject recommendations, while both-label or empty sets do not produce a decision.

The fitter requires matched forecast/outcome IDs in one evaluator-owned stratum. Its claim ceiling is finite-sample **marginal set coverage under exchangeability within that stratum**. It is not conditional coverage, arbitrary-shift robustness, a cross-stratum guarantee or an online sequential guarantee.

Five additional tests cover finite-sample rank and singleton output, conservative full-set fallback when calibration is small, mixed-stratum/invalid-alpha rejection, unmatched/duplicate calibration rows and prediction-probability range.

**Exact-head qualification passed:** code head `80f48aae87e57d73efa31d69296c665cb5796693` passed Rust 1.96.0 formatting, 105 unit tests plus doctests, and strict Clippy in workflow [#45359834929](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45359834929).

## Split-conformal preconditions and set-valued edge cases

The split-conformal contract now states that predictor, preprocessing, probability forecast and nonconformity mapping must be frozen independently of calibration outcomes; the calibration cohort cannot be used to fit/select that predictor. Exchangeability is required between independent calibration and future claims within one predeclared evaluator-owned stratum.

A new test covers inclusive quantile ties and the empty prediction set. Empty and both-label sets are distinct in the set-membership fields but both produce no forced class decision. This protects the core reject-option behaviour.

## Owner-issued split-conformal receipt

Added an owner-side receipt schema and illustrative example:
- `schemas/split_conformal_transfer_receipt_v0.schema.json`
- `examples/split_conformal_transfer_receipt_v0.example.json`

The receipt binds a fitted split-conformal quantile to exact predictor/preprocessing/score-function digests, calibration cohort and sample IDs, split manifest, evidence-prefix, evaluator stratum, independent unit and count, claim type/scope, alpha, order-statistic rank, quantile and owner custody receipt. It supports active/revoked/expired state, requiring a reason when revoked.

This prevents a quantile from being silently reused after the model, score function, cohort, stratum or scope changes. Digests remain identifiers, not proof of acquisition/custody or exchangeability. Both receipt schema and example were parsed as JSON; static checks confirmed all supplied SHA-256-shaped digests and the example order-statistic rank (ceil(41 × 0.9) = 37). This is not a full Draft 2020-12 validator run.

## Latest exact-head qualification — split conformal and owner receipt — 24 November 2026

Current executable code head: `80f48aae87e57d73efa31d69296c665cb5796693`. Focused workflow [#45359834929](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45359834929), Rust 1.96.0, passed all steps:
- `cargo fmt --check`: passed;
- `cargo test --locked`: **105 unit tests passed, 0 failed**, plus doctests;
- `cargo clippy --locked --all-targets -- -D warnings`: passed.

The new tests cover split-conformal rank/coverage, conservative full-set fallback at insufficient calibration size, mixed-stratum and invalid-alpha rejection, malformed calibration bindings, inclusive quantile ties and empty-set abstention. The owner-issued receipt schema/example parse and the example rank arithmetic matches. Qualification remains focused to the finite microcosm: no production #11171 adapter, sealed multi-process evaluator, temporal/online conformal guarantee, conditional coverage, physical deployment or SOTA result is claimed.

## Conformal receipt schema tightening

The owner-issued receipt schema now requires both the independent-unit count and exact claim-scope digest. Its semantic owner checks are documented: model freeze precedes calibration start, which precedes receipt issuance; quantile rank is at most n+1 and matches alpha/count; and sample-ID digest/count match the sealed calibration manifest. JSON parse/static example checks remain distinct from cryptographic custody validation.

## Tie-aware selective risk, decision coverage and separate forecast/outcome artifacts

The calibration evaluator now reports:
- exact fixed-point Brier and ECE numerators/denominators;
- log loss with explicit boundary clipping count;
- per-stratum reports using evaluator-owned strata;
- Wilson reference intervals for coverage and selective risk;
- a tie-aware right-step AURC over decided samples;
- decision coverage over the full forecast cohort.

AURC groups equal selected-class confidence values before integration, making the result input-order invariant. It is normalized over decided claims, while decision coverage uses all forecasts; both must be reported together so the learner cannot improve AURC by abstaining on nearly everything. Brier/log loss/ECE still score abstentions' probabilities.

Separate artifacts now exist:
- learner-side `schemas/transfer_forecast_cohort_v0.schema.json` and `examples/transfer_forecast_cohort_v0.example.json`;
- evaluator-only `schemas/transfer_evaluator_outcomes_v0.schema.json` and `examples/transfer_evaluator_outcomes_v0.example.json`;
- split-conformal output `schemas/split_conformal_transfer_v0.schema.json` and `examples/split_conformal_transfer_v0.example.json`.

Static cross-file audit passed:
- all three schema documents and three examples parse as JSON;
- candidate forecast and evaluator outcome cohort IDs match;
- 12 sample IDs are unique on both sides and form a one-to-one join;
- the learner forecast objects contain no evaluator stratum field;
- evaluator outcomes carry four illustrative strata (0–3);
- the conformal example's finite-sample rank equals `ceil((19+1)*(1-0.10)) = 18`.

This is JSON parse plus static consistency, not a full Draft 2020-12 schema-validation engine or owner-side cryptographic/custody verification.

## Latest exact-head qualification — calibration, bundle and AURC — 24 November 2026

The exact executable code head is `ca85791f015d9446cd6559fb8d2a7718e2c88399`. Focused workflow [#45394880303](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45394880303) ran under Rust 1.96.0 and passed all steps:
- `cargo fmt --check`: passed;
- `cargo test --locked`: **118 unit tests passed, 0 failed**, plus doctests;
- `cargo clippy --locked --all-targets -- -D warnings`: passed.

The subsequent current PR/branch commits are documentation/schema/example additions after that code head; no executable Rust source changed after the verified code head. The pass is focused microcosm qualification, not repository-wide cross-platform CI, production #11171 integration, a sealed multi-process evaluator, block/online conformal, physical deployment, or external SOTA performance.

## Bound-claim identity and evidence-bound mechanism transfer — implementation added, verification pending

New executable additions after the previous green 118-test head:
- `TransferClaimTypeV0`, `BoundTransferForecastV0`, `BoundTransferOutcomeV0`, and `evaluate_bound_transfer_calibration`: bind each probability to exact claim ID, type and owner-recomputed scope digest before scoring.
- `EvidenceBoundMechanismCandidateV0`, `OwnerInterventionOutcomeV0`, and `validate_evidence_bound_mechanism_transfer`: verify source intervention evidence against the public source prefix, require a complete action map/source observation/target prediction/owner outcome set, compare predictions to evaluator-held target effects and delegate to the existing preserve/negate effect-transform witness.
- The bundle schema now requires a mechanism candidate artifact whenever a `mechanism_transfer` forecast exists.
- The versioned claim-scope envelope and registry schema/example define RFC 8785 JCS plus SHA-256. Three example vectors cover structural, simulation and mechanism claim payloads.

**Verification:** the exact code head with these new validators has not yet passed formatting/tests/doctests/strict Clippy. An earlier run on the mechanism code passed formatting but failed test compilation because a test-only import referenced `SourceInterventionEvidenceV0` from the wrong module; that import was corrected in the latest code. Do not inherit the previous 118-test pass for these new source changes.

## Calibration implementation audit and stratum fail-closed correction — 24 November 2026

A source audit identified and corrected two issues:
1. calibration-bin records were all initialized with `bin_index = 0`; initialization now assigns each bin its actual index, and the boundary test asserts the complete index sequence;
2. split-conformal prediction did not require the evaluator-assigned target stratum. The prediction API now takes the owner-assigned stratum and, when it differs from the calibration stratum, returns the full binary prediction set, no decision, and `exchangeability_admitted = false`.

This protects against accidentally carrying a split-conformal statement across a declared stratum boundary. It does **not** detect distribution shift within a stratum or create a guarantee under arbitrary mechanism/concept shift. Such claims require separate shift diagnostics and assumptions.

The current branch exact head `3da8b62e8a29720a7cba2d2d4fd453f23f467f51` passed focused Rust 1.96 workflow [#45400521911](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45400521911): formatting, **129 unit tests plus doctests**, strict Clippy. The stratum fail-closed change is newer and now requires a fresh exact-head run.

## Independent-unit enforcement — added after the 129-test head

Split-conformal fitting now consumes `SplitConformalCalibrationOutcomeV0`, whose rows carry an evaluator-owned independent-unit ID. It rejects repeated unit IDs so repeated frames/windows from one campaign cannot inflate the conformal rank. v0.1 therefore requires one scored claim per independent unit; multiple claims inside one block need a separately specified block-maximum protocol.

Added the evaluator-only JSON Schema/example at `schemas/split_conformal_calibration_outcomes_v0.schema.json` and `examples/split_conformal_calibration_outcomes_v0.example.json`. Domain fingerprint resolution and minimum independent-block thresholds are delegated to #7041; the conformal fitter consumes only the owner-approved stratum. This source change requires fresh exact-head Rust verification.

## Independent-unit conformal calibration input

The split-conformal fitter now accepts `SplitConformalCalibrationOutcomeV0`, an evaluator-only record containing:
- sample ID;
- evaluator-issued independent-unit ID;
- evaluator-assigned stratum ID;
- exact claim-qualification outcome.

Duplicate independent-unit IDs are rejected. v0.1 uses one scored claim per independent unit. Multiple correlated claims within one block require a separately specified aggregate/block-conformal method; they must not be treated as extra independent calibration rows.

Added `schemas/split_conformal_calibration_outcomes_v0.schema.json` and `examples/split_conformal_calibration_outcomes_v0.example.json`. Domain fingerprint admission and minimum independent-block counts compose with Symtropy #7041. The new source and schema work is newer than the last passing 129-test exact head and remains pending a new Rust 1.96 format/test/doctest/strict-Clippy run.

## Calibration transport protocol

Added `CONFORMAL_CALIBRATION_TRANSFER_V0_2_PROTOCOL.md` defining a future structural-map-gated calibration transport tournament against DS-CP, Transported Conformal Calibration and KMM-CP. The protocol explicitly distinguishes exchangeability, covariate shift with support, paired calibration transport and concept/mechanism shift. It defines a paired-world factorial matrix, owner-bound transport receipt, baselines, metrics, acceptance gates and integration with #7041/#7193/#5990/#11171.

This remains a research protocol, not an implemented weighted/TCC/KMM method or a claim of target-domain coverage outside the admitted split-conformal stratum.

## Full conformal calibration identity binding — implementation added, verification pending

The split-conformal Rust API now stores `SplitConformalCalibrationIdentityV0` in the calibrator. It binds:
- owner-issued receipt ID;
- evaluator-assigned stratum and independent-unit kind;
- claim type and scope digest;
- predictor, preprocessing and score-function digests;
- calibration cohort, sample-ID set, split manifest and evidence-prefix digests.

Fit rejects malformed identity digests and empty receipt IDs. Prediction takes the owner-resolved full identity; any mismatch returns the full two-label set, no decision and `calibration_scope_admitted=false`. Tests cover changes to claim type/scope, predictor, preprocessing, score function, independent-unit kind and receipt/cohort/manifest/evidence identity.

The schema/example now mirror the complete identity. The Rust code validates digest shape and field equality only; active receipt signature, custody, revocation, expiry and chronology remain owner-side checks. This is a new source head after the 129-test pass and requires exact-head Rust 1.96 formatting, tests/doctests and strict Clippy.

## Full owner-issued calibration receipt identity binding

The split-conformal calibrator now stores `SplitConformalCalibrationIdentityV0`: receipt ID, stratum, independent-unit kind, claim type/scope, predictor/preprocessing/score digests, calibration cohort digest, sample-ID digest, split-manifest digest and evidence-prefix digest.

- Fit validates all required digest formats and rejects empty receipt IDs.
- Prediction requires the complete evaluator-resolved identity; any mismatch returns the full two-label set, no decision and `calibration_scope_admitted=false`.
- Tests mutate receipt ID, claim type/scope, predictor, preprocessing, score function, independent-unit kind, calibration cohort, sample-ID set, split manifest and evidence prefix; all changed identities fail closed.
- The schema and example mirror this receipt identity.

**Boundary:** the Rust module checks syntax and identity equality only. The episode owner still must verify signature/custody, active/revoked/expired status, chronology, and the actual calibration manifest before constructing this identity. The current code head requires exact-head Rust 1.96 format/test/doctest/strict-Clippy qualification.

## Receipt-status gating

`SplitConformalCalibrationIdentityV0` now includes `receipt_status` (`Active`, `Revoked`, `Expired`). Fitting rejects non-active receipts; prediction identity comparison fails closed to the full set/no decision if a receipt is revoked/expired or otherwise differs. The JSON schema/example mirror the status field.

This is a typed owner-to-evaluator handoff, not cryptographic verification. The owner still verifies signature/custody, expiry/revocation, chronology, cohort digest and receipt state before creating the identity. Exact-head Rust verification remains pending.

## Sequential/dependence ownership composition

The static split-conformal implementation does not cover adaptive probe selection, optional stopping, delayed label maturation, or temporally dependent episodes. Related owners have been identified and linked:
- #6943: dependence-aware trajectory calibration and independence-group contract;
- #5285: block-maximum calibration and transfer failure;
- #6887: anytime-valid sequential decision sets under adaptive observation;
- Symthaea #13280: online conformal calibration under delayed metrology feedback and process drift.

The v0.1 calibrator remains offline/static; no online update or anytime-valid claim is made. These systems should be integrated by receipt/episode identity and owner policy, not reimplemented here.

## Complete calibration-result receipt verification

The conformal identity now also carries the receipt's `alpha_ppm`, `calibration_count`, `quantile_rank` and `quantile_nonconformity_ppm`. The fitter recomputes the finite-sample count/rank/quantile from the supplied calibration outcomes and refuses the calibration with `CalibrationReceiptMismatch` unless all values and alpha match the owner receipt. Prediction equality includes those fields; a different nominal miscoverage level or quantile cannot reuse the calibrator.

The JSON schema/example place these finite-sample values inside the bound calibration identity, avoiding a separate unbound copy on the calibrator object. Exact-head Rust verification is pending.
