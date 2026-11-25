# Independent Loss-Model Universe Adequacy Benchmark v0.1

**Status:** bounded synthetic reference evaluator; exact-head validation pending.  
**Owner:** [#7214 — independently challenge consequence-model coverage](https://github.com/Luminous-Dynamics/symtropy/issues/7214).  
**Implementation:** draft PR on the #7161 branch stack.  
**Claim ceiling:** synthetic loss-model universe adequacy relative to the declared grammar only.

## 1. The gap this closes

The incumbent bridge (#7161) verifies that the consumed joint-scenario rows match a pinned loss-model manifest and that the projected loss vector is exact. This is necessary but not sufficient: a producer can accurately hash and emit an incomplete manifest.

This benchmark independently derives the required model family and applicable physical-world × model pairs. It therefore distinguishes:
- physical-world coverage;
- loss-model family coverage;
- world/model applicability;
- emitted joint-scenario coverage;
- exact rational-to-integer projection;
- downstream decision regret.

A manifest digest proves integrity relative to a declaration, not completeness of the declaration.

## 2. Independent v0.1 grammar

Two bounded mission classes are registered:
- `orbital_vehicle`: propellant consumption, thermal stress, propulsion degradation and avionics reliability;
- `powered_flight`: propellant consumption, thermal stress and avionics reliability.

The grammar derives the model IDs independently of producer and consumer manifests. It also derives applicability from world profile tags:
- propellant and avionics models apply to all worlds;
- thermal stress applies only to worlds tagged `thermal`;
- propulsion degradation applies only to worlds tagged `propulsion`.

This is deliberately not a Cartesian-product assumption. A model can be required by the mission class but inapplicable to a particular physical world. Such an exclusion must be explicitly recorded with reason `profile_not_applicable` and a non-empty evidence receipt ID.

Every physical-world × required-model pair must have one applicability row, either applicable or an evidenced exclusion. Every applicable pair must appear in the #6939-style joint scenario output. Unknown mission classes are model-family inadequacy, not empty model sets.

## 3. Three declarations plus an independent oracle

The evaluator compares:
1. producer model inventory: model ID, family, version, scope, source receipt and producer state;
2. declared model inventory: consumer/applicability-side model IDs;
3. total applicability manifest: an explicit applicable or witnessed-excluded row for every world/model pair;
4. joint scenario output: actual scenario ID, world ID and model ID pairs.

The fourth model-family grammar is independent of all four records. Agreement between producer and consumer inventories is insufficient when both omit a required model.

## 4. Distinct failure states

- `LOSS_MODEL_UNIVERSE_CLOSED_WITHIN_DECLARED_GRAMMAR`
- `COMMON_MODE_MODEL_OMISSION`
- `MISSING_LOSS_MODEL`
- `MISSING_WORLD_MODEL_PAIR`
- `UNWITNESSED_MODEL_EXCLUSION`
- `MODEL_FAMILY_INADEQUATE`
- `MODEL_STALE`
- `MODEL_OUT_OF_PROFILE`
- `MODEL_SOURCE_UNAVAILABLE`
- `MODEL_SCOPE_MISMATCH`
- `MODEL_RECEIPT_ALIAS`
- `UNEXPECTED_MODEL`
- `UNEXPECTED_WORLD_MODEL_PAIR`
- `INVALID_CASE`

A receipt alias across distinct model IDs, stale model, out-of-profile model, source-unavailable model or scope mismatch is not a clean model-universe result.

## 5. Frozen challenge corpus

The 17 cases cover:
1. Complete orbital model universe and applicability.
2. Valid profile-based exclusions with evidence receipts.
3. Common-mode model omission from both producer and declared inventory.
4. One-sided producer inventory omission.
5. Missing world/model pair while the model exists elsewhere.
6. Unwitnessed exclusion row.
7. Unsupported mission class.
8. Stale model.
9. Out-of-profile model.
10. Unexpected model.
11. Unexpected world/model pair.
12. Missing source receipt.
13. Model scope mismatch.
14. Duplicate joint world/model pair.
15. Source receipt alias across distinct models.
16. Missing applicability row for a model that applies.
17. Missing applicability row for a model excluded by profile, which is an unwitnessed exclusion.

Rust unit tests include positive closure, common-mode omissions, pair omissions, witnessed/unwitnessed exclusions, unsupported mission classes, stale/out-of-profile models, unexpected models/pairs and permutation invariance. A separate JavaScript evaluator derives the expected model family and applicable pairs and is compared field-by-field with the Rust receipt.

## 6. Prospective benchmark targets

Targets, not measured real-world achievements:
- 100% recall of planted common-mode model omissions on finite oracle cases;
- 100% recall of required world/model pair omissions;
- zero false closed results on seeded omissions, stale models, scope mismatches or unwitnessed exclusions;
- zero unsupported model family accepted as complete;
- zero source receipt alias across distinct model IDs accepted;
- zero Rust/JS receipt mismatches;
- permutation invariance on all unordered inventory rows;
- report extra-model and over-approximation burden separately from omission recall.

For larger external evaluations, report model count, world/model pair count, exclusion count, dependency/common-cause coverage, reviewer time, runtime and memory at p50/p95/p99. Use independently authored challenge families and held-out model classes; repeated seeds from one grammar are not independent real-world evidence.

## 7. Integration and claim ceiling

#7214 owns model-universe adequacy. #6939 owns coherent joint scenario construction and regret inputs; #7161 owns source-pinned consumption and exact projection; #6887 owns physical/observation world evidence; #7093 owns evidence dependency graph adequacy; #7117 owns common-action incumbent adequacy.

The model-universe receipt is a necessary prerequisite for describing the loss-model family as closed. It does not prove that the grammar includes every real consequence mechanism, validate causal physics, calibrate model outputs, authenticate issuers, prove physical-world coverage or grant authority. The evaluator's positive result is only completeness within the named synthetic grammar and exact source head.
