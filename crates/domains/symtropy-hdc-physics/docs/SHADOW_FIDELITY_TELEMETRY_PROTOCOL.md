# Shadow Fidelity Telemetry Protocol

Status: research contract; no runtime fidelity reduction is authorized by this document.

## Purpose

The adaptive-physics program needs data before it needs intervention. This protocol defines how HDC retrieval, exact physical diagnostics, and future CfC/LTC error prediction are recorded while the authoritative solver continues to run unchanged.

The governing rule is:

> Unknown evidence is not favorable evidence.

A missing conservation residual is not zero. A missing constraint metric is not zero. A raw nearest-neighbor similarity is not a calibrated novelty estimate. A predictor output without held-out calibration provenance is not an error bound.

## Shadow-only phase

During the initial campaign, the highest-certified physics path remains authoritative and unchanged. The shadow system may:

- observe exact-state digests;
- encode HDC frames and episodes;
- retrieve analogous episodes;
- record raw similarity;
- derive a conservative retrieval-novelty proxy for promotion analysis;
- record exact numerical-health, conservation, constraint, activity, and causal signals when available;
- record calibrated semantic novelty when a calibration exists;
- record calibrated solver-error predictions when a predictor and held-out calibration exist;
- compute a known-risk fidelity floor;
- report which evidence is still missing for a future reduction experiment.

It may not:

- reduce fidelity;
- reduce solver iterations/substeps;
- switch to a cheaper solver;
- mutate authoritative physics state;
- claim compute savings from an intervention that was not actually executed;
- treat missing measurements as zero/healthy;
- treat raw HDC similarity as a calibrated physical-error estimate.

## Observation schema

`ShadowFidelityObservation` carries:

- tick and exact-state digest;
- current fidelity tier;
- optional nearest-episode retrieval similarity;
- optional calibrated novelty plus calibration fingerprint;
- optional exact numerical-health signal;
- optional normalized conservation residual;
- optional normalized constraint/contact error;
- optional physical-activity signal;
- optional causal-importance signal;
- optional calibrated solver-error prediction.

The calibrated error prediction additionally binds:

- predicted relative physical error;
- calibrated confidence;
- predictor fingerprint;
- calibration fingerprint.

Zero fingerprints are invalid. This is intentional: calibration provenance must be an explicit artifact, not an implied property of a number.

## Retrieval novelty proxy

When calibrated novelty is unavailable, shadow telemetry may use the conservative proxy

`novelty_proxy = 1 - clamp(similarity, 0, 1)`.

Therefore:

- similarity `1.0` -> proxy novelty `0.0`;
- similarity `0.65` -> proxy novelty `0.35`;
- similarity `0.0` or negative -> proxy novelty `1.0`.

This proxy may raise the known-risk fidelity floor. It may never satisfy the calibrated-novelty requirement for a reduction experiment.

The mapping itself is a research baseline, not a claim that HDC cosine/Hamming similarity is a calibrated probability of physical novelty.

## Known-risk floor

The shadow assessment evaluates only signals that are actually present. Each known risk signal may independently raise the fidelity floor using the same threshold family as `AdaptiveFidelityPolicy`.

The result is named `known_risk_floor`, not `safe_fidelity`, because unknown evidence may require a higher tier.

A known numerical-health failure may force `Exact` even if every other metric is missing. Likewise, high observed conservation residual, constraint error, causal importance, novelty, or predicted error may raise the floor independently.

No missing signal lowers the floor.

## Reduction readiness

An observation is only marked `reduction_ready` when all of the following are present and valid:

1. exact numerical-health status;
2. conservation/reconciliation residual;
3. constraint/contact error;
4. physical activity;
5. causal importance;
6. calibrated semantic novelty with a calibration fingerprint;
7. calibrated solver-error prediction with predictor and calibration fingerprints.

`reduction_ready` still does not authorize intervention. It means only that the evidence package is complete enough to be passed into a later controlled experiment and the independent Physics Epistemic Firewall.

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

For candidate solver/fidelity `S`, estimate:

`epsilon_hat(S, state, history, dt)`.

Ground truth should be computed from paired exact/reference runs using declared metrics such as:

- position/orientation trajectory error;
- momentum error;
- per-reservoir energy residual;
- constraint/contact error;
- topology/event disagreement;
- task-specific physical observables.

The predictor must be calibrated on held-out scenario families before its output can populate `CalibratedErrorPrediction`.

## First intervention gate

Only after shadow calibration should a paired intervention campaign be allowed.

Recommended initial restrictions:

- one-tier demotion maximum;
- short bounded intervention windows;
- deterministic checkpoint before intervention;
- exact shadow/reference replay available;
- periodic forced exact checkpoints;
- automatic exact fallback on numerical-health failure, high novelty, excessive residual, or predictor disagreement;
- no solver-family switching in the first campaign.

Compare intervention against the exact/reference run on both cost and physical error. Report failures, not only successful windows.

## Claims ladder

### Shadow-observed

Allowed claim: the telemetry stack ran and produced reproducible observations.

### Calibrated

Allowed claim: novelty/error estimates achieved declared held-out calibration metrics.

### Intervention-validated

Allowed claim: a declared adaptive policy reduced measured cost while staying within declared physical-error bounds on held-out scenarios.

### Competitive

Requires matched external-engine or baseline comparison under the Physics Excellence Program.

No earlier stage implies the next.

## Integration target

The existing `echo_memory_system` is the natural runtime sampling point because it already observes the authoritative physics world, encodes deterministic HDC frames, builds temporally ordered episodes, and performs associative retrieval at a fixed sampling rate.

The first runtime integration should emit shadow records only. It should not alter `PhysicsWorld`, fidelity, substeps, solver selection, or scheduling.

## Exit criteria for shadow phase

The shadow phase is complete only when:

- telemetry serialization is deterministic;
- exact-state provenance is retained;
- missing signals remain explicit;
- novelty calibration has a versioned artifact;
- solver-error prediction has a versioned held-out calibration artifact;
- false-confidence and out-of-distribution failure cases are documented;
- replay reproduces the same shadow decisions from the same exact input trace;
- a preregistered first intervention protocol exists.
