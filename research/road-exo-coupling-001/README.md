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


## Evidence provenance and ground-truth eligibility (ROAD-REALITY-GAP-001 #3635)

The evidence_provenance module separates observed facts, controlled interventions, natural experiments, simulator/model outputs, formal references and human adjudications.

### Typed evidence / claim pairs

Every evidence packet binds:
- origin class and claim class;
- use role (fit, calibration, development validation, sealed test, safety challenge, illustrative only);
- dataset/version, source system, actual action and action-realization status;
- assignment protocol and measurement calibration;
- identification package or human annotation protocol where applicable;
- complete generator/source/parent lineage;
- evidence content digest and separately computed metadata digest.

Qualification receipts bind both the evidence content digest and a canonical digest of the complete metadata/lineage. A replay verifier recomputes the full eligibility decision from the packet and subject lineage.

### Counterfactual boundary

A factual observation only qualifies for the action that was actually observed. It cannot be used as outcome truth for an unexecuted alternate action. A controlled intervention can support an outcome under the assigned and executed action if the action, assignment protocol and measurement calibration match. A natural experiment only qualifies with an explicit estimand, assumptions digest, overlap evidence, sensitivity plan and preregistration reference.

### Anti-self-validation and lineage

A reference is model-dependent when it shares generator ancestry or upstream source/parent content with the evaluated system, even if its filename, vendor wrapper or random seed differs. Unknown lineage is Incomplete, not independent by default. Simulator-generated outcomes remain SimulationOnly as real-world anchors; model-generated outcomes remain ModelDependentReference.

### Split leakage

The SplitManifest groups by scenario family, drive, capture session and upstream parent digests. A group crossing fit/calibration/development/sealed/safety splits is rejected. Its canonical digest is declaration-order independent and can be pinned for sealed evaluation; mutation invalidates the frozen digest.

### V0.1 limits

This is an evidence-provenance and eligibility contract, not a causal-inference engine or an empirical validation study. The presence of a natural-experiment identification package does not prove its assumptions; it records what must be reviewed. It does not verify sensor calibration certificates, privacy permissions, dataset licences, or empirical ODD coverage. Those need independent validation and evidence-specific reviewers.
