# Animal Replay Evaluation V0

Status: draft evaluation contract  
Research snapshot: 2026-10-19  
Tracks: [ANIMAL-REPLAY-001](https://github.com/Luminous-Dynamics/symtropy/issues/2149), [ANIMAL-OBSERVATION-001](https://github.com/Luminous-Dynamics/symtropy/issues/2128)

## Purpose

Define how Symtropy/Symthaea can demonstrate measurable progress toward useful, causally grounded animal simulation and animal-kingdom coverage. This contract connects capability profiles and rights-bound observation manifests to task-specific replay results. It does **not** claim that these results or the existing synthetic oracles validate biological realism.

The central shift is from *one plausible-looking animal* to **a falsifiable, calibrated model that predicts held-out observations and the direction and magnitude of responses to controlled interventions, within a declared taxon/stage/condition envelope**.

A result must answer: which capability, measured by what observable, on which independent holdout, against which baselines, with what uncertainty, at what compute cost, and under what claim ceiling?

## Integration with the existing Living World Observatory

This contract **specializes** the shared Observatory; it does not define a competing qualification ladder. Reuse [the Living World Observatory and causal-realism contract stack (PR #245)](https://github.com/Luminous-Dynamics/symtropy/pull/245), [the non-omniscient fauna signal-perception oracle (PR #246)](https://github.com/Luminous-Dynamics/symtropy/pull/246), and [ANIMA's epistemic-state contract (PR #1190)](https://github.com/Luminous-Dynamics/symtropy/pull/1190). Preserve the existing distinction between world truth, emitted signals, raw evidence, percept, fallible interpretation/memory, decision and action. An animal agent must not query hidden exact world state as if it had directly sensed it.

The shared Observatory ladder remains authoritative:

- Q0: static contract and schema evidence.
- Q1: exact deterministic execution.
- Q2: differential and invariance evidence (for example, headless/rendered or fidelity-path comparisons).
- Q3: ecological and causal behavior under controlled interventions.
- Q4: presentation causal legibility.
- Q5: human validation/review.

The requirements added here are task-specific observables, comparison protocols, statistics, baseline targets and replay-result receipts within those shared levels. Source rights, measurement quality, taxonomy identity, split independence and transfer envelope are **orthogonal prerequisites**, not new Observatory levels. A pose score does not qualify ecological causality; a Q1 replay does not qualify a real-world model; a Q3 causal pass does not establish presentation quality or human-judged realism. Cross-taxon transfer is a separate empirical claim that must pass a target-specific, held-out evaluation; it is not granted by a broader stage label.

For the integrated target, use the existing deterministic Living World Cell and causal-history intervention cases where possible. Do not create an animal-only parallel world or duplicate shared ecology authority merely to run a benchmark.

## Paradigm shifts

1. **From species count to capability coverage.** Count represented body plans, sensory modalities, locomotion strategies, life stages, and ecological roles. A long species list is not coverage if every profile is an unvalidated copy of one quadruped.
2. **From visual plausibility to observable agreement.** A video can support tracking or pose metrics; it cannot by itself establish force, metabolism, internal physiology, motivation, or cognition.
3. **From next-frame prediction to intervention prediction.** Evaluate whether the model predicts what changes when a sensory cue, terrain, food availability, temperature, injury proxy, social partner, or energy constraint is deliberately changed. Observational correlation alone cannot establish causal response.
4. **From a single deterministic trajectory to calibrated ensembles.** Animals are stochastic and contexts are incompletely observed. Preserve multimodality, predictive intervals, abstention, and uncertainty rather than selecting one plausible trajectory.
5. **From random splits to explicit transfer claims.** Separate familiar-frame interpolation, unseen-individual, unseen-study/site, future-time, unseen-taxon, and cross-stage evaluations. Never label one as another.
6. **From pooled averages to a per-taxon evidence ledger.** Report counts, uncertainty, missingness, worst-performing strata, and failures. Macro-average taxa so abundant/easy datasets cannot hide poor performance on rare body plans.
7. **From render-bound behavior to presentation-independent dynamics.** The same physical state and inputs should have the same causal outcomes with rendering on/off and across supported fidelity settings, within explicitly declared numerical tolerance.
8. **From leaderboard chasing to reproducible scientific claims.** Pin data manifest, rights receipt, split, taxonomy release, code HEAD, model/profile, config, seeds, hardware/runtime and evaluator. A queued check or mutable benchmark URL is not a result.

## Task-specific evidence gates

Keep these statuses distinct in the UI, reports and APIs, but map them onto the shared Observatory ladder above rather than assigning them new Q-levels.

- **Contract/oracle status:** typed task, metric, profile, reference, split and result structures validate; frozen adversarial cases agree with independently authored expected dispositions.
- **Source eligibility:** exact source revision/artifact, allowed operation, attribution, measurement semantics, taxonomy identity, identity grouping and sensitive-location handling are reviewed. This is an input prerequisite, not model validation.
- **Replay reproduced:** the same immutable inputs recreate the result receipt, with declared numeric tolerances where exact floating-point equality is not portable. This supports reproducibility, not empirical truth.
- **Reference task qualified:** a named model passes a narrow real-reference task on a frozen holdout against stated baselines. Report the task's Observatory evidence level and claim ceiling.
- **Scientific/counterfactual qualification:** held-out experimental conditions and preregistered interventions support an explicit causal claim; this belongs at Observatory Q3, with the experiment and uncertainty bound in the receipt.
- **Transfer qualified:** a separate target taxon, stage, site or condition passes its own predeclared holdout. Transfer is a task-specific result property, not an automatic level or permission to generalize.

Synthetic oracles can prove that the evaluator rejects bad evidence; they cannot prove an animal model is true. Rights review, deterministic replay, ecological causality, presentation legibility and human validation remain different forms of evidence.

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

### Behavior classification and learned representations

These references cover behavior labels and interaction-aware representations—different from pose/tracking, and still narrower than full biological simulation.

**Animal Kingdom** ([official project](https://github.com/SUTDCV/Animal-Kingdom), [CVPR 2022 paper](https://arxiv.org/abs/2204.08129)) reports 50 hours of video-grounding annotations, around 30,000 fine-grained multi-label action-recognition sequences, around 33,000 pose frames and coverage across more than 850 animal species in six major animal classes. Its source includes tasks for action recognition, pose and temporal video grounding. Exact downloadable artifacts, source-video rights and downstream training/evaluation permissions must be reviewed before payload use; the repository link is not itself a universal data license.

**BEBE (Bio-logger Ethogram Benchmark)** ([paper](https://pmc.ncbi.nlm.nih.gov/articles/PMC11654173/)) combines behaviorally annotated bio-logger datasets totaling approximately 1,654 hours from 149 individuals across nine taxa. It is a candidate for testing whether simulation/sensor models predict declared ethogram labels from measured kinematic/environmental channels and generalize to new individuals. Its independent-unit split is individual-based. Do not treat sensor-derived labels as proof of internal motivation or cognition, and separately review each contributing dataset’s rights.

**MABe22** ([PMLR paper](https://proceedings.mlr.press/v202/sun23g.html)) tests multi-species behavior representations using interacting mice, beetle-ant interactions and groups of flies. The paper reports millions of video/pose frames and downstream tasks tied to experimental conditions and behavior. Use it to test whether representations retain interaction-relevant information and whether models trained on human-action data transfer to animal behavior; it does not qualify a mechanistic simulator by itself. Verify the exact artifact revision, data terms and permitted operation before ingestion.

**DISK (Deep Imputation for Skeleton Data)** ([Nature Methods paper](https://www.nature.com/articles/s41592-025-02893-y), [official code](https://github.com/bozeklab/DISK)) provides a targeted gap-reconstruction reference across seven skeleton datasets, with single-/multi-animal and 2D/3D cases. The paper evaluates RMSE, MPJPE and PCK@0.01, compares against linear interpolation and other imputation baselines, and reports uncertainty-oriented imputation variants. Use it to test **missing-data recovery separately from pose estimation**: randomly injected gaps are not a substitute for genuine missingness; hold out gap lengths, keypoints, speed regimes, and keypoint-identity switches, and measure downstream behavioral-statistic distortion as well as reconstruction error. The code repository's software license does not automatically license its underlying seven datasets; review each dataset's provenance and permission before use.

**Targets for Symtropy:** first reproduce one source-specific task and its published baseline, then evaluate leave-one-individual-out and (when source identity permits) leave-one-study/site-out splits. Report class-wise F1/precision/recall, calibration, temporal boundary error and macro-average by taxon/action class; for interaction representations, report task-specific downstream performance and subgroup gaps. Use the paired comparison receipt from the implementation PR, with the individual/episode as the independent unit—not correlated frames. No cross-taxon transfer claim should follow from an ordinary random split.

### Proposed benchmark execution matrix

This matrix avoids treating “animal intelligence” as one leaderboard. First reproduce one task exactly; then extend difficulty while preserving the source's official metrics and split rules.

| Benchmark family | Reference task | Primary evaluation | Baselines to reproduce | Harder-than-random-split test |
|---|---|---|---|---|
| Pose / identity tracking | AP-10K or APT-36K pinned split | Source-defined AP/PCK/HOTA and ID-switch metrics as applicable | Published reference; simple visual/temporal baseline | Species/category-disjoint and long-occlusion clips; report each taxon and identity switch |
| Pose-gap recovery | DISK pinned release and dataset with verified rights | RMSE, MPJPE, PCK@0.01 plus downstream behavior-statistic distortion | Linear interpolation, same-input GRU, published DISK reference where reimplementation is feasible | Held-out gap length/keypoints/speed; keypoint swaps; compare genuine and synthetic gaps separately |
| Video behavior | One Animal Kingdom action-recognition or temporal grounding task | Source metric: multi-label mAP/F1 or temporal localization metric, never interchangeable | Published method plus class-prior and simple temporal baseline | Held-out scene, animal identity, viewpoint or context where annotations support it |
| Bio-logger ethogram | One BEBE dataset and permitted operation | Official task metrics, class-wise precision/recall/F1 and per-individual scores | Published reference and label-prior / simple feature baseline | Leave-one-individual-out; source/study transfer only when there is a defensible mapping |
| Multi-agent representations | One MABe22 downstream task | Reproduce that task's official classification/regression metric and its split | Official evaluator/reference; simple linear probe on the same features | Held-out recording/animal/context; report mice, beetle-ant and fly tasks separately |
| Movement forecasting | One rights-reviewed Movebank study | Predeclared horizon-specific location error plus probabilistic score when available | Persistence/constant-velocity plus published baseline | Future-time, held-out individual/site/study; prevent neighboring windows from crossing splits |
| Active sensing | Synthetic known-answer scenes first | Task utility minus explicit sensing/movement cost, false-percept rate, regret to finite oracle | Passive, periodic, event-triggered, EVSI and resource-aware EVSI | Held-out emitter/noise/occlusion regime with identical sensor budgets |
| Counterfactual sensorimotor response | Controlled intervention task | Signed response effect, calibration and task cost | No-change/control policy and matched baseline | Held-out intervention combinations with expected null effects preserved |

For supervised behavior tasks, use class-balanced macro-F1 and per-class precision/recall in addition to the benchmark's official metric when class imbalance is material. For regression tasks, preserve the official error/score and uncertainty; never substitute F1. Report both per-independent-unit results and task-level aggregation. A cross-task summary must first normalize each official metric using a predeclared rule, show all component tasks, and may not conceal a failed hard gate behind an average.

### Release gates for a credible SOTA claim

1. **Protocol reproduction:** pin upstream version, labels, code/evaluator, split file and permissions; reproduce the published baseline to a predeclared tolerance before evaluating the candidate.
2. **Leakage audit:** verify no animal/individual/clip/scene or near-duplicate spans train and test contrary to the target generalization claim. Group by the real independent unit, not frames.
3. **Paired evaluation:** run candidate and baseline on exactly the same held-out units; pin uncertainty method and practical effect threshold in advance.
4. **Ablation:** remove one modality, temporal history or interaction cue at a time. A claimed mechanism should degrade in its predicted direction; otherwise weaken the claim.
5. **Robustness:** test noise, occlusion, missing sensors, label uncertainty and at least one held-out condition. Publish failures and subgroup gaps.
6. **Cost frontier:** report compute, latency, memory, training-data volume and sensing/action budget beside quality. Only compare “better” under a declared matched budget or as an explicit Pareto trade-off.
7. **Reproduction package:** export a rights-checked, immutable manifest and receipt, metric-by-stratum report, exact run identity, seeds, and negative-control results.

A proposed initial win threshold is a preregistered practically meaningful paired improvement whose confidence interval excludes zero and whose estimate clears the practical margin, with no protected key stratum beyond its predeclared non-inferiority margin. A universal “+3 points” target is not automatically appropriate across AP, F1, HOTA, forecast error and calibrated decision utility; margins must be chosen per task before seeing holdout scores.

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

A metric is keyed by its metric definition plus a stable stratum kind and identifier. The same metric may appear for the overall cohort, each taxon, life stage, site/study, occlusion band or condition; duplicate keys for the same metric/stratum are rejected. Keep per-stratum sample counts and uncertainty visible rather than averaging rare taxa out of the report.

A preregistered threshold is met only when the declared uncertainty interval clears it in the required direction. If the interval overlaps the threshold, record `uncertain`; if no suitable interval exists, the threshold is `not_assessed`, not passed. A point estimate on the favorable side is not sufficient to call a threshold met.

A leaderboard delta alone is not a superiority claim. For each claimed model-vs-baseline result, use a paired comparison at the declared independent unit (for example animal, recording or study), with a pinned paired-effect estimator, confidence level, unit count, signed candidate-minus-baseline difference, minimum meaningful effect, and non-inferiority margin. Require the paired mean difference to agree with the candidate/baseline means when both are available; otherwise mark the comparison not assessed or incomplete. Report `outperforms_by_margin`, `non_inferior`, `regresses_beyond_margin`, `uncertain`, `not_assessed`, or `incomplete` separately from the absolute-threshold decision. A higher-is-better vs lower-is-better flag must be explicit; direction is never inferred from a metric name.

Every paired comparison must bind an exact canonical digest of paired-unit membership. A baseline row must include its own run ID, immutable code HEAD and config digest, evaluator/toolchain/platform, revision, training-data scope and compute budget. A free-form model name or leaderboard label is insufficient provenance. The task must state whether it is a matched-budget comparison or an explicit cost/performance trade-off. Matched-budget mode rejects mismatched declared training-data scope or compute budget; a trade-off may be reported, but must not be described as an equal-budget SOTA comparison.

The paired-unit manifest digest should not expose sensitive animal IDs or wildlife locations: the permitted evaluator may bind an access-controlled manifest by digest without copying it into a public receipt. Record the independent unit and manifest revision alongside the digest so another qualified evaluator can reproduce membership decisions under the same permissions.

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
