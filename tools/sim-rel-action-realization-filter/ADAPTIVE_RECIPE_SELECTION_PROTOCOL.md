# SIM-REL-038 — adaptive development-only recipe selection

Status: research design on a branch. This protocol is not yet an implemented adaptive selector and does not claim an adaptive benchmark result.

## 1. Objective

SIM-REL-036 provides deterministic, exact binary-channel challenge generation. SIM-REL-038 adds the missing control loop: select and refine challenge recipes using Development outcomes, without allowing Calibration or SealedHoldout results to influence future challenge generation.

The controller must make a benchmark more discriminating without simply making tasks unsolvable or optimizing against one agent.

## 2. Separation of responsibilities

Keep four components separate:

1. Recipe registry: immutable, versioned candidate challenge specifications and their declared mechanism strata.
2. Task compiler: deterministic instantiation of a recipe into a public task, a sealed exact oracle and a generation receipt (SIM-REL-036).
3. Outcome evaluator: independent task-validity and agent-outcome assessment under a versioned harness.
4. Adaptive selector: updates only from eligible Development outcome receipts and emits a deterministic recipe-selection receipt.

The selector must not be able to inspect sealed oracle fields, hidden seeds, sealed manifests or calibration/holdout outcomes. The compiler must not choose the split after looking at the generated task or its expected agent outcome.

## 3. Challenge strata

Maintain separate coverage targets for at least:

- inferential approximation / posterior misspecification;
- data-generating model-family inadequacy and omitted mechanisms;
- action realization, actuator and sensor faults;
- sequential complementarity, adaptive stopping and non-myopic value;
- rare high-consequence outcomes and asymmetric loss;
- physical safety, recovery and safe abort;
- held-out design ranking and scientific generalization.

A new physical stratum is not considered covered merely because a synthetic finite-channel task has a similar label.

## 4. Development outcome contract

An eligible outcome receipt binds:

- unique attempt ID and task ID;
- recipe ID, immutable recipe version and recipe digest;
- mechanism stratum and declared difficulty band;
- Development family-group ID;
- agent/model, tool, policy, harness and evaluator versions;
- task-validity result from the independent challenge verifier;
- outcome: solved, not solved, abstained, or invalid task;
- a controlled failure category, where applicable;
- observed resource consumption and budget exhaustion flags;
- raw trajectory / evidence artifact digest.

Only tasks whose validity verifier passes enter the difficulty statistics. Invalid tasks are recorded as generator defects and excluded from solver success/failure denominators. Abstention is a separate outcome, not silently counted as either success or failure.

The ingestion API should accept a typed Development receipt, not an arbitrary split string supplied by a caller. Calibration and SealedHoldout receipts must be rejected at the type/API boundary where practical and rechecked by the ledger.

## 5. Deterministic selection policy

Use a two-stage controller.

### Stage A — coverage deficit

Choose the most under-covered mechanism stratum relative to pre-registered targets. Coverage is computed from unique, valid Development family groups, not raw repeated task rows.

### Stage B — difficulty discrimination

Within the selected stratum:

1. give every candidate recipe a minimum number of paired attempts across the fixed agent panel;
2. retain per-agent outcomes as well as pooled results;
3. calculate confidence-aware success intervals only after the minimum sample count;
4. target a pre-registered discrimination band (initial proposal: 35–70% solve rate for the fixed panel);
5. select the immutable recipe variant whose interval is closest to the target band, subject to mechanism coverage and compute/safety budgets;
6. if all candidates are too easy, instantiate a new harder recipe version; if all are too hard, create a simpler version or a different mechanism representation;
7. preserve a deterministic tie-break based on the prior Development ledger digest and a recorded selector seed.

The numerical solve-rate band is a starting hypothesis to validate, not a universal benchmark law. The selector must report the full per-agent and per-stratum distribution; an average inside the band can hide a trivial task for one agent and an impossible task for another.

## 6. Immutable selection receipt

Every selection receipt should include:

- selector policy/version and source commit;
- prior Development ledger digest and event count;
- chosen recipe ID/version/digest and mechanism stratum;
- coverage deficits before selection;
- per-agent and pooled difficulty statistics used;
- minimum-sample status and uncertainty interval method/version;
- deterministic tie-break seed;
- reason code (coverage, too-easy refinement, too-hard refinement, balanced discrimination);
- intended split group and generation request digest.

Replaying the same registry, Development ledger and selector version must reproduce the same selection and receipt. A changed recipe must receive a new immutable version; never rewrite a recipe or task that has already been exposed.

## 7. Split firewall and freeze protocol

- Reserve Development, Calibration and SealedHoldout family groups before any challenge is generated.
- Only validated Development outcomes may update the adaptive selector.
- Calibration is used to evaluate selector/harness calibration and to select a final frozen policy only if that role was pre-registered; it cannot be recycled into Development.
- Freeze the selector version, candidate registry, recipe versions, agent panel, evaluator and sampling policy before opening sealed evaluation.
- Sealed outcomes are one-time measurements. They cannot update recipe difficulty, strata weights, selector priors, candidate registries or the next benchmark release.
- A failed sealed replay is still consumed and must be recorded as a failed evaluation, not retried for tuning.

## 8. Benchmark-validity metrics

Report independently:

- valid-task rate / false-task rate;
- recipe and mechanism-family coverage;
- task novelty and near-duplicate rate;
- solver success intervals by agent and stratum;
- difficulty stability across agent panels and versions;
- inter-evaluator agreement and verifier disagreement;
- shortcut/leakage detection rate;
- calibration and sealed split leakage;
- reproducibility from recipe + seed + source identity;
- selection entropy / concentration, so one stratum cannot monopolize the benchmark;
- generator compute cost and unsupported-task rate.

A fall in one agent's accuracy is not sufficient evidence that a generated benchmark is better. The challenge must remain valid, solvable, discriminative across a declared panel, and resistant to obvious shortcuts.

## 9. Required negative controls

- Inject Calibration and SealedHoldout outcomes into the Development update path.
- Change a sealed receipt and confirm the selector state does not change.
- Reorder or duplicate Development events and check deterministic replay / duplicate rejection.
- Mutate recipe contents without incrementing the recipe version.
- Supply invalid tasks and verify they do not alter the solver difficulty estimate.
- Starve one mechanism stratum and verify coverage selection corrects the deficit.
- Create one agent with extreme failure rate and verify pooled selection does not erase per-agent diagnostics.
- Create all-easy and all-hard candidate pools and verify explicit refinement/abstention rather than false discrimination.
- Replay selection from the exact ledger digest and verify identical recipe and receipt.

## 10. Qualification sequence

1. Exact deterministic replay on fixed Development ledgers.
2. Exhaustive small registries and synthetic outcome tables.
3. Split-leakage and mutation corpus.
4. Paired multi-agent trials with a pre-registered panel.
5. Sealed mechanism-family evaluation only after the selector is frozen.
6. Comparison against static recipes, random selection, difficulty-only selection and AutoSciBench-style iterative refinement under matched task validity and harness rules.

Use at least 30 paired seeds per family where practical; target 100 or more. Report paired confidence intervals and the complete failure taxonomy. Never optimize sealed results, and never advertise benchmark SOTA from a single score drop.


## 11. Current executable increment (SIM-REL-038a)

The branch now includes `decision_value::adaptive_recipe_selection`, a deterministic Development-only selector over an immutable, caller-supplied candidate registry.

The implementation currently enforces:

- Every outcome is explicitly tagged `EvidenceSplit::Development`; Calibration and SealedHoldout receipts fail closed.
- Every outcome is tied to a registered family group, fixed agent panel, recipe ID/version/fingerprint and mechanism stratum.
- Duplicate attempt IDs and repeated agent × recipe × task attempts are rejected.
- Invalid tasks are retained in a separate count but excluded from solve-rate and coverage denominators.
- Mechanism coverage is based on unique valid Development family groups.
- Candidate summaries preserve pooled and per-agent solved/abstained counts.
- Difficulty uses a conservative 95% Hoeffding interval with a fixed rational upper radius `sqrt(2/n)`, rounded upward in integer parts-per-million. No floating-point arithmetic is used.
- Candidate selection is deterministic from the caller seed and prior Development-ledger digest; the derived FNV tie-break is only a deterministic tie-break, not a cryptographic commitment.
- An immutable selection receipt records the prior digest/event count, selected candidate, reason, coverage state and all candidate difficulty summaries.

### Explicit implementation limits

This increment selects among already registered immutable recipe descriptors. It does not yet synthesize harder/easier recipe variants, perform semantic near-duplicate detection, or automatically evolve a recipe grammar. Those are later increments and must not rewrite recipes already exposed to an agent.

The caller-supplied ledger digest is validated as an identifier but not recomputed or authenticated here. The selection API trusts the supplied Development outcome receipts after checking their split, registry, agent-panel and identity bindings. Authentication of the external outcome/artifact ledger remains a separate provenance responsibility.

The 35–70% solve-rate band is a configurable initial operating hypothesis, not an empirically optimized or universal target. Per-agent intervals remain visible because a pooled score can mask an agent-specific failure mode. Calibration and sealed holdout outcomes are intentionally unavailable to this selector.
