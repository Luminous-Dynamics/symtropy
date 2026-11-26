# Spatial state construction protocol v0

**Status:** benchmark comparison protocol only. It does not implement a spatial map or qualify any model.

**Motivation:** the current official OVO-S-Bench analysis finds allocentric mapping is the dominant bottleneck, while evidence recall is near-uncorrelated with correctness for tested memory systems and oracle evidence selection / doubled frame budgets yield at most +0.3 points over uniform 128-frame sampling. The experiment must isolate state construction from evidence retrieval.

Primary source: https://internlm.github.io/OVO-S-Bench/

## 1. Experimental question

Given the same causal observation prefix and downstream query, can a candidate construct a geometrically/topologically consistent allocentric world state whose claims are grounded in source observations, calibrated for uncertainty, revisable under contradiction and useful for downstream decisions?

The central isolation is:

> When every candidate receives the same admissible evidence spans, how much of the remaining error is caused by map construction, identity association, coordinate reasoning, topology, or decision use?

Do not assume that more retrieved frames, a larger memory, a spatial fine-tune or chain-of-thought improves these capabilities.

## 2. Matched evidence-access lanes

Run all lanes on identical frozen query IDs, prefix cutoffs, source videos, task definitions and evaluation code.

- **L0 — Native causal stream:** each candidate's own admitted sampling policy, under a fixed frame/compute/latency budget. This is the deployment-relevant end-to-end lane.
- **L1 — Common sampled evidence:** every candidate receives the same fixed-rate or uniform-128-frame sample from the permitted prefix. This controls input availability.
- **L2 — Oracle evidence spans:** every candidate receives the same evaluator-selected annotated evidence intervals, but never future frames, answer labels, hidden map IDs or world coordinates. This is a diagnostic upper bound on evidence selection, not a deployable score.
- **L3 — Common map-state handoff:** each candidate emits a versioned spatial-state projection consumed by one frozen downstream evaluator for map consistency, topology and DecisionNeed-specific action.

L0 is the native OVO-S comparison. L1–L3 are separate diagnostic score planes. Never average them into the native leaderboard result. Ensure that L2's oracle spans do not disclose answers through interval IDs, filenames, ordering or hidden metadata.

## 3. Candidate architecture arms

Use matched backbones and fixed downstream interfaces wherever possible:

- General-purpose video MLLM with native prefix input.
- Same MLLM with common sampled evidence.
- Same MLLM with oracle evidence spans.
- Explicit geometry/SLAM/topology pipeline where pose and calibration are actually available.
- Persistent object-centric belief state using existing Vision Mind owners.
- Dense visual features plus explicit geometric/topological state.
- Patch-HDC/HDC features plus explicit geometric/topological state.
- CfC/LTC temporal state plus explicit geometric/topological state.
- Hybrid general backbone plus evidence-indexed entity/geometry/topology state.
- Hybrid plus bounded decision-sufficient acquisition.
- Lesions: no map, no geometry, no persistence, no loop closure, no active acquisition, shuffled evidence, removed provenance, stale pose.

The current OVO-S evidence that 13/15 specializations regress against their base means every adaptation needs a paired matched-backbone comparison. Do not admit specializations on narrative plausibility.

## 4. Spatial-state projection

Candidate-visible state records must preserve:

- candidate-local entity/landmark hypothesis IDs;
- semantic and geometric claim types;
- image/camera/world coordinate frame identity;
- camera pose and transform revision with uncertainty;
- point/region/trajectory or set-valued alternative;
- topological relations with explicit support status: directly observed, inferred, remembered, predicted, simulated, counterfactual or unknown;
- exact source observation IDs and half-open source-time intervals;
- measurement/capture time and arrival/known-at time separately;
- source lineage, transform parents and digests;
- lifecycle / supersession / correction references;
- competing identity/map hypotheses and unresolved residuals.

No candidate-visible record contains evaluator world IDs, canonical witness IDs, hidden map coordinates, truth labels or future observations. Image-plane coordinates must never be silently promoted to world coordinates.

## 5. Scorecard

Report by OVO-S level, task family, source video cluster, evidence-span horizon, viewpoint change, calibration status, occlusion and OOD stratum.

### 5.1 Grounding

- claim-to-source-span precision and recall;
- unsupported visual/geometric claim rate;
- epistemic-origin confusion;
- source interval and transform lineage completeness;
- future-frame/hidden-state leakage.

### 5.2 Entity/landmark state

- identity association precision/recall;
- false merge and false split rates;
- re-identification after occlusion and viewpoint change;
- temporal state-transition correctness;
- false persistence.

### 5.3 Geometry

- pose/landmark error only where calibrated truth exists;
- trajectory ATE/RPE only when estimator outputs satisfy the metric contract;
- camera/world transform consistency;
- uncertainty interval coverage and calibration;
- drift / extrapolation failure by horizon.

### 5.4 Topology

- adjacency/containment/connectivity precision and recall by relation type;
- false adjacency and missed connectivity;
- route/trajectory-to-map consistency;
- false loop closure;
- same-endpoint/different-path discrimination;
- correct Unknown when continuity is unobservable.

### 5.5 Decision quality

- native OVO-S accuracy and per-level score;
- evaluator decision-partition agreement;
- DecisionNeed-scoped sufficiency validity;
- absolute decision regret and compression regret gap;
- correct stop / probe / wait / abstain disposition;
- false-safe and unsupported action rates on deterministic fixtures.

### 5.6 Cost

- frames, audio samples, bytes, tokens and model calls;
- persistent state and provenance sidecar bytes;
- map-update, correction, replay and verification cost;
- p50/p95/p99 latency;
- measured CPU/GPU/VRAM and energy when available.

Do not combine these into one scalar. Publish separate Pareto frontiers and all family denominators.

## 6. Required falsification controls

- Oracle spans but wrong coordinate transform: evidence recall is perfect; map must still fail.
- Correct local geometry but wrong camera/world frame: no allocentric correctness credit.
- Same local appearance at a different location: no false loop closure.
- Same endpoint reached by different routes: route history remains distinguishable.
- Sequential co-occurrence without direct doorway observation: no fabricated adjacency.
- Late contradiction invalidates only affected map claims and preserves prior known-at state.
- Two derived views from one physical source are not independent corroboration.
- Pose/calibration outside the qualified envelope yields Unknown/NotEvaluated for unsupported metric geometry.
- Reordered evidence and renamed candidate IDs do not alter substantive state scores.
- Missing evidence spans cannot be replaced by inferred continuity.
- Chain-of-thought or answer rationale cannot substitute for exact visual grounding.

## 7. Benchmark composition

- **OVO-S-Bench:** native streaming spatial score and L1–L4 diagnostics: https://internlm.github.io/OVO-S-Bench/
- **LongSpace-Bench:** room-tour spatial memory and explicit 3D structural cue comparator; do not conflate its chunked method with causal streaming: https://aclanthology.org/2026.emnlp-main.227/
- **4DMemoryBench:** very long multi-history video memory (approx. 887–900 h, 5,214 QA, 97.98 h max temporal extent). Its evaluated models use Gemini 3.1 Pro as a separate media observer; reproduce this observer/harness condition explicitly: https://arxiv.org/abs/2610.32332
- **EgoMemReason:** week-long entity/event/behavior memory, 500 queries, best reported 39.6%; use as long-horizon memory, not map-only score: https://arxiv.org/abs/2605.09874
- **DCASE 2026 Task 3:** spatial acoustic image reconstruction; keep audio-only and audiovisual metrics distinct: https://dcase.community/challenge2026/task-semantic-acoustic-imaging-for-sound-event-localization-and-detection-from-spatial-audio-and-audiovisual-scenes

## 8. Owners and non-duplication

- Symthaea #13855 / #3934: spatial-state construction and Vision Mind persistent belief state.
- Symthaea #8152: HDC/dense/hybrid representation ablation.
- Symthaea #12837: streaming consumer and benchmark adapter.
- Symtropy #6183/#6184: synthetic hidden-world fixtures and evaluator/observation separation.
- Symtropy #7555/#7558/#7577: decision partition and independent sufficiency evaluator.
- Symtropy #6675: sequential action/probe scheduling and deadlines.
- Symthaea MACE #13284: multi-query memory cost–regret and retained residuals.

No second encoder, evidence graph, memory store, map service, simulator, sensor driver, clock service or scheduler is introduced.

## 9. Promotion gates (prospective)

- Reproduce the exact official OVO-S protocol and strongest compatible baselines.
- L2 oracle-evidence gains are reported separately and never claimed as native streaming results.
- Candidate map projections pass deterministic false-adjacency, false-loop-closure, transform-mismatch and hidden-truth leak controls.
- Matched-backbone ablation demonstrates that each retained module improves a predeclared score plane.
- First targets: OVO-S overall >=62 and L4 >=58; stretch >=65 and L4 >=62, with no material L1/L2 regression.
- Efficiency target: >=20% lower all-in measured cost at a preregistered quality non-inferiority margin.
- Publish paired source-video-clustered uncertainty intervals, failures, unsupported cases and all denominators.

These are research targets, not Symthaea results.

## 10. First implementation tranche

1. Add the L0–L3 adapter manifest to Symthaea #12837.
2. Bind 10–20 paired cases from #6184 to versioned #1595 worlds.
3. Implement one common candidate-visible map projection; keep evaluator joins private.
4. Run at least one general video backbone, one explicit geometry/topology pipeline and one HDC/hybrid arm on the same projections.
5. Publish per-axis failures before changing training, retrieval or production memory policy.


## 11. Constraint-carrying spatial state hypothesis

A single best-estimate global map can hide ambiguity, identity swaps, transform drift and unsupported topology. The preferred representation hypothesis is a **versioned set of spatial constraints and admissible map alternatives**, rather than an early collapse to one map.

Candidate outputs may include:
- relative camera/landmark transforms with uncertainty and calibration revision;
- observed traversals and inferred connectivity as different relation types;
- competing entity/landmark identity hypotheses;
- temporal pose/trajectory constraints;
- visibility, occlusion, support and containment relations;
- loop-closure proposals with supporting observation references;
- contradiction/supersession links and residual unresolved alternatives.

### Required invariants

1. **Frame correctness:** image coordinates do not become world coordinates without an admitted transform and uncertainty envelope.
2. **Cycle consistency:** a transform composition around a closed pose-graph cycle must agree with identity within a preregistered residual/uncertainty bound. A failed closure is retained as rejected or alternative, not silently optimized away.
3. **Topological validity:** adjacency requires direct observation or an explicitly typed inference rule; sequential visits alone do not prove a connection.
4. **Identity consistency:** loop closure cannot merge landmarks on appearance alone when route/geometry constraints conflict.
5. **Temporal consistency:** late evidence revises current map state but preserves what was known at earlier query times.
6. **Decision sufficiency:** resolve only distinctions that change the current DecisionNeed's action; unresolved constraints remain reopenable.

### Additional scorecard dimensions

- pose-graph cycle residual and uncertainty coverage;
- false loop-closure rate and accepted/rejected closure counts;
- topological edge support class and false adjacency;
- number and size of admissible map hypotheses retained;
- contradiction localization and minimum affected-subgraph revision;
- consistency under viewpoint change and camera/world frame perturbation;
- cost to reopen a distinction after a DecisionNeed change.

This is a testable architecture hypothesis, not a commitment to make a factor graph or SLAM package authoritative by default. Dense models and HDC may propose evidence/relations; explicit geometry/topology constraints can expose inconsistencies; the independent evaluator determines admissible truth and decision sufficiency.
