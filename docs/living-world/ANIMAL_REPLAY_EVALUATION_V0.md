# Animal Replay Evaluation V0

Status: draft evaluation contract  
Research snapshot: 2026-10-19  
Tracks: [ANIMAL-REPLAY-001](https://github.com/Luminous-Dynamics/symtropy/issues/2149), [ANIMAL-OBSERVATION-001](https://github.com/Luminous-Dynamics/symtropy/issues/2128)

## Purpose

Define how Symtropy/Symthaea can demonstrate measurable progress toward useful, causally grounded animal simulation and animal-kingdom coverage. This contract connects capability profiles and rights-bound observation manifests to task-specific replay results. It does **not** claim that these results or the existing synthetic oracles validate biological realism.

The central shift is from *one plausible-looking animal* to **a falsifiable, calibrated model that predicts held-out observations and the direction and magnitude of responses to controlled interventions, within a declared taxon/stage/condition envelope**.

A result must answer: which capability, measured by what observable, on which independent holdout, against which baselines, with what uncertainty, at what compute cost, and under what claim ceiling?

## Paradigm shifts

1. **From species count to capability coverage.** Count represented body plans, sensory modalities, locomotion strategies, life stages, and ecological roles. A long species list is not coverage if every profile is an unvalidated copy of one quadruped.
2. **From visual plausibility to observable agreement.** A video can support tracking or pose metrics; it cannot by itself establish force, metabolism, internal physiology, motivation, or cognition.
3. **From next-frame prediction to intervention prediction.** Evaluate whether the model predicts what changes when a sensory cue, terrain, food availability, temperature, injury proxy, social partner, or energy constraint is deliberately changed. Observational correlation alone cannot establish causal response.
4. **From a single deterministic trajectory to calibrated ensembles.** Animals are stochastic and contexts are incompletely observed. Preserve multimodality, predictive intervals, abstention, and uncertainty rather than selecting one plausible trajectory.
5. **From random splits to explicit transfer claims.** Separate familiar-frame interpolation, unseen-individual, unseen-study/site, future-time, unseen-taxon, and cross-stage evaluations. Never label one as another.
6. **From pooled averages to a per-taxon evidence ledger.** Report counts, uncertainty, missingness, worst-performing strata, and failures. Macro-average taxa so abundant/easy datasets cannot hide poor performance on rare body plans.
7. **From render-bound behavior to presentation-independent dynamics.** The same physical state and inputs should have the same causal outcomes with rendering on/off and across supported fidelity settings, within explicitly declared numerical tolerance.
8. **From leaderboard chasing to reproducible scientific claims.** Pin data manifest, rights receipt, split, taxonomy release, code HEAD, model/profile, config, seeds, hardware/runtime and evaluator. A queued check or mutable benchmark URL is not a result.

## Evaluation hierarchy

Keep these evidence levels separate in the UI, reports and APIs.

- **Q0 — Contract/schema:** typed task, metric, profile, reference, split and result structures validate.
- **Q1 — Synthetic oracle:** frozen known-answer tests, including adversarial controls, agree with independently authored expected dispositions.
- **Q2 — Data and rights reviewed:** exact source revision, artifact, attribution, allowed operations, measurements, identity grouping and sensitive-location handling are recorded. This is not model validation.
- **Q3 — Replay reproduced:** the same immutable inputs recreate the result receipt, with declared numeric tolerances where exact floating-point equality is not portable.
- **Q4 — Reference task qualified:** a named model passes a narrow real-reference task on a frozen holdout against stated baselines.
- **Q5 — Scientific/counterfactual qualification:** held-out experimental conditions and preregistered interventions support an explicit causal claim.
- **Q6 — Transfer qualified:** a separate target taxon, stage, site or condition passes its own predeclared qualification gate.

Passing Q0–Q3 never implies Q4–Q6. Synthetic oracles can prove that the evaluator rejects bad evidence; they cannot prove an animal model is true.

## Task families and metrics

Each benchmark entry declares its **primary observable**, claim ceiling, unit of analysis, split group, metric version, uncertainty procedure, baselines, and acceptance threshold before evaluating the sealed test partition.

| Task family | Appropriate metrics | Useful baselines | Claim ceiling |
|---|---|---|---|
| Detection / pose | COCO-style AP/OKS-AP, keypoint PCK at declared thresholds, per-keypoint error, visible/occluded subgroup scores | Published baseline under the same boxes, image resolution, split and training-data rules | Image-space detection/pose only |
| Multi-animal tracking | HOTA, DetA, AssA, ID switches, fragmentation, track coverage, per-category scores | Strongest reproducible tracker under the exact split/protocol | Visual identity association and tracks |
| Movement forecasting | ADE/FDE where appropriate, negative log likelihood or CRPS for probabilistic forecasts, interval coverage, turn/step distributions | Persistence/last-state, constant velocity, fitted statistical movement model, published task-specific models | Forecast of observed movement, not its hidden cause |
| Kinematics / gait | Joint-space error, stride/stance timing error, speed/turn distribution, symmetry and variability, event timing | Empirical mean/phase model and strongest reproduced domain baseline | Kinematics within reference coordinate/calibration limits |
| Contact / locomotion | Contact-event precision/recall, slip/fall/obstacle rates, terrain success, force/torque error only when measured, energy per distance if supported | Simple controller, fixed-parameter physics baseline, best task-specific controller | Tested mechanics and conditions only |
| Sensorimotor response | Cue detection/response latency, path efficiency, task success, path integration error, sensory-ablation effects | Random/reactive controller, single-modality controller, best published/reproduced controller | Defined experimental task and sensory conditions |
| Behavior sequences | Per-class precision/recall/F1, calibrated transition matrices, event-time error, sequence likelihood and duration distributions | Majority/Markov baseline, trained task-specific classifier/model | Declared observed behavior labels; not motivation or cognition |
| Social/ecological dynamics | Interaction-network statistics, contact/avoidance rates, spatial/temporal distributions, survival/recruitment or population statistics when measured | Null/random mixing, fitted statistical/ecological models | Named group/population scale and environment |
| Counterfactual response | Error in treatment-effect estimate, sign accuracy, interval coverage, false intervention response on null controls | No-effect, observational-only, and simplest mechanistic baseline | Interventions and outcomes actually measured |
| Runtime/scaling | p50/p95 latency, steps per second, memory, energy if measured, deterministic replay rate | Same machine, precision, output resolution and fidelity | Declared hardware/software profile |

Do not average incompatible measurements into a universal “animal realism” number. Publish a multidimensional scorecard; any application-specific composite must expose its predeclared weights and never conceal a failed hard gate.

## External benchmark anchors (snapshot 2026-10-19)

These are external **task baselines**, not evidence that Symtropy has reached them. Recheck each exact release/protocol when implementing an adapter.

### AP-10K pose

The AP-10K dataset describes 10,015 annotated images spanning 54 species and 23 families. A third-party leaderboard snapshot lists ViTPose++ at 80.4 AP on validation and ViTPose++-H at 82.4 mAP on test; these are different splits/settings and should not be compared as though they were one number. The leaderboard includes model/config details and notes ground-truth boxes for listed test baselines. Sources: [official AP-10K repository](https://github.com/AlexTheBad/AP-10K), [validation leaderboard](https://www.sota2.com/research/sota/animal-pose-estimation-on-ap-10k-val), [test leaderboard](https://www.sota2.com/research/sota/animal-pose-estimation-on-ap-10k-test).

**Target:** first reproduce a legitimate baseline under the exact split, bounding-box protocol, keypoint convention and permitted training data. A future “beats SOTA” claim requires a controlled rerun on the same evaluation protocol, report of parameter/compute budget, and per-species/family and occlusion breakdown. Do not compare image-space pose scores to simulation validity.

The official AP-10K repository lists CC BY 4.0 for the dataset, but that is not blanket clearance for every artifact or operation. Bind the exact artifact, required attribution/change notices, source-image or third-party terms and contemplated analysis/training/redistribution separately before use. Do not retrieve payloads simply because this reference is named here.

### VastMAT multi-animal tracking

The September 28, 2026 preprint reports 2,947 videos, 1,002,562 annotated frames, 337 animal categories and 22,883 identity trajectories. It reports its strongest baseline at HOTA 66.37 on seen categories and 52.90 on category-disjoint unseen categories. The preprint says benchmark/code release is planned, so the exact public artifacts and rights must be verified before any ingestion; the figures are preprint-reported and not an approved dependency. Source: [VastMAT preprint](https://arxiv.org/abs/2609.34390).

**Target:** under the exact released protocol, aim first to reproduce the published scores within a preregistered tolerance; then target at least +3 absolute HOTA points on category-disjoint unseen evaluation without losing more than 1 point on the seen-category split, subject to a fair compute/training-data comparison. This is an aspirational engineering goal, not a result. Also report DetA/AssA, per-category uncertainty, fragmentation and failures. Never claim the tracker benchmark validates behavior, cognition or physiology.

### NeuroMechFly v2 / FlyGym

NeuroMechFly v2 is a useful narrow reference for experimentally grounded adult Drosophila sensorimotor control. The paper describes walking kinematics, path integration, head stabilization, multimodal vision/olfaction navigation, complex odor-plume navigation, and fly-following; its data/code availability are documented by the authors. Source: [Nature Methods paper](https://www.nature.com/articles/s41592-024-02497-y), [FlyGym project](https://github.com/NeLy-EPFL/flygym/).

**Target:** reproduce one task and its relevant empirical outputs on a pinned release before extending it. Compare empirical and simulated distributions, task success, path/heading error and response to declared sensory manipulations. For a counterfactual claim, preregister interventions and null controls; report effect-size error and uncertainty coverage. It is a Drosophila-specific mechanistic anchor, not a generic arthropod benchmark or universal cross-taxon proof.

### Movement ecology / Movebank

Movebank is a rich source ecosystem, not one homogeneous benchmark: its September 2026 platform totals are 11.8 billion locations, 9.5 billion other animal-borne sensor measurements, 10,192 studies and 1,712 taxa. Studies have independent owners and permissions; IDs should not be presumed globally unique, and sensitive animal locations need conservation-aware handling. Sources: [Movebank overview](https://www.movebank.org/cms/movebank-content/about-movebank), [data policy](https://www.movebank.org/cms/movebank-content/data-policy), [data model](https://www.movebank.org/cms/movebank-content/mb-data-model).

**Target:** start with metadata-only qualification and a specific published/open study whose precise intended operations are permitted. Namespace subject IDs by study unless source-backed identity links prove equivalence; bind site/time splits; compare against persistence and a fitted movement model. Include environmental/time covariates only if they are available at prediction time. Never treat missing tracks or species fields as zero activity or expose sensitive locations.

## Acceptance targets

The values below are **targets to implement**, not current accomplishments.

### Hard gates

- Zero false accepts in the frozen hostile corpus for revoked/restricted/unknown permissions, forbidden uses, known identity leakage, prohibited location disclosure, unsupported taxon/stage claims, or synthetic evidence relabelled as empirical.
- 100% agreement with independently authored expected dispositions and reason codes on the current synthetic profile and observation corpora, plus an independently authored replay-result oracle.
- 100% of benchmark results include exact code/config/data/split/taxonomy/evaluator identities and per-stratum counts; no silent dropped failures.
- Same immutable inputs and deterministic mode produce the same canonical receipt digest. Any numeric tolerance is field-specific and versioned.
- Invalid or unsupported tasks return explicit indeterminate/incomplete/reject, never a fabricated score or implied pass.

### Research / model targets

- Baseline ladder: beat persistence/constant-velocity and statistical baselines before claiming value over them; reproduce the strongest same-protocol public baseline before claiming SOTA.
- Cross-taxon transfer: define leave-one-taxon-out (and where possible leave-one-family/order-out) tests; report macro performance and worst-stratum gap, not only pooled performance.
- Calibration: report 50/80/90/95% predictive interval coverage and proper probabilistic scores for stochastic outputs. A nominal 90% interval should land near its advertised coverage on sufficient independent trials; preregister acceptable tolerance and interval method.
- Intervention tests: at least 20 preregistered intervention/control pairs in the initial scientific suite when data and power allow; report treatment-effect error and sign accuracy, null-control false responses, sample-size limitations and confidence intervals. This is a development target, not a guarantee that 20 trials are scientifically sufficient.
- Non-inferiority: improvements may not hide material regressions on rare taxa, life stages, occlusion, missing modalities or constrained compute. Set per-task non-inferiority margins before scoring.
- Efficiency: report accuracy-quality-cost curves (latency, memory, energy when instrumented) at fixed output/fidelity settings. Do not trade away causal/measurement correctness to produce a faster demo.

### Statistical reporting

- Freeze the primary metric, secondary metrics, split, subgroup analyses, sample exclusions and stopping rule before opening the final test set.
- Group bootstrap or other uncertainty estimation at the independent unit (often animal, recording, study or site), not individual correlated frames.
- Report sample count, missingness, censoring, failed/abstained predictions and confidence intervals per taxon/stage/condition.
- Maintain separate development, calibration and final test data. If results are used for parameter selection, they are not final test results.
- No baseline should receive extra training data or tuning access without disclosure. Report parameter count, data overlap, compute budget, input resolution, inference settings and source revision.
- Do not choose a baseline after seeing the final test result and then call the comparison preregistered.

## Replay-result receipt contract

A future typed result record should bind at minimum:

- schema version, stable run ID, task ID/version and result status (eligible, indeterminate, reject, incomplete);
- profile ID/revision, taxon ID and taxonomy source/release, life stage and declared transfer envelope;
- observation pack manifest digest, allowed-input/snapshot digest, split manifest digest and source permission/attribution receipt references;
- simulator code HEAD, dirty-tree state where applicable, build/toolchain/runtime/platform, model/controller revision, configuration digest, random seeds and determinism policy;
- baseline IDs/revisions and comparable data/training/compute allowances;
- exact metric definitions, units, sample/stratum counts, missingness, estimate, uncertainty interval, abstentions and failure reasons;
- primary metric and preregistered decision threshold, claim ceiling, known limitations and references;
- canonical receipt digest and independently reproducible evaluator version.

The receipt should never store private raw wildlife coordinates just to make a result reproducible. Bind a permitted source/snapshot reference or restricted digest according to data rights and privacy policy. Access-controlled evidence can be referenced without redistributing it.

## Initial execution sequence

1. Preserve current exact-head CI and synthetic oracles as separate evidence; queued checks remain non-passing.
2. Implement typed task and metric definitions plus a hostile synthetic replay-result oracle.
3. Implement canonical replay receipt creation and compare-two-runs reporting, including order-invariance and missing-data controls.
4. Add baseline adapters in metadata/manifest mode first; do not fetch payloads until exact rights/release reviews pass.
5. Reproduce one visual task and one narrow sensorimotor/kinematic task in separate scorecards.
6. Only then run held-out counterfactuals, transfer tests and broader ecology scenarios.

## Non-claims

This contract does not assert that the current animal-simulation system beats any published baseline, accurately models every taxon, has calibrated biological parameters, predicts animal consciousness or motivation, or is safe for veterinary, husbandry, conservation or animal-welfare decisions. Those require task-specific held-out empirical evidence and appropriate expert review.
