# CFI-Space v0.1 — Counterfactual Fault-Injection Benchmark

Issue: [#7828](https://github.com/Luminous-Dynamics/symtropy/issues/7828)

## Purpose

CFI-Space evaluates whether a spacecraft perception/relative-navigation stack detects and contains plausible sensor and model failures before a plausible-looking but invalid pose can flow into a decision. It complements pose accuracy, calibration, and OOD benchmarks; it does not replace them.

The primary unit is a **counterfactual episode**: a frozen scene/trajectory/sensor configuration with a clean reference and one or more controlled fault variants. The evaluator knows the injected fault; the candidate does not.

## Research hypotheses

1. **Agreement can be unsafe.** RGB/event estimates can agree while sharing a biased clock, extrinsic, target model, renderer, training set, annotation lineage, or calibration artifact.
2. **Framewise calibration is insufficient.** A pose stream can have acceptable marginal frame coverage while missing at least one critical frame in an operational horizon.
3. **Geometry-preserving appearance translation is a falsifiable assumption.** Translation must pass independent keypoint, visibility, crop, mask-topology, and temporal-motion checks before translated samples are admitted to calibration.
4. **Abstention is a measurable capability.** A system should distinguish safe-to-score, out-of-domain, dependency-unknown, disagreement, and modality-unavailable states rather than silently selecting a preferred sensor.
5. **Decision quality is not reducible to pose error.** A slightly less accurate estimator that correctly abstains under a common-cause fault may be preferable to a low-error estimator that silently accepts a biased pose.

## Evaluation unit and split policy

- Primary independent unit: target × session/trajectory × sensor-rig/configuration. The manifest must declare which grouping key is the independence unit; do not assume frames are independent.
- All frames in an episode share a fixed horizon and are scored jointly.
- Development, calibration, confirmatory, and blind-holdout manifests are disjoint by target/session/trajectory and by any known renderer or data-lineage group relevant to the claim.
- Counterfactual variants derived from one parent episode remain in the same split.
- No random frame split. No test-time calibration on the blind holdout.
- Public synthetic fixtures validate scorer semantics only. Blind performance requires evaluator custody and candidate/evaluator process separation; public labels are not blind.

## Fault taxonomy

| Family | Examples | Required expected behaviour |
|---|---|---|
| RGB-local | saturation, underexposure, blur, dead rows, dropout | detect degradation; use valid alternate evidence only if scope and dependence permit |
| Event-local | polarity loss, refractory/dead pixels, background activity, burst loss | detect degradation; abstain if observability falls below declared floor |
| Timing | fixed offset, drift, jitter, stale timestamps, rolling-shutter mismatch | OUT_OF_SCOPE or DEPENDENCE_UNKNOWN; never fuse mismatched epochs |
| Extrinsics/conventions | baseline shift, axis swap, unit mismatch, transform-direction mismatch | reject scope mismatch; do not normalize away an unverified convention |
| Common cause | shared clock, optics, target geometry, renderer, training/annotation/calibration lineage | DEPENDENCE_UNKNOWN unless independent evidence closes the declared dependency |
| Translation | keypoint drift, visibility/crop change, mask IoU/topology change, temporal jitter | reject translated calibration unit when any required geometry invariant fails |
| Dynamics/observability | acceleration, occlusion transition, weak parallax, low event activity | OOD/LOW_OBSERVABILITY abstention or explicitly qualified dynamic model |
| Integrity | digest substitution, missing dependency, stale revision, replayed identity, incomplete horizon | fail closed; no candidate-consistent status |

## Required benchmark arms

1. RGB-only estimator.
2. Event-only estimator.
3. Matched uncertainty-driven per-keypoint fusion (UE-Pose-style).
4. Matched physics-cue/local reliability fusion.
5. Geometry + temporal smoothing baseline.
6. Symtropy consistency/dependency gate without pose fusion.
7. Symtropy gate + independent-unit conformal sequence regions + translation audit.
8. Research-only correlation-aware fusion. This arm is excluded from promotion until its dependence assumptions and tangent-space covariance handling are separately qualified.

Every arm receives the same candidate-visible inputs, preprocessing opportunity, data splits, compute budget and output deadline. Paper-reported scores are context, not matched head-to-head results.

## Generalization matrix: do not let unseen spacecraft disappear in averages

The evaluator ledger now carries one explicit generalization stratum per episode:
- `seen_target`
- `unseen_target`
- `unseen_target_family`
- `unseen_renderer`
- `unseen_sensor_rig`

Score fault containment, false abstention and unsafe acceptance separately for every stratum. Counterfactual siblings remain in one stratum and one split. A large seen-target denominator must not hide failure on a new target family or sensor rig.

This is motivated by SpaceSense-Bench (2026), which supplies 136 satellite models, synchronized RGB/depth/LiDAR, seven part classes and 6-DoF pose, and reports that zero-shot generalization to unseen spacecraft and recognition of small components such as thrusters and omni-antennas remain difficult. The CFI-Space scorecard should therefore report small-part strata explicitly wherever the dataset provides independent labels. The public synthetic P1 fixture currently exercises only `seen_target` and `unseen_target`; it does not claim performance on unseen target families, renderers or rigs.

## Scorecard: report every row, never collapse to one scalar

### Pose and geometry
- translation and rotation median, mean, P95, P99;
- success rate at mission-declared tolerances;
- reprojection residual and rigid-body consistency;
- sequence maximum error and duration above tolerance.

### Uncertainty and calibration
- simultaneous complete-horizon coverage by independent unit;
- region size/volume and sharpness;
- conditional coverage by illumination, angular rate, range, occlusion, modality health and domain;
- silent-miss rate: output claims valid while the true pose lies outside the declared region;
- coverage-risk and availability-risk curves.

### Fault detection and containment
- per-family AUROC/AUPRC (diagnostic only);
- recall at a predeclared false-alarm rate and precision at a predeclared recall;
- detection delay in frames and milliseconds;
- fault-family localization accuracy;
- unsafe accepts, correct abstentions, unnecessary abstentions and time to recover;
- exact denominator and per-case disposition.

### Temporal behaviour
- drift, jitter, discontinuity, stale-data detection, sequence-level failure rate;
- maximum consecutive unqualified frames;
- recovery overshoot and settling duration.

### Resource envelope
- p50/p95/p99 end-to-end latency, throughput, peak resident memory, energy/frame, model size;
- hardware, compiler/runtime, precision, thermal/power mode and batch size.

### Evidence integrity
- exact input/output digests and replay determinism;
- dependency declaration completeness;
- split/lineage leakage controls;
- malformed and incomplete-input rejection;
- scorer parity across independent implementations.

## Non-compensatory promotion gates

- Zero unsafe accepts on the enumerated deterministic fault suite. Publish the complete finite denominator and case list; this is not a universal zero-risk statement.
- No average score can compensate for a failed critical fault family.
- Coverage is computed over independent complete episodes, not frame counts alone.
- Confidence intervals and denominators are mandatory. Cluster/bootstrap at the declared independent-unit level; do not treat correlated frames as independent samples.
- Fault detection is scored at predeclared operating points; AUROC alone is not a deployment criterion.
- Missing fault families are OUT_OF_SCOPE, never implicitly passed.
- A clean synthetic pass does not imply public-real, HIL, physical or flight qualification.

## Implementation phases

### P0 — contract and scorer (this branch)
- Freeze manifest schema, taxonomy, scorecard, promotion gates and claim ceiling.
- Add tiny synthetic known-answer fixtures only after schema review.
- Keep evaluator fault ledger separate from candidate-visible manifest.

### P1 — deterministic corpus and Rust scorer
- Build deterministic episode/variant identity and counterfactual pairing.
- Implement integer/fixed-point metrics where practical; define rounding and overflow behaviour.
- Add an independent challenger and raw-byte corpus digest binding.
- Negative controls: missing frames, duplicate identities, changed parent, altered horizon, unknown fault, digest mismatch, split leakage.

### P2 — matched baselines
- Adapter contracts for RGB, event, per-keypoint fusion, physical cue fusion and temporal geometry.
- Resource-normalized execution; no unmatched paper score comparisons.
- Preserve original modality estimates and uncertainty; no implicit fusion fallback.

### P3 — public-real and blind evaluation
- Lawful data/label admission, target/session grouping and rights register.
- Candidate/evaluator separation and hidden fault ledger.
- Prospective holdout and reproducible scorecard.

### P4 — hardware and operational qualification
- Exact sensor and compute configuration.
- HIL fault injection, clock/extrinsic perturbation, thermal/power constraints and measured end-to-end latency.
- Physical tests and mission-specific safety case remain independent promotion gates.

## Existing work ownership

- #7786: split/translation lineage auditor.
- #7800/#7801: independent-unit conformal calibration.
- #7806/#7808: translation geometry and temporal-label preservation.
- #7810/#7813: RGB/event consistency and declared common-cause gate.
- #7782: statistical pose regions.
- #7715: synchronization; #7743: pose trustworthiness.

CFI-Space owns cross-program benchmark orchestration, counterfactual pairing, baseline protocol, scorecard and blind evaluation governance. It must consume these outputs rather than duplicate or silently reinterpret their contracts.

## Claim ceiling

This document specifies a research benchmark. No estimator accuracy, fault rate, physical performance, SOTA superiority, deterministic safety outside the finite injected suite, flight readiness or certification is claimed. Synthetic known-answer fixtures validate scorer semantics only.

## P1 executable contract slice (2026-11-27)

The first machine-readable and executable contract increment is now present:

- `schemas/cfi-space/candidate-manifest-v0.1.schema.json`: candidate-visible episode/frame identity, fixed horizon and modality metadata. It contains no injected fault labels or expected dispositions.
- `schemas/cfi-space/evaluator-ledger-v0.1.schema.json`: evaluator-side fault identity, variant type, independent-unit group, split and expected disposition. This file must not be mounted in a candidate process.
- `schemas/cfi-space/predictions-v0.1.schema.json`: candidate output disposition and detected fault-family labels.
- `research/spacecraft/cfi-space/fixtures/`: ten public synthetic episodes across two independent units and two counterfactual groups, including clean, single-fault and interaction cases. The public evaluator ledger is a scorer fixture, not a blind secret.
- `tools/cfi-space-bench/`: strict typed Rust scorer that checks schema versions, recomputes SHA-256 over exact source-bundle bytes, rejects path traversal/symlink escapes, and cross-checks the bundle's episode ID, frame IDs, sequence numbers, timestamps, modality list, frame budget and fixed horizon against the candidate manifest. Predictions must bind to the verified source-bundle digests. It also validates independent-unit split isolation, counterfactual sibling integrity and complete prediction coverage.
- `scripts/cfi-space-challenger.py`: independent standard-library implementation of the joins, byte-digest verification, bundle-to-manifest metadata cross-checks and score counts. It rejects duplicate JSON keys and boolean schema versions. CI compares complete Rust/Python JSON receipts for positive and unsafe-acceptance controls.
- `fixtures/bundles/*.bin`: tiny, deliberately labelled **non-image synthetic payloads**. Their byte digests pin the data, and their internal header/frame records must agree exactly with candidate-visible frame identity, time, modality and horizon metadata. These fixtures are not spacecraft images or sensor captures.

Run locally:

```sh
cargo fmt --manifest-path tools/cfi-space-bench/Cargo.toml -- --check
cargo test --manifest-path tools/cfi-space-bench/Cargo.toml
cargo clippy --manifest-path tools/cfi-space-bench/Cargo.toml --all-targets -- -D warnings

cargo run --quiet --manifest-path tools/cfi-space-bench/Cargo.toml -- \
  --manifest research/spacecraft/cfi-space/fixtures/synthetic-candidate-manifest-v0.1.json \
  --ledger research/spacecraft/cfi-space/fixtures/synthetic-evaluator-ledger-v0.1.json \
  --predictions research/spacecraft/cfi-space/fixtures/synthetic-predictions-safe-v0.1.json \
  --expect-unsafe 0 --bundle-root .
```

The paired unsafe prediction fixture deliberately marks a common-cause-corrupted episode `qualified`; the scorer must report exactly one unsafe acceptance when run with `--expect-unsafe 1`. The focused workflow runs both controls.

### P1 scorer limits

The scorer recomputes SHA-256 over the committed synthetic non-image payload bytes and validates the supplied result joins. This establishes byte integrity for those fixtures only; it does not decode images, authenticate a sensor/source or camera capture chain, estimate pose, measure physical fault prevalence, prove real statistical independence, or establish blind custody.

The scorecard reports raw counts and per-family detection counts. It intentionally does not claim a meaningful precision/recall confidence interval from this ten-episode synthetic fixture. Those statistics require independently sampled target/session/trajectory units and prospective holdout data.

### Next P1 increment

Add canonical manifest receipts and broader malformed/incomplete episode variants. After that, integrate the Symthaea observer witness as an additional candidate output without changing the pose/authority gates.

## P1 exact-head scorer qualification (2026-11-28)

Dedicated workflow run [46122592113](https://github.com/Luminous-Dynamics/symtropy/actions/runs/46122592113) passed on exact head `d872e59fa11ad66d2aed9926d19820070358b2a7`:

- Rust formatting check passed.
- 11 Rust unit/adversarial tests passed.
- Strict Clippy with warnings denied passed.
- Positive-control Rust scorer and independent Python challenger passed with identical canonical score receipts.
- Negative-control Rust scorer and independent Python challenger passed with identical receipts, deliberately counting one unsafe acceptance.

Synthetic known-answer counts:
- 10 episodes, 2 independent target/session/trajectory/rig units, 2 counterfactual groups.
- 2 clean and 8 fault episodes.
- Safe control: 0 unsafe accepts, 0 false abstentions, 8/8 fault episodes contained, 10/10 exact disposition matches.
- Unsafe control: common-cause-corrupted episode incorrectly labelled qualified; scorer counts exactly 1 unsafe acceptance and 9/10 exact disposition matches.

This qualifies the P1 scorer/fixture semantics only. Two independent units and ten synthetic episodes are intentionally far too few for generalization, rare-event safety-rate, or statistical confidence claims. The candidate bundles are synthetic non-image payloads; this run does not execute Symthaea or evaluate spacecraft imagery.

## P2 factorized pose scorecard v0.2 (2026-11-28)

CFI-Space now extends the disposition-only score into a component-wise 6-DoF pose and uncertainty-region contract, motivated by the complementary orientation/translation strengths observed in the 6 November 2026 SPARK 2026 event-pose comparison.

New files:
- `schemas/cfi-space/evaluator-ledger-v0.2.schema.json`: evaluator-only 6-DoF ground truth, annotation digest and label-audit digest.
- `schemas/cfi-space/pose-predictions-v0.2.schema.json`: candidate pose, uncertainty region, and separate orientation/translation method identifiers.
- `research/spacecraft/cfi-space/fixtures/synthetic-evaluator-ledger-v0.2.json`.
- `research/spacecraft/cfi-space/fixtures/synthetic-pose-predictions-{safe,unsafe}-v0.2.json`.

Candidate pose receipts use:
- translation vector in metres;
- XYZW unit quaternion, sign-invariant SO(3) geodesic scoring;
- explicit coordinate frame and transform direction;
- uncertainty translation box (metres) plus orientation geodesic cap (radians);
- separate orientation and translation estimator method identifiers.

The evaluator ledger contains the reference pose and label-audit digests. It must remain evaluator-only. The scorer rejects non-finite coordinates, non-unit quaternions, frame/direction mismatch, reversed region bounds, malformed ground-truth audit digests, qualified predictions without pose+region+component methods, and any pose emitted on an abstention/non-qualified disposition.

Scorecard additions:
- per-episode translation absolute error (m) and relative translation error;
- SO(3) geodesic rotation error (rad and degrees), with quaternion antipodes treated as the same rotation;
- translation-box + SO(3)-cap coverage, denominator and silent-miss count;
- median and P95 component errors without collapsing metres and radians into an opaque single metric.

Run locally with the v0.2 files:

```sh
cargo test --manifest-path tools/cfi-space-bench/Cargo.toml
cargo clippy --manifest-path tools/cfi-space-bench/Cargo.toml --all-targets -- -D warnings

cargo run --quiet --manifest-path tools/cfi-space-bench/Cargo.toml -- \
  --manifest research/spacecraft/cfi-space/fixtures/synthetic-candidate-manifest-v0.1.json \
  --ledger research/spacecraft/cfi-space/fixtures/synthetic-evaluator-ledger-v0.2.json \
  --predictions research/spacecraft/cfi-space/fixtures/synthetic-pose-predictions-safe-v0.2.json \
  --expect-unsafe 0 --bundle-root .
```

The safe known-answer fixture has two qualified pose estimates, both regions covering the evaluator truth. The unsafe control also qualifies a common-cause-corrupted episode with a biased translation/orientation and a tight region around that incorrect estimate; the scorer should report one unsafe acceptance and one silent region miss. The fixture demonstrates scorer semantics only; it is not a real estimator result or a meaningful coverage estimate.
