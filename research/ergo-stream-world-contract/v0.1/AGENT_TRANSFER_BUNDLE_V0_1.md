# ErgoStream Agent Transfer Bundle v0.1

**Schema:** `schemas/agent_transfer_bundle_v0.schema.json`  
**Example:** `examples/agent_transfer_bundle_v0.example.json`  
**Composition:** Symthaea #11060 / #11171 · Symtropy #5990 / #7171 / #7193

## Purpose

This bundle adds a typed, independently replayable structural/dynamics-transfer sidecar without changing the existing blind `AgentView` / `AgentAction` JSONL protocol or the strict `AgentSubmission` v0 schema.

It is designed to be attached to an existing submission as a second, versioned artifact. The learner does not obtain a new episode runner, evaluator handle, hidden seed, or alternate model DSL.

## Binding and freeze order

The bundle's `agent_submission_binding` records:
- the base AgentSubmission protocol version;
- the digest of the exact serialized submission frame;
- source and target episode IDs;
- source and target evidence-prefix SHA-256 digests.

The owner must verify that:
1. the submission digest resolves to the exact already-received Submit frame;
2. each episode ID exists in the trusted episode owner;
3. each evidence-prefix hash matches the public view actually delivered for that episode;
4. `structural_transfer.source/target` repeat the same episode IDs and evidence-prefix hashes as the bundle binding;
5. structural snapshot evidence IDs resolve to observations available in the corresponding public prefix;
6. no evidence ID, node, relation, transition or prediction cites future or evaluator-only information.

The learner freezes the complete bundle before the evaluator releases hidden target transitions, holdout structural probes or intervention outcomes. The evaluator records a custody receipt over the bundle bytes/digest and runner identity; timestamps supplied by the learner are not proof of precommitment.

The existing AgentSubmission v0 is not mutated. A future change to the base JSONL protocol requires an explicit new protocol version and compatibility review.

## Structural proposal

`structural_transfer` references the standalone scoped proposal schema. Each source and target side now carries:
- episode ID and evidence-prefix digest;
- a finite node snapshot and relation snapshot;
- the public evidence IDs supporting each node and relation;
- selected and explicitly excluded nodes/relations;
- a declared map kind (currently isomorphism, induced embedding, or subgraph embedding);
- node, relation-kind and relation-instance correspondences;
- assumptions, alternatives, unresolved state and counterexample evidence IDs.

The evaluator reconstructs the selected structures from these snapshots, validates IDs and evidence references, validates complete scope partitions, then calls the finite scoped witness. It must not accept a map because the learner's own serialized witness says it passed.

The snapshots are candidate-authored structural interpretations of public observations. Their evidence references establish provenance linkage, not that the inferred nodes/relations are objectively true.

## Dynamics proposal

The optional `dynamics_transfer` currently supports only `deterministic_transition_commutation`. It contains:
- source and target state/action scopes, including excluded IDs;
- evidence-backed source/target state and action snapshots;
- total state mappings and action mappings with action-kind IDs;
- source-transition evidence IDs and candidate-predicted target successors;
- explicit assumptions and an unresolved flag.

It deliberately does **not** include the evaluator's actual hidden target transitions. The candidate freezes target successor predictions; after freeze, the trusted evaluator compares them with its owner-side transitions through `validate_dynamics_correspondence` or a stricter scope-specific adapter.

The current Rust finite dynamics validator expects complete deterministic source and target transition systems and total one-to-one state/action maps. It does not implement nondeterministic simulation, bisimulation, quotient dynamics or approximate/continuous transfer.

## Behavioural transfer proposal

The optional `behavioural_transfer` property uses the finite LTS contract in `src/behaviour.rs`. It carries:
- one combined source/target state/action scope matching `BehaviouralScopeV0`;
- evidence-backed source and target state/action snapshots;
- a many-to-many state relation (not a forced function);
- a one-to-one mapped action correspondence and consistent action-kind map;
- frozen response predictions bound to observed transition evidence;
- a declared map kind: `simulation` or `bisimulation`.

A simulation proposal predicts target responses to observed source transitions; additional target behaviours are allowed. A bisimulation proposal must also predict source responses to observed target transitions. The evaluator compares those frozen predictions with owner-side transitions after bundle freeze. Actual held-out transitions are never included in the learner sidecar.

The schema models candidate predictions; it does not itself prove the predictions correct. The Rust validator checks finite LTSs, while the trusted evaluator still has to construct the owner-side systems from public and hidden data and verify the custody/freeze order.

## Transfer probability forecasts and explicit abstention

The top-level optional `transfer_forecasts` field lets the learner forecast exact claim qualification:
- claim ID and claim type;
- transfer probability in ppm;
- decision: `assert_transfer`, `reject_transfer` or `abstain`.

It is a probability for the exact scoped claim, not general confidence in an analogy. For example, a valid induced embedding does not entail a valid mechanism transfer.

The trusted evaluator resolves claim IDs and computes calibration after the bundle is frozen. It owns the binary outcome labels, fixed reliability bins and selective-risk confidence thresholds. Do not include evaluator labels or tune thresholds against the sealed confirmatory outcomes. See `TRANSFER_CALIBRATION_V0_1.md`.

The optional `transfer_forecasts` field lets the learner forecast an exact scoped claim with a unique claim ID, claim type, transfer probability in ppm and one decision: `assert_transfer`, `reject_transfer` or `abstain`. Forecasts remain separate for structural correspondence, deterministic dynamics, simulation, bisimulation and mechanism transfer. A valid structural map is not evidence that a mechanism claim is valid.

The evaluator attaches `evaluation_stratum_id` to each owner-side outcome. That label is never included in the learner forecast and cannot be self-selected by the candidate. The metric helper validates the complete forecast/outcome cohort before producing per-stratum Brier, log-loss, ECE and selective-risk reports. This makes representation-only, semantic/mechanism, combined-shift and claim-family failures visible rather than hiding them in a pooled score. Bins, thresholds and outcome labels remain evaluator-owned. See `TRANSFER_CALIBRATION_V0_1.md`.



## Decision value and prospective forecasts

Do not add a new scheduler or VoC implementation to this bundle. A future action-selection record should compose with Symthaea #5476/#5478/#8721:
- immutable forecast before execution;
- evidence-prefix and operation identity;
- prospective decision-value prediction distinct from information gain;
- predicted resource cost, latency and downside;
- a separate outcome receipt after execution.

The current structural/dynamics bundle is a hypothesis artifact, not proof that the action chosen to acquire it had positive decision value. The finite #13418 / PR #13407 decision-value oracle is a useful Stage-A known-answer control, not a general research-policy callable.

## Existing policy / model ownership

- #11060 remains the blind episode wire format, finite worlds and baseline runner.
- #11171 remains the real Symthaea research-policy adapter owner.
- #5990 remains owner-side episode custody.
- #6027/#6065 remain the bounded typed model DSL / ScientificModel path.
- #7171/#7193 remain the finite structural/dynamics witness and paired-world scoring owners.

Keep the bundle optional and sidecar-versioned until these owners approve the adapter contract. Do not add a second JSONL runner, scheduler, posterior engine, model AST, evidence ledger or truth store.

## Validation limits

JSON Schema validation is necessary but not sufficient. The evaluator must additionally check:
- source/target episode and prefix binding consistency across the bundle and nested proposal;
- unique IDs and complete selected/excluded partitions;
- participant references and selected relation scope;
- evidence IDs exist in their claimed episode prefix;
- node/relation snapshots are derived only from public observations;
- maps cover precisely the selected scope;
- predicted target transitions are frozen before hidden outcomes;
- all actual target transition checks are performed owner-side.

An in-process schema/type boundary is not an OS sandbox. Learner/evaluator separation must continue to use the existing trusted process/episode owner.

## Reproduction / static checks

- Parse `agent_transfer_bundle_v0.schema.json` and `agent_transfer_bundle_v0.example.json` as JSON.
- Validate the example against the schema using a Draft 2020-12 validator.
- Cross-check bundle and nested source/target episode/prefix bindings.
- Cross-check snapshot IDs against selected/excluded scope partitions.
- Resolve every evidence reference against the owner-issued public episode prefix.
- Preserve exact artifact digests in the owner receipt.

The current example demonstrates a source/target induced embedding and two frozen target transition predictions. The bundle schema references the scoped structural proposal schema by its aligned relative schema ID; both side snapshots and the dynamics state/action snapshots carry evidence IDs. It is an illustrative wire artifact, not an executed AgentView/AgentAction integration or a causal-transfer result.

## Owner-issued split-conformal receipt

The split-conformal quantile and its calibration evidence are never placed in the candidate-authored AgentTransferBundle. The trusted episode/calibration owner issues a separate receipt using `schemas/split_conformal_transfer_receipt_v0.schema.json`. That receipt binds the frozen predictor, preprocessing, score function, calibration cohort, sample-ID manifest, evidence-prefix, evaluator stratum, independent unit, claim scope, alpha, rank and quantile, as well as the custody receipt. A model or cohort change requires a new receipt; a revoked receipt is unusable. See `TRANSFER_CALIBRATION_V0_1.md`.
