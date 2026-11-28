# Spatial Reliability Scorecard v0

**Status:** research protocol proposal; not an implementation or measured result  
**Scope:** common reporting semantics for Symtropy spatial-state, trajectory, dynamic-map and active-acquisition evaluations  
**Owners composed:** #7693 (projection evaluator), #7787 (dynamic map persistence), #6675 (sequential acquisition), #6754 (blind custody), #6184 (scenario registry)  
**Non-goal:** this document does not add a simulator, map store, tracker, scheduler, oracle, custody runner or production model.

## 1. Evaluation principle

A spatial system is not qualified by low geometric error conditional on a small surviving subset. It is qualified by the joint outcome:

1. it receives only admissible evidence available at the decision time;
2. it maintains a spatial state whose claims and uncertainty are supported by that evidence;
3. it preserves unresolved alternatives when observations do not identify a unique world;
4. it obtains additional evidence only when permitted and decision-relevant;
5. it makes a correct, calibrated decision or abstains when the evidence is insufficient; and
6. it accounts for every assigned sequence, trial, query and failure.

Execution disposition, individual acquisition outcomes and answer disposition MUST be separate fields. A correct abstention is not a crash; a denied probe is not automatically a failed trial if the system safely selects an admissible alternative or abstains. Conversely, an always-abstain policy MUST be penalized through false-abstention cost and answer coverage. A completed process with a correct abstention may still have a successful geometric trajectory; task-level answer/decision success is a separate score plane.

Every report MUST include both the benchmark's native metric and a failure-inclusive companion scorecard. Native metrics are retained for comparability; the companion scorecard prevents selective survival from being mistaken for system performance.

## 2. Units, denominators and run identity

Every evaluation manifest MUST identify:

- benchmark and protocol revision;
- exact scenario, world-pair, source-scene, session and episode IDs;
- candidate source/model/configuration digests;
- sensor profile, calibration revision and modality set;
- observation rate/resolution, action and memory budgets;
- hardware/runtime profile and measured compute/energy scope;
- expected attempts, expected frames or measurement opportunities;
- split and cluster keys; and
- all terminal dispositions, including timeout, crash, invalid output, tracking loss, permission denial, abstention and not-evaluated.

The statistical unit MUST match the randomization and dependence structure. Frames from one sequence are not independent trials. Repeated views of one world, multiple questions from one episode, and paired A/B renderings share upstream causes and MUST remain in the same resampling cluster.

The report MUST publish raw denominators for every metric. It MUST NOT silently change the denominator between methods or strata.

## 3. Trajectory and localization reporting

For each sequence or trial, define the benchmark's expected pose opportunities before execution. Let:

- N_expected be all expected pose opportunities;
- N_valid be opportunities with a valid, on-time, correctly frame-bound pose;
- coverage = N_valid / N_expected;
- ATE_conditional be the benchmark-native pose error computed only on valid aligned poses;
- failure indicate crash, timeout or terminal tracking failure.

Report coverage and ATE_conditional separately. An ATE value without its corresponding coverage and full attempt denominator is incomplete.

Also report a sequence-level joint success indicator:

**joint_success = not failure AND coverage >= C_min AND ATE_conditional <= E_max**

C_min and E_max MUST be selected and preregistered for the benchmark/task, not tuned after observing candidate results. The primary system-success rate is the number of joint successes divided by all assigned sequences/trials, including failures and timeouts. If no valid trajectory exists, ATE is NOT_EVALUATED, not zero and not omitted from the attempt ledger.

Where the native benchmark defines different failure semantics, publish them unchanged alongside this companion scorecard. In particular, do not claim that every metric of a benchmark excludes failed runs: CROSS T1 has a completeness gate below which ATE is excluded from the mean, while its T2/T3 localization metrics explicitly count absent poses or failed trials as failures. Match the exact task and native protocol.

Minimum trajectory table, per method and stratum:

| Field | Required interpretation |
|---|---|
| Assigned trials | All trials allocated before execution |
| Completed trials | Trials with terminal output |
| Crash / timeout / invalid | Separate counts, never silently dropped |
| Pose coverage | Valid on-time poses / expected pose opportunities |
| Tracking-loss time | Total and longest continuous loss |
| Conditional ATE / RPE | Native error on the declared valid support |
| Joint success | Coverage and error thresholds both met |
| Relocalization | Recovery rate and time-to-recovery |
| Map support | Fraction of task area/route with valid map support |

## 4. Map, topology and claim reliability

Every claim opportunity MUST be scored against evaluator truth or an explicitly marked NOT_EVALUATED condition. Report by claim type and observability stratum:

- false unique-map claims when multiple worlds remain observationally equivalent;
- false or unsupported adjacency, connectivity, scale, orientation and frame-transform claims;
- false loop closures and entity merges/splits;
- correct preservation of Unknown, Ambiguous, Inferred, Remembered, Predicted, Simulated and Counterfactual status;
- correct correction, retraction and bitemporal preservation after a distinguishing observation;
- evidence lineage validity, calibration/clock-transform currentness and common-cause independence; and
- query-specific decision-partition agreement and worst-case decision regret.

A claim is not correct merely because its geometry is close: it must also have admissible evidence, valid coordinate/time semantics and an appropriate uncertainty/status. Derived frames, inverse transforms, repeated optimizer residuals, duplicate captures and multiple outputs from one common cause MUST NOT be counted as independent evidence.

For each decision query, report:
- **answer coverage:** proportion of admissible query opportunities receiving a non-abstained answer;
- **selective risk:** error among answered opportunities;
- **correct abstention:** abstentions on cases whose evidence does not support a safe/unique answer;
- **false abstention:** abstentions where the preregistered evidence suffices;
- **unsupported-claim rate:** claims lacking a valid source/transform/uncertainty chain;
- **worst-case regret:** maximum evaluator-computed decision loss over admissible hidden worlds.

Publish risk-versus-coverage curves and per-stratum values. Do not collapse epistemic correctness into geometric accuracy.

## 5. Active acquisition and memory

Compare policies at identical episode sets, initial evidence, permitted action catalogue, action/observation budgets, deadlines, memory budget and hardware envelope.

Required policy lanes:

1. no additional observation / immediate decision;
2. fixed-rate or uniform sampling;
3. same-step random walk with full-history retention (ActSpace-Bench RW-40 or the benchmark's exact equivalent);
4. same-step random walk with the candidate-matched rolling evidence window (ActSpace-Bench RW-8 or the exact same retained-view budget);
5. passive common-evidence reasoning control, where the candidate receives the same fixed evidence set without choosing it;
6. benchmark-native strongest active baseline;
7. candidate active policy; and
8. exact oracle route/policy as an evaluator-only diagnostic upper bound.

The two random controls answer different questions. Full-history random measures the combined value of exploration plus retaining more evidence; matched-window random isolates purposeful exploration under the same memory constraint as the active candidate. Do not report only the full-history random comparison as an active-perception result, and do not report only the matched-window comparison as a memory result. The oracle is not a model competitor and MUST NOT be included in the headline model ranking.

Report task decision loss, policy regret, action count/cost, observations actually delivered, bytes/frames/tokens, time-to-sufficient-evidence, deadline misses, memory retained/evicted, stale-certificate reuse, and reopening cost when a later DecisionNeed invalidates an earlier sufficiency certificate. Also report separately: (a) evidence found by each policy, (b) evidence still retained and available at answer time, (c) evidence retention loss after eviction/truncation, and (d) reasoning accuracy when every method receives the same evidence. This separates finding, remembering and reasoning rather than assigning every failure to exploration.

A request is not an acquired observation. A requested observation that is denied, dropped, stale or late remains a failed/undelivered action in the accounting. A retrieved historical frame is memory evidence, not a new measurement. No state update may be attributed to evidence before its availability tick.

### Episodic instance-memory lane: EgoWitness / EgoClue

Add a distinct long-horizon instance-memory cohort rather than pooling it with map construction or geometric SLAM. [EgoWitness (19 November 2026)](https://arxiv.org/abs/2611.25849) contains 8,453 human-verified questions over approximately 324 hours, covering seven abilities such as instance identity, latest-witness attribution, object history, room identity and visit reasoning. Its week-long split is task-group disjoint; all 787 short-horizon questions and their environments are reserved for cross-environment evaluation.

Keep at least three reference conditions separate:
- direct VLMs using the benchmark's shared timestamped-frame input condition;
- tool-using controllers with the paper's separate perception/retrieval backends and declared decision/tool-call limits;
- evidence-trace synthesis from already gathered traces, with no new visual calls.

The paper reports direct VLM performance substantially below its trained EgoClue system, but EgoClue's result is an agent-stack result (Qwen3.5-4B controller, Gemini visual backend, GPT retrieval backend, SFT+GRPO), not a parameter-matched direct-VLM result. The paper's shortest-path query type uses a supplied room graph, so it is not a map-reconstruction or SLAM score.

Report latest-witness accuracy, wearer attribution, instance merge/split, missed positive sighting windows, room identity, visit-count/duration and evidence-linked correctness separately. Cluster by source video / wearer / task group. Verify source-media rights, observer dependencies and exact tool limits before ingestion or comparison; the preprint does not itself establish commercial reuse rights for all underlying recordings.

External protocol note: ActSpace-Bench (arXiv:2611.31215, 23 November 2026) compares a model's own exploration against both a same-step full-history random walk and a same-step random walk constrained to the same recent-view window. Its strongest models show much larger gains over the matched-window control than over full-history random, because observations discovered earlier may be evicted before the answer. Our adapters MUST preserve this two-control design where applicable; it is not fair to infer active-perception superiority from an unmatched memory condition. ActSpace's 40-step budget and start-plus-eight-recent-frame window are native to that benchmark; other benchmarks should preregister their own matched budget rather than blindly copy these values.

## 6. Dynamic-scene persistence

Keep three layers distinct:

1. **persistent structure:** walls, fixed openings, room topology and verified structural changes;
2. **transient occupancy:** people, carts and movable objects with time-bounded presence; and
3. **task-relative traversability:** current route feasibility under occupancy, permissions and operating conditions.

Report false static additions/deletions, transient-to-static promotion, static-to-transient demotion, dynamic-track continuity, map-change precision/recall, time-to-detect a real structural change, and historical-state preservation. Report trajectory coverage, relocalization and ATE/RPE on the same dynamic-density strata; never improve apparent map stability by losing tracking.

For THUD++-style density comparisons, publish each crowd/density stratum independently and keep real RGB-D observations, synthetic Unity annotations and emulator outcomes as distinct evidence cohorts. A simulator-exported camera pose is simulator truth, not independent physical ground truth.

## 7. Statistical comparison

The primary comparison MUST be paired on identical scenario/world/session assignments wherever the protocol permits. Use scene/session/world-pair/episode clusters as the resampling unit, not individual frames or repeated questions.

Minimum reporting:

- effect size in native units and absolute percentage points where applicable;
- confidence interval with the cluster method and cluster count stated;
- per-stratum effects and denominators;
- predeclared primary metric and multiplicity handling;
- model/configuration and attempt ledger;
- power/sample-size plan for confirmatory claims; and
- all exclusions with a reason fixed independently of candidate outcome.

Use cluster bootstrap or a design-appropriate hierarchical method when enough independent clusters exist. With too few clusters, mark inference UNDERPOWERED or NOT_EVALUATED; do not substitute a frame-level confidence interval. For equivalence, use preregistered margins and an equivalence procedure (e.g. TOST for scalar effects); a non-significant difference test is not evidence of equivalence.

Do not use one unqualified weighted average as the leaderboard. Publish the metric vector and Pareto frontier, with any weighting policy preregistered and justified by the task.

## 8. Fairness and SOTA comparison rules

Every external comparison MUST match or explicitly stratify:

- sensor modalities and their exact inputs;
- calibration assumptions and ground-truth source;
- frame rate, resolution and temporal support;
- map availability, initialization and prior knowledge;
- action and memory budgets;
- training-data rights and train/test scene separation;
- runtime, hardware and resource caps; and
- failure/abstention semantics.

Keep camera-only, stereo, RGB-D, odometry, IMU and LiDAR lanes separate. A system using stereo plus odometry MUST NOT be ranked as a camera-only method. Reproduce the official evaluator and at least one strongest compatible baseline before claiming improvement. A public leaderboard number is a reference, not proof of a matched comparison.

## 9. Qualification ladder

- **Q0 — integrity:** exact manifests, attempt ledger, sensor profile, rights/split evidence and negative leakage controls pass.
- **Q1 — protocol reproduction:** reproduce the official baseline/protocol on a rights-qualified, sensor-matched subset within a preregistered tolerance.
- **Q2 — capability improvement:** improve a preregistered primary metric versus the strongest matched baseline without violating hard integrity, authority or reliability gates.
- **Q3 — system superiority:** show non-inferiority on coverage, calibration, failure rate, uncertainty, latency and resource cost while improving the primary task outcome.
- **Q4 — independent confirmation:** frozen candidate, sealed held-out scenes, independent evaluator/custody, complete attempt ledger and reproducible receipts.

A failure at any hard gate returns NOT_QUALIFIED or NOT_EVALUATED; it is not converted into a lower task score. No CI pass, renderer success, candidate performance or SOTA result is implied by this protocol document.

## 10. Machine-readable attempt ledger

The companion [spatial-attempt-ledger-v0.schema.json](./spatial-attempt-ledger-v0.schema.json) and [example](./spatial-attempt-ledger-v0.example.json) define a one-record-per-attempt contract. Each attempt preserves scene/session/world-pair/episode cluster identity, terminal disposition, expected and valid pose opportunities, coverage, conditional ATE/RPE or explicit NOT_EVALUATED, and a joint-success flag. The schema is checked by [.github/workflows/spatial-attempt-ledger.yml](../../.github/workflows/spatial-attempt-ledger.yml) against the repository-pinned JSON Schema validator, including negative controls for zero-pose ATE, missing NOT_EVALUATED reason, failure-as-success and hidden evaluator fields.

The schema separates three types of outcome: `terminal_disposition` (execution completed/crashed/timed out/tracking-lost/invalid/not-evaluated), `acquisition_dispositions` (delivered/dropped/stale/late/permission-denied/duplicate/reordered actions), and `answer_disposition` (answered/abstained/deferred/no-response/not-applicable). Abstention is not a terminal execution failure, and permission denial is a per-action outcome. The example includes a completed, high-coverage trajectory with an abstained answer after a denied optional probe to test this separation.

Run identity also binds the scoring-code digest, reference profile/dataset digest, reference-source class and independence class, plus the exact qualification-criteria record and digest. The criteria record includes minimum pose coverage and maximum conditional ATE/RPE. The example deliberately marks thresholds `development_only`; it is not a preregistration or qualification result. A confirmatory attempt must bind criteria fixed before candidate results are examined.

The schema does not recompute aggregate statistics or prove that a candidate's reported coverage and metrics are truthful. The evaluator MUST recompute counts and metrics from source attempt receipts, compare valid counts with expected opportunities, verify metric/frame identity and ground-truth provenance, and derive geometric joint success using the bound criteria. Task-level decision correctness, correct/false abstention and selective risk/coverage remain independently scored. The example is illustrative only and contains no candidate performance result.

## 11. Composition

- #6184 owns scenario matrices and candidate-visible contracts.
- #6201 owns typed observations and causal replay.
- #6754 owns AV custody, sealed truth and split qualification.
- #7693 owns candidate spatial projection validation and map/geometry/topology scores.
- #7787 owns dynamic persistent-versus-transient map challenges.
- #6675 owns sequential acquisition policy tournaments.
- #7577 owns evaluator-side decision-sufficiency checking.
- #7902 / #7904 own observability-conditioned claim reliability, calibration and paired negative controls.

This scorecard defines common reporting semantics only. It MUST be implemented by adapters in the owning evaluators rather than by adding a parallel scoring engine.
