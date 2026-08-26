# Shadow Fidelity Telemetry Protocol

Status: research contract; no runtime fidelity reduction is authorized by this document.

## Purpose

The adaptive-physics program needs data before it needs intervention. This protocol defines how HDC retrieval, exact physical diagnostics, accounting/lifecycle evidence, and future CfC/LTC solver-error predictions are recorded while the authoritative solver continues to run unchanged.

The governing rule is:

> Unknown evidence is not favorable evidence.

A missing conservation residual is not zero. Missing accounting completeness is not complete accounting. Missing lifecycle provenance is not stable lifecycle. A raw nearest-neighbor similarity is not calibrated novelty. A predictor output without a declared target, metric profile, and held-out calibration provenance is not a physical-error bound.

## Shadow-only phase

During the initial campaign, the highest-certified physics path remains authoritative and unchanged. The shadow system may:

- observe exact-state digests;
- encode HDC frames and episodes;
- retrieve analogous episodes;
- record raw similarity;
- derive a conservative retrieval-novelty proxy for promotion analysis;
- record exact numerical health when available;
- record lower-level accounting diagnostics without upgrading them into stronger reconciliation claims;
- record interval-level accounting completeness when a real reconciliation producer exists;
- record authoritative lifecycle stability when a real lifecycle/transition producer exists;
- record conservation, constraint, activity, and causal signals when available;
- record calibrated semantic novelty when a versioned calibration exists;
- record calibrated solver-error predictions when a predictor, target fidelity, error-metric profile, and held-out calibration exist;
- compute a known-risk fidelity floor;
- report which evidence is still missing for a future reduction experiment.

It may not:

- reduce fidelity;
- reduce solver iterations/substeps;
- switch to a cheaper solver;
- mutate authoritative physics state;
- claim compute savings from an intervention that was not actually executed;
- treat missing measurements as zero/healthy/complete/stable;
- infer lifecycle legitimacy from endpoint digest equality;
- treat raw HDC similarity as calibrated novelty or physical error;
- reuse a scalar error estimate outside the metric profile and target fidelity for which it was calibrated.

## Observation schema

`ShadowFidelityObservation` carries:

- tick and exact-state digest;
- current fidelity tier;
- optional nearest-episode retrieval similarity;
- optional calibrated novelty plus encoder and calibration fingerprints;
- optional exact numerical-health signal;
- optional accounting-completeness signal;
- optional lifecycle-stability signal;
- optional normalized conservation/reconciliation residual;
- optional normalized constraint/contact error;
- optional physical-activity signal;
- optional causal-importance signal;
- optional calibrated solver-error prediction.

`None` means unknown. It is never converted to a favorable default.

### Calibrated novelty provenance

A calibrated novelty observation binds:

- novelty value;
- HDC encoder fingerprint;
- calibration fingerprint.

The explicit encoder fingerprint prevents novelty calibration from one semantic schema/configuration being silently reused after ranges, quantizers, dimensions, roles, or encoded features change.

### Calibrated solver-error provenance

A calibrated error prediction binds:

- target fidelity tier;
- predicted relative physical error;
- calibrated confidence;
- **error-metric profile fingerprint**;
- predictor fingerprint;
- calibration fingerprint.

Zero fingerprints are invalid. The target fidelity must be strictly cheaper/lower than the currently observed tier.

The metric-profile fingerprint is essential. A scalar such as `0.001` is uninterpretable without knowing whether it summarizes trajectory error, contact error, conservation/reconciliation residual, topology disagreement, task observables, or a declared conservative envelope over several of them.

## Error-metric profiles

Before a learned error estimate can count toward reduction readiness, the campaign must define a versioned metric profile. Prefer a multi-metric envelope rather than one arbitrary scalar when different failure modes matter.

A profile should declare, as applicable:

- position/orientation trajectory metric and scale;
- contact impulse / penetration / constraint metric;
- linear/angular momentum metric;
- per-reservoir energy and total reconciliation metrics;
- lifecycle/topology/event-disagreement metric;
- thermal/entropy metrics;
- task-specific observables;
- aggregation rule used to produce the reported relative error;
- catastrophic/fail-fast conditions that cannot be averaged away.

Changing any metric, normalization, weighting, envelope rule, or catastrophic condition requires a new metric-profile fingerprint and new held-out calibration.

## Retrieval novelty proxy

When calibrated novelty is unavailable, shadow telemetry may use the conservative proxy

`novelty_proxy = 1 - clamp(similarity, 0, 1)`.

Therefore:

- similarity `1.0` -> proxy novelty `0.0`;
- similarity `0.65` -> proxy novelty `0.35`;
- similarity `0.0` or negative -> proxy novelty `1.0`.

This proxy may raise the known-risk fidelity floor. It may never satisfy the calibrated-novelty requirement for a reduction experiment.

The mapping itself is a research baseline, not a claim that HDC similarity is a calibrated probability of physical novelty.

## Known-risk floor

The shadow assessment evaluates only signals that are actually present. Each known risk signal may independently raise the fidelity floor using the same threshold family as `AdaptiveFidelityPolicy`.

The result is named `known_risk_floor`, not `safe_fidelity`, because unknown evidence may require a higher tier.

Known evidence that independently forces the highest-certified tier includes:

- numerical-health failure;
- incomplete accounting;
- unstable/unresolved lifecycle;
- sufficiently large conservation/reconciliation residual;
- sufficiently large constraint error;
- sufficiently high calibrated or conservative-proxy novelty;
- sufficiently large predicted physical error.

No missing signal lowers the floor.

`AdaptiveFidelityPolicy` itself is validated before its thresholds are used. NaN or incoherent policy thresholds fail closed rather than silently changing shadow decisions.

## Completeness versus reduction readiness

The assessment deliberately distinguishes two concepts.

### `evidence_complete`

True only when all of the following are present and valid:

1. numerical-health status;
2. accounting completeness;
3. lifecycle stability;
4. conservation/reconciliation residual;
5. constraint/contact error;
6. physical activity;
7. causal importance;
8. calibrated semantic novelty with encoder/calibration provenance;
9. calibrated solver-error prediction with target/metric/predictor/calibration provenance.

A complete packet can be converted into `FidelityEvidence` without manufacturing defaults.

### `reduction_ready`

A complete packet is only a one-tier reduction candidate when, additionally:

- numerical health is true;
- accounting completeness is true;
- lifecycle stability is true;
- the error predictor targets exactly one tier below the current tier;
- the known-risk floor is below the current tier.

Failures are exposed as explicit `ShadowReductionBlocker` values.

`reduction_ready` still does **not** authorize intervention. It means only that the packet is a coherent candidate for the later adaptive controller, independent epistemic firewall, and authoritative intervention gate.

## Accounting semantics

Do not confuse a lower-level diagnostic with the stronger authority signal.

For example, `InvariantSnapshot::has_complete_modeled_energy_accounting()` establishes that every attached thermal reservoir participated in the current modeled energy total. That is useful telemetry, but by itself it does **not** establish:

- interval state-versus-ledger reconciliation;
- absence of untracked ledger ports;
- stable reservoir identity across endpoints;
- valid lifecycle provenance for reservoir appearance/disappearance;
- representation-transition accounting.

Therefore a runtime may record `modeled_energy_accounting_complete = true` while correctly leaving `ShadowFidelityObservation.accounting_complete = None` until the stronger interval-level evidence producer exists.

## Lifecycle semantics

Exact-state digest v2 records thermal presence and exact source-state bits, but a digest is an endpoint identity, not a transition receipt.

`lifecycle_stable = Some(true)` requires an authoritative producer that can establish the relevant body/reservoir/representation identity continuity or valid creation/removal/transition provenance for the assessed interval. It must never be inferred merely because two digests happen to match or because total numeric energy is unchanged.

## Calibration campaign

### Semantic novelty

Calibrate HDC novelty on scenario-stratified data rather than a random train/test split that leaks nearly identical trajectories across partitions.

At minimum retain:

- encoder fingerprint;
- exact digest version;
- corpus manifest;
- scenario-family split;
- similarity distribution for in-family states;
- similarity distribution for held-out regimes;
- chosen mapping from retrieval statistics to novelty;
- calibration fingerprint.

### Solver-error prediction

The preferred CfC/LTC target is not authoritative next-state replacement. It is the error incurred by a declared cheaper physics configuration relative to the highest-certified reference.

For candidate fidelity `S`, estimate:

`epsilon_hat(S, state, history, dt, metric_profile)`.

Ground truth must come from paired highest-certified/reference and candidate runs under the exact metric profile named in the prediction artifact.

The predictor must be calibrated on held-out scenario families before its output can populate `CalibratedErrorPrediction`.

## Live echo-memory integration

The existing `echo_memory_system` is the current runtime sampling point because it already observes the authoritative physics world, encodes deterministic HDC frames, builds temporally ordered episodes, and performs associative retrieval at a fixed sampling rate.

At present it knows:

- nearest-episode retrieval similarity when a prior episode exists;
- exact digest v2;
- numerical-health status;
- lower-level modeled-energy-accounting diagnostics.

It intentionally does **not** claim to know:

- interval-level accounting completeness;
- lifecycle stability;
- normalized conservation/reconciliation residual;
- calibrated constraint/contact error;
- normalized physical activity;
- causal importance;
- calibrated novelty;
- calibrated lower-tier solver error.

Those remain `None` until dedicated evidence producers exist. Consequently the live path cannot become `evidence_complete` or `reduction_ready` today.

The runtime keeps a bounded history and records assessment failures instead of silently dropping episodes whose telemetry or policy fails validation.

## First intervention gate

Only after shadow calibration and Phase-Zero authority gates should a paired intervention campaign be allowed.

Recommended initial restrictions:

- one-tier demotion maximum;
- prediction explicitly calibrated for that one-tier target;
- short bounded intervention windows;
- deterministic checkpoint before intervention;
- highest-certified shadow/reference replay available;
- periodic forced highest-certified checkpoints;
- automatic fallback on numerical-health failure, incomplete accounting, lifecycle uncertainty, high novelty, excessive residual, metric-envelope violation, or predictor disagreement;
- no solver-family switching in the first campaign.

Compare intervention against the reference run on both measured cost and every metric in the declared error profile. Report failures, not only successful windows.

## Claims ladder

### Shadow-observed

Allowed claim: the telemetry stack ran and produced reproducible observations.

### Calibrated

Allowed claim: novelty/error estimates achieved declared held-out calibration metrics for named encoder, target fidelity, and metric profile.

### Intervention-validated

Allowed claim: a declared adaptive policy reduced measured cost while staying inside its preregistered physical-error envelope on held-out scenarios.

### Competitive

Requires matched external-engine or baseline comparison under the Physics Excellence Program.

No earlier stage implies the next.

## Exit criteria for shadow phase

The shadow phase is complete only when:

- telemetry serialization/replay is deterministic under the declared mode;
- exact-state provenance is retained;
- missing signals remain explicit;
- accounting/lifecycle producers exist and have negative controls;
- novelty calibration has a versioned artifact bound to an encoder fingerprint;
- solver-error prediction has a versioned target fidelity, metric profile, predictor, and held-out calibration artifact;
- false-confidence and out-of-distribution failure cases are documented;
- replay reproduces the same shadow assessment from the same exact input trace;
- a preregistered first intervention protocol exists;
- Phase-Zero authoritative runtime/lifecycle/angular/contact prerequisites required by that intervention have executable evidence.
