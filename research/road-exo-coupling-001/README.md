# Road-autonomy exogenous coupling fixtures v0.1

**Status:** standalone semantic reference and conformance fixtures; not a production RNG and not integrated into physics execution.  
**Parent:** Symtropy #3543.  
**Consumers:** #3532 reciprocal loop, #3502 actor adapter, #3539 pose/replay substrate, Symthaea #10356/#10372.

## Research position

The general event-keyed RNG design is established prior art. Buffalo, Pearson & Klein (2026), [Realizing Common Random Numbers: Event-Keyed Hashing for Causally Valid Stochastic Models](https://arxiv.org/abs/2603.11084), define execution invariance and show how counter-based generators plus stable event identifiers avoid execution-path-dependent draw shifts. Salmon et al. (SC'11) established counter-based RNGs for parallel computation ([DOI](https://doi.org/10.1145/2063384.2063405)).

Our bounded contribution is a road-autonomy conformance layer for the meaning of event identity across interactive scene branches. The paper explicitly treats event matching as a scientific modeling choice. This crate makes three candidate identity bases explicit:

- Actor event: one scenario-authored actor, tick and event kind.
- Directed actor pair: initiator, recipient, tick and event kind; roles are not interchangeable.
- Scene slot: one encounter/opportunity slot, tick and event kind; using it across different actors asserts a slot-level exchangeability assumption.

These are distinct coupling hypotheses, not interchangeable encodings. Keep alternative hypotheses as sensitivity branches when evidence cannot select one.

## Contract

DrawAddress.canonical_bytes creates a versioned, length-prefixed representation. It is intentionally not a hash, random generator or security primitive. A later pinned counter-based generator can map canonical addresses to its counter/key space.

Shared addresses omit branch identity but require an explicit equivalence group and justification reference. Branch-local and independent-replicate addresses include their branch identity. Random values are not generated here.

An actor decision may receive the same exogenous policy-noise input in both branches while its delivered observation differs. The policy output must still be recomputed from that branch's observation. Do not automatically relabel every actor-policy draw after an ego intervention as branch-specific.

DrawPresence.NotRealized records an event address that exists in the potential event space but was not realized in that branch. It requires a causal-condition witness. A missing record is different: the pair audit marks it incomplete rather than assuming conditional absence.

The API deliberately does not accept action, cue contents, velocity, pose, planner output or collision state as identity fields. Those are endogenous state or mechanism inputs and must not silently contaminate an invariant exogenous key.

Stable actor IDs are scenario-authored. Runtime BodyHandle, allocation order, mutable vector index, spawn order and current pose are not admissible identity sources. Dynamic births require separately justified lineage semantics and are out of this v0.1 crate.

## Executable checks

The standalone Rust crate has no third-party dependencies and is isolated from the main Cargo workspace. Its tests cover invariant shared addresses, branch separation, call-order independence, distinct identity bases, directed-pair role preservation, duplicate consumption, shared-value mismatch, conditional presence, missing counterparts, branch-scope enforcement and malformed IDs.

## Claim ceiling

Passing establishes only that this small reference implementation serializes the declared identity contract and checks represented pair records. It does not validate the causal graph, select an event identity map, prove random-number quality, identify real-world counterfactuals, validate actor behavior or physics, establish a safety case, or authorize vehicle actuation.


## Deterministic Philox resolver (ROAD-PHILOX-ADAPTER-001)

The crate now includes an adapter over two pinned dependencies:

- `rand_philox = 0.2.1`, default features disabled: Philox4x32-10 block generation.
- `blake3 = 1.8.7`, default features disabled: BLAKE3 derive-key mode for deterministic semantic-address mapping.

The adapter is not a new PRNG. It uses the documented `Philox::new([u32; 2], u128)` API and reads exactly four words from exactly one block. It does not use a sequential stream seed or random-draw cursor.

### Frozen v0.1 address derivation

1. Accept a 32-byte experiment root seed and an already-qualified DrawAddress.
2. Obtain DrawAddress canonical bytes. Shared versus branch-local semantics are entirely decided upstream by the DrawAddress/coupling record.
3. Length-prefix the seed and canonical address bytes as little-endian u64 lengths.
4. Apply BLAKE3 derive-key mode with the fixed context string in the source.
5. Map digest bytes [0..8] to the two little-endian u32 Philox key words, [8..24] to a little-endian u128 counter, and retain [24..32] as an eight-byte collision audit tag.
6. Generate one Philox block and retain its four u32 words in canonical output order.

The Philox input is 192 bits (64-bit key + 128-bit counter), derived from a 256-bit digest. It is not collision-free. Each receipt retains the full derivation digest; a run-local registry rejects one key/counter pair claimed by different full digests. Cross-run receipt comparison is still required. This is not authentication or a cryptographic security boundary.

### Receipt and limits

A DrawReceipt binds the address schema ID, KDF ID/context, PRNG crate/algorithm version, full derivation digest, collision tag, key, counter and all four output words. verify_receipt recomputes the full mapping from the frozen experiment seed and canonical address.

The algorithm is a reproducible simulation RNG, not a CSPRNG. The 32-byte experiment root seed is reproducibility material, not a secret. Distinct semantic addresses do not automatically prove independent random variables or a valid causal coupling. Event identity and causal admissibility remain upstream responsibilities.

The adapter uses raw u32 words only. It deliberately adds no float conversion, bounded sampling, modulo reduction or rejection loop. Domain-specific distribution transforms and their consumption semantics must be specified separately.

### Verification

Tests include the three Random123 Philox4x32-10 known-answer vectors, same-address determinism, shared-vs-branch-specific behavior, call-order invariance, seed/source/component separation, receipt recomputation and run-local collision handling. Exact-head GitHub CI is required before this resolver is considered qualified.


## Reproducible dependency and fixture contract

The isolated package now has a committed `Cargo.lock`. Direct dependencies are exact-pinned and the focused workflow uses `cargo metadata/test/clippy --locked`; a resolver may not silently change the dependency graph.

The Philox test suite pins the three upstream Random123 Philox4x32-10 KAT rows (zero, all ones, and mixed pi-digit input) from `tests/kat_vectors`. Separate property tests check that changing key or counter changes the block; those are not mislabeled as upstream KAT rows.

The BLAKE3 adapter tests the official derive-key KAT using the official context string and zero-length key material from BLAKE3's `test_vectors/test_vectors.json`. The address serializer separately length-prefixes the 32-byte experiment seed and canonical event address.

Machine-readable `event_identity_cases.json` now records the dependency versions, KAT inputs/outputs, KDF context, address mapping, length-prefix convention and the 192-bit collision model. The workflow checks both event-identity and correspondence fixture JSON.

The locked dependency graph is a reproducibility contract. Any dependency update must update the lockfile intentionally and rerun KATs, formatting, tests, Clippy and cross-target checks.


### Additional adapter invariants

The resolver's unit tests also pin:
- the official BLAKE3 derive-key zero-length-input vector;
- exact little-endian u64 length-prefix framing for the experiment seed and canonical address as separate fields;
- fixed BLAKE3 KDF context domain separation;
- raw Philox output sensitivity to key and counter changes;
- unchanged output after unrelated address draws;
- seed/source/component separation, receipt recomputation and run-local block-address collision rejection.

The standalone manifest uses `blake3 = =1.8.7` and `rand_philox = =0.2.1`; `Cargo.lock` freezes the resolved transitive graph, and the workflow requires `--locked`.


## Exact finite dependence oracle (ROAD-DEPENDENCE-001 #3599)

The crate now includes a deliberately small, analytic finite-state joint-distribution reference. It is **not** a Gaussian random-field implementation or a fitted traffic stochastic model.

### Why it exists

A Philox block is an execution-order-independent random primitive; it does not say whether two downstream variables should be independent, correlated, or share a cause. The same scalar marginals can produce very different joint traffic outcomes when their dependence structure changes.

The fixture makes the failure concrete:
- a scene/group-level latent factor G is shared by two actor responses;
- actor-local residuals epsilon_a and epsilon_b are distinct;
- each response is G + epsilon_i;
- G, epsilon_a and epsilon_b are exactly enumerated as independent fair {-1,+1} innovations in the declared finite table.

The exact output has mean zero, variance two per actor, and covariance one between actors (correlation 0.5). A deliberately incorrect per-actor group-factor model has exactly the same marginal output distributions but covariance zero. A marginal-only benchmark would miss this error.

### Implemented V0.1 scope

- Explicit finite joint outcome table with positive integer masses; the complete table defines the joint distribution directly, so no independence is inferred merely from different IDs.
- Typed source/factor identity, source-profile digest, units, physical scope, equivalence group and invariance justification.
- Deterministic linear structural descendants with explicit exogenous and intervention parents.
- Separate branch intervention inputs: the same latent factor table is reused while intervention descendants are recomputed.
- Exact integer-weight means, marginal counts and covariance; no floating-point tolerance is needed for the finite-state reference.
- Canonical manifest digest with order-independent variable/state/output declaration order.
- Per-state dependence receipts binding manifest digest, branch, latent values, intervention values, output values and an unkeyed digest.
- Receipt integrity check plus stronger replay verification against the exact manifest and intervention.
- Negative controls for independent-per-event factors, missing/duplicate states, zero mass, unknown inputs, invalid dyads and receipt tampering.

The table is a joint distribution of primitive innovations. Shared causes are represented by one factor variable reused by multiple structural outputs; actor-local residual variables remain distinct. The finite table explicitly defines the joint law and therefore does not smuggle in an independence assumption.

### Deliberately deferred

This first implementation does not yet generate random samples from Philox, model Gaussian/correlated random fields, infer covariance or copulas from data, represent nonstationary spatial bases, or implement temporal recurrences such as AR(1). Sampling is owned by #3582/#3586. The next dependence stages should add (1) time-indexed innovations and temporal recurrences, (2) finite-rank spatial basis fields, (3) cross-variable dependence and tail-dependence controls, and (4) calibrated empirical field profiles. Each must retain exact source/factor provenance and cross-world invariance classifications.

The graph is currently bipartite: primitive finite exogenous variables feed one layer of deterministic linear outputs and explicit intervention inputs. It is not yet a general recursive SCM/DAG engine.

### Measurement targets

- Exact joint-probability and covariance agreement with the finite table.
- Detect wrong joint covariance even when scalar marginals match exactly.
- 100% invariance of shared latent values across declared branch interventions.
- 100% recomputation of intervention-dependent linear descendants.
- Zero duplicate/omitted variable values or zero-weight states admitted.
- Zero receipt replay accepted against a different manifest or intervention.
- Exact declaration-order-independent manifest digest and receipts.


## Exact counterfactual decision closure oracle (ROAD-DECISION-CLOSURE-001 #3637)

The decision_closure module turns a finite set of action utilities across admitted reference/model hypotheses into an exact minimax-regret certificate.

### Rules

- The complete hypothesis set is explicit; no hypothesis probability weights are inferred.
- Every action must have an outcome status under every hypothesis.
- Supported utility values are exact rationals.
- Unsupported, NotIdentified or Incomplete action/hypothesis pairs stop a global decision certificate.
- Per-hypothesis best action sets and action regrets are retained.
- Worst-case regret is computed for every action.
- A stable decision requires the same unique best action in every hypothesis.
- If rankings reverse but a unique minimax-regret action exists, the result is DecisionSensitive and records the minimax-regret action without calling it uniformly optimal.
- If multiple actions tie for minimum worst-case regret, the result is NoRobustDecision.
- Missing support cannot be silently dropped; missing probability weights cannot be invented.

### Exact examples

1. Stable: H1 utilities A=10,B=2; H2 A=8,B=4. A uniquely wins both; status DecisionStable.
2. Sensitive: H1 A=10,B=8; H2 A=0,B=6. Rankings reverse; worst-case regret is A=6,B=2; minimax-regret action B, status DecisionSensitive.
3. No robust unique action: H1 A=10,B=0; H2 A=0,B=10. Both actions have worst-case regret 10; status NoRobustDecision.

### Receipt and limits

The manifest binds action IDs, each hypothesis ID and reference-manifest digest, support status and exact utility. The certificate records the manifest digest, hypothesis-level regret tables, action worst-case regrets, support gaps and decision status. Receipt verification recomputes the complete certificate.

V0.1 is a finite exact decision oracle. It is not an empirically calibrated decision model, causal identification engine or safety policy. Utility scales and reference hypotheses must be supplied and justified upstream; this module only verifies the regret calculation and the completeness of the declared finite hypothesis set.


## Exact counterfactual decision closure oracle (ROAD-DECISION-CLOSURE-001 #3637)

The decision_closure module evaluates a finite set of actions across an explicitly admitted set of reference/model hypotheses using exact minimax regret.

### V0.1 semantics

For each supported hypothesis h:
- best utility: U*(h) = max_a U(a,h)
- regret: R(a,h) = U*(h) - U(a,h)
- worst-case regret: W(a) = max_h R(a,h)
- minimax regret: min_a W(a)

No probabilities are inferred or assigned to epistemic hypotheses. Every declared hypothesis contributes to the worst-case calculation. A probabilistic expected utility over hypotheses would require a separate evidence-backed hypothesis probability model and is not implemented here.

Every action/hypothesis pair must carry a support status. Any Unsupported pair returns Unsupported; NotIdentified or Incomplete returns Incomplete; no global action certificate is emitted. Missing outcome rows, duplicate/unknown actions, duplicate hypotheses and missing reference-manifest digests reject before evaluation.

### Decision statuses

- DecisionStable: the same unique action is best under every admitted hypothesis and is also the unique minimax-regret action.
- DecisionSensitive: rankings differ across hypotheses, but a unique minimax-regret action exists. It is a robust compromise, not uniformly optimal.
- NoRobustDecision: multiple actions tie for minimum worst-case regret.
- Unsupported / Incomplete: one or more required action/hypothesis pairs are not qualified.

### Exact examples

1. Stable ranking: H1 A=10,B=2; H2 A=8,B=4. A uniquely wins both.
2. Ranking reversal: H1 A=10,B=8; H2 A=0,B=6. Worst-case regret A=6,B=2; B is minimax-regret but status is DecisionSensitive.
3. Symmetric reversal: H1 A=10,B=0; H2 A=0,B=10. Both actions have worst-case regret 10; status is NoRobustDecision.

### Receipt and limits

The decision manifest binds action IDs, hypothesis IDs, per-hypothesis reference-manifest digests, support statuses and exact rational utility. The certificate records manifest digest, per-hypothesis best-action/regret tables, per-action worst-case regret, support gaps and decision status. It is replay-verified against the exact manifest.

V0.1 is a finite exact regret oracle. It does not estimate real-world utilities, identify causal effects, establish hypothesis completeness, calibrate a probability distribution over model families, or certify a driving policy. It only verifies that the finite declared decision problem is handled without silently inventing support or weights.


### Utility comparability contract

Minimax regret across hypotheses is valid only when all utilities use the same declared cardinal scale and semantic direction. The decision manifest therefore binds both a utility-scale ID and a utility-semantics ID (including the "higher is better" convention). Changing either changes the manifest digest. This is a declared comparability contract; future evidence-bound decision integration must verify that each source utility was transformed onto that scale under a reviewed rule. The oracle does not infer scale equivalence from numeric values.


## Evidence-bound decision closure (ROAD-DECISION-EVIDENCE-BINDING-001 #3638)

The decision_evidence module closes the gap between an exact regret computation and the evidence supporting each claimed utility value.

### One-to-one support binding

Every Supported action/hypothesis outcome must have exactly one DecisionEvidenceBinding containing:
- the hypothesis ID and action ID;
- evidence purpose (EmpiricalDecision, HumanAdjudication or FormalConformance);
- exact EvidencePacket and SubjectLineage;
- exact QualificationTarget and QualificationReceipt.

Validation recomputes the evidence qualification from the packet, subject lineage and target. The outcome's evidence digest must equal the resulting qualification receipt digest. Missing, duplicate, orphan, mismatched, tampered or metadata-mutated bindings reject. Unsupported/NotIdentified/Incomplete outcome cells do not receive fabricated evidence bindings.

### Purpose-specific admissibility

- EmpiricalDecision accepts only IndependentObservedAnchor, ControlledInterventionOutcome or QualifiedNaturalExperiment. The target must bind the same action ID as the decision action.
- HumanAdjudication accepts only HumanAdjudicationReference with the declared human-label target.
- FormalConformance accepts only AnalyticReference with the declared formal domain target.

SimulatorGenerated and ModelGenerated outputs are never accepted as independent empirical outcome evidence. The inherited #3635 use-role and lineage gates remain mandatory.

### Evidence-bound certificate

The certificate contains the exact regret certificate, a canonical evidence-bundle digest and sorted per-cell qualification receipt references. Replay recomputes the complete decision and every bound evidence qualification. Utility changes alter the decision certificate without altering the evidence receipt digest; evidence metadata changes invalidate the evidence bundle.

### V0.1 limits

This does not make utilities unbiased or prove causal assumptions. It ensures that every supported value used by the finite minimax-regret calculation is bound to an evidence packet whose declared origin, target, lineage and use-role pass the inherited evidence verifier. Natural-experiment assumptions and utility construction still require independent review.


### Positive and negative purpose tests

The Rust fixtures now cover valid human-adjudication and formal-conformance references in their own purposes, in addition to rejection of those sources for empirical claims. An Unsupported action cell requires no evidence packet and remains Unsupported; it cannot be silently omitted or turned into an empirical result.


## Robust decision-directed measurement selection (ROAD-DECISION-MEASUREMENT-001 #3639)

The measurement_selection module chooses candidate measurements/interventions by worst-case decision-regret reduction over a finite admitted model-hypothesis set. It does not invent probability weights over hypotheses or possible test outcomes.

### Decision criterion

For a current complete decision matrix:
- baseline minimax regret is the minimum, over actions, of the worst regret across all admitted hypotheses;
- each possible measurement outcome names the set of hypotheses that remain compatible with that observation;
- compatibility sets may overlap to represent noisy / ambiguous measurements;
- conditional minimax regret is computed for each outcome's compatible subset;
- worst-case residual regret is the maximum conditional regret over every possible outcome;
- regret reduction is baseline regret minus this worst-case residual.

A measurement that reduces prediction error but leaves the worst-case action regret unchanged receives zero decision-regret reduction. If the test is noisy, the worst ambiguous outcome governs the value; no posterior mean is fabricated.

### Independent constraints and Pareto frontier

The candidate binds:
- modality (passive observation, controlled intervention, closed-course intervention, simulation challenge);
- protocol, preregistration and risk-assessment digests;
- exact acquisition cost and its scale ID;
- nonnegative conservative risk upper bound and its scale ID;
- declared authorization reference and safety-case digest;
- complete named outcome-to-hypothesis compatibility relation.

Cost and risk are hard constraints and are not subtracted from decision utility. Feasible informative tests are compared along acquisition cost, risk upper bound and worst-case residual regret. Non-dominated candidates form the Pareto frontier; the oracle does not choose an arbitrary scalar winner across the frontier.

### Exact example

Baseline:
- H1: A=10, B=0
- H2: A=0, B=10

Current minimax regret is 10. A perfect test has outcome O1 compatible only with H1 and O2 compatible only with H2. After either outcome the residual regret is 0, so the worst-case decision-regret reduction is 10. A test with one ambiguous outcome compatible with both H1 and H2 gives zero reduction. A noisy test with one ambiguous outcome still has residual regret 10 even if another outcome is fully diagnostic.

### Safety and evidence boundaries

A nonzero authorization reference and safety-case digest are **declared metadata**, not cryptographic proof of approval. This reference oracle does not verify signing authorities, risk models or operational permits. Do not execute physical interventions on the strength of this receipt alone; independent authority verification and a safety case are still required.

A SimulationChallenge remains simulation/model evidence. Even if it discriminates hypotheses, it cannot be upgraded to an independent observed real-world result. Actual measurements must pass the evidence provenance and decision-evidence-binding gates (#3635/#3638).

### V0.1 claim ceiling

Passing the finite fixtures proves exact worst-case residual-regret computation, outcome coverage, cost/risk hard-constraint enforcement, deterministic Pareto filtering and certificate replay for declared hypotheses and outcomes. It does not prove the possible-outcome mapping is physically exhaustive, experiment identification assumptions are true, test risk is correctly estimated, or that a selected road intervention is safe.

 
## SAM path-support to evidence-bound decision adapter (ROAD-SAM-EVIDENCE-BINDING-001)

The `trajectory_evidence` module composes the SAM #10514 sequential joint-trajectory support result with the existing #3637 minimax-regret and #3638 evidence-binding certificate. It does not duplicate SAM's support algorithm and does not convert support into utility.

### Contract

The bridge accepts one typed assessment envelope for every action × admitted-hypothesis cell. Each envelope retains:
- the adapter schema ID and SAM build commitment;
- the original SAM evidence-manifest identifier preserved verbatim;
- BLAKE3 commitments to canonical support-manifest and trajectory-query bytes;
- a BLAKE3 commitment to canonical SAM source-generated assessment-receipt bytes;
- target policy and context digests;
- whether the finite path universe is exhaustive;
- query completeness, decision-point count and finite-path membership;
- the exact SAM disposition; and
- an unkeyed content checksum over those fields.

The schema ID belongs to this **Symtropy adapter envelope**, not to a SAM-emitted wire format. SAM exposes canonical bytes for `SupportManifest`, `TrajectoryQuery` and `Assessment`, and `assess_with_receipt` returns an `AssessmentReceipt` that length-prefixes and binds all three exact artifacts. The bridge's `seal_from_sam_artifacts` and `verify_against_sam_artifacts` recompute that source-receipt byte string from the retained support-manifest bytes, query bytes and assessment fields, then compare it byte-for-byte before accepting the envelope. A shared four-vector fixture pins the manifest, query, result and combined receipt formats across repositories. These are content commitments, not signatures, authentication, proof of origin, or independent verification of the SAM binary. A consumer requiring authenticity must anchor the SAM build and artifact digests through an independently controlled channel.

### Five disposition rules

| SAM path disposition | Required decision-cell support state | Consequence |
|---|---|---|
| `TrajectorySupported` | May be Supported, Unsupported, NotIdentified or Incomplete for other reasons | Necessary path condition only; utility still needs #3638 evidence |
| `UnsupportedSequence` | Unsupported | Exact absence only under an exhaustive finite path universe |
| `SupportUnknown` | NotIdentified | Non-exhaustive absence is not structural zero |
| `Incomplete` | Incomplete | Missing/censored/incomplete query cannot authorize a decision |
| `OutOfScope` | Incomplete | Policy/context mismatch cannot be treated as in-scope support |

The adapter requires exactly one envelope per action × hypothesis cell. Missing, duplicate, orphan or mismatched support dispositions reject. It then delegates utility qualification to #3638 unchanged. Therefore:
- a supported path without a qualified utility evidence binding does not produce a decision certificate;
- a supported path can still have unidentified utility;
- a simulator-generated crossing cannot enter `EmpiricalDecision` merely because SAM supports its synthetic path;
- Unsupported, NotIdentified and Incomplete remain distinct through the certificate;
- no support status is inferred from marginal action coverage.

### Tests and claim ceiling

Hostile fixtures cover all five dispositions, unsupported-path/Supported-utility mismatch, supported-path/unidentified-utility, missing utility evidence, one-to-one action × hypothesis coverage, duplicate/missing/orphan envelopes, support/query/source assessment-receipt mutation, cross-repository golden bytes and order-independent bundle digest.

This is a **composition adapter**, not a new support estimator, propensity estimator, causal-identification method, or safety judgment. It does not independently invoke SAM or authenticate the supplied build/manifest/query commitments. Passing tests establishes only that the adapter preserves and composes the declared finite dispositions and existing evidence/decision gates.
