# ROAD Decision Integrity & Reciprocity Benchmark Matrix

**Research freeze:** 2026-11-06  
**Status:** proposed internal benchmark specification; not an external standard or validated result  
**Owners:** Symtropy (scenario, action, physics and replay evidence); Symthaea (policy, world-model and counterfactual decision analysis); EGCI/SAM (independent decision evidence and qualification)

## 1. Why another benchmark?

The field has strong planning and world-action results, but its headline scores answer different questions. NAVSIM-v1 PDMS, NAVSIM-v2 EPDMS, HUGSIM HD-Score, Bench2Drive Driving Score, and production crash rates are not interchangeable. They differ in data, splits, candidate budgets, action interfaces, rollout horizons, simulators, route sets, training data and evaluation code.

The defensible Luminous contribution is therefore not a new aggregate score. It is an independently checkable *decision evidence layer* that can answer:

1. What did the policy actually observe, and when did that information become available?
2. Which candidate actions were supported by the current world state and evaluation model?
3. What outcomes were measured versus predicted?
4. Was a better supported action available, and how uncertain is that comparison?
5. Did other agents respond to the ego action, or were they incorrectly treated as passive scenery?
6. Can the decision, command, physics transition and conclusion be independently replayed and audited?

This complements planner benchmarks. It does not replace them.

## 2. Research baseline matrix (results reported by their sources)

| Family | Reported result | What is interesting | Comparison caution |
|---|---|---|---|
| **World4Scorer** | NAVSIM-v1 94.0 PDMS; NAVSIM-v2 navtest 91.4 EPDMS, 93.0 with inertial re-ranking; Bench2Drive-220 73.27 Driving Score | Outcome-grounded candidate scoring; 64 candidate proposals; simulator labels supervise generated alternatives, with one logged future anchoring the shared predictor | Its Bench2Drive run is an adapted system using the official 1,000-clip training subset, a route point and deployment rules. Do not compare 73.27 directly with scores on other Bench2Drive route sets/configurations |
| **PlanWAM** | NAVSIM-v1 93.8 PDMS; NAVSIM-v2 90.9 EPDMS; HUGSIM zero-shot 38.7 HD-Score and 53.7 route completion | Future representation is shaped by planning objectives; a privileged future posterior is train-only, while the history-only prior is used at inference | Different metrics and closed-loop protocol; project-page figures are author-reported |
| **DriveReferee** | NAVSIM navtest 92.02 PDMS and 91.56 EPDMS for its reported full system | Predict the missing geometry, then apply explicit collision/drivable-area rules. The safety verdict itself need not be learned | The cited result uses a 16B system. The paper's matched-candidate/matched-map ablations are more informative than the headline score alone |
| **MM-Future** | NAVSIM navtest 94.0 PDMS; 91.5 EPDMS; HUGSIM zero-shot 32.3 HD-Score | Multiple paired scene/action futures; joint and reciprocal evolution of action and world hypotheses | Multi-mode paired futures are not directly comparable to single-mode or action-only systems without a fixed sampling budget |
| **DriveUniTok / DriveUniWAM** | DriveUniWAM reports 91.54 PDMS after RL post-training among evaluated WAM-based planners | One decodable continuous latent for appearance, projected LiDAR depth and ego motion | Preprint result; separate modalities and post-training choices must be held fixed when reproducing |
| **WA-JEPA** | NAVSIM-v2 navtest 91.7 EPDMS; HUGSIM reported HD-Score 0.4462 across 436 scenarios | V-JEPA-native future masking, latent flow matching and joint future/action prediction | The project page's #1 claim is relative to its publication comparison. The 0.4462 HUGSIM scale differs from some papers' percentage-style HD-Score reporting; do not compare until scorer/version normalization is verified |
| **ReWorld** | Reports closed-loop PDMS 89.1 → 90.4 without RL/post-processing; FVD 81.3 → 61.9 (23.9% lower) | Directly shapes intermediate video/action representations and hard negatives near safety-critical boundaries | Its “closed-loop PDMS” is a different reported evaluation track from NAVSIM-v1 PDMS or NAVSIM-v2 EPDMS; reproduce the exact simulator protocol before comparing |
| **ReWAM** | Reports state-of-the-art NAVSIM results; the abstract gives no exact score | Explicit Level-k reciprocal response hierarchy: ego and other-agent policies condition on one another | Numeric comparison requires the complete paper's same-version tables, splits and candidate budgets. No score is inferred here |
| **DriveVLA-W0 / DriveWorld-VLA** | Current literature/leaderboard reports differ by NAVSIM task and version | Dense future supervision and shared latent-space world/action prediction | Treat as a reproduction family, not one single number |
| **PDM-Lite / Autoware / other reference stacks** | Used as strong classical/reference systems in several benchmark and measurement programmes | Essential non-WAM baselines; can clarify whether gains come from model architecture or candidate search / post-processing | Lock exact software build, route interface, tuning and expert privilege |

**Important:** these are source-reported figures, not Luminous reproductions. A paper's “state of the art” means state of the art under its stated comparison table at publication time; it is not a universal real-world safety ranking.

### Sources

- World4Scorer paper, official release and hashes: https://github.com/World4Scorer/World4Scorer ; https://arxiv.org/abs/2609.36438
- PlanWAM project page: https://jinchan9.github.io/PlanWAM_project_page/ ; https://arxiv.org/abs/2610.11382
- DriveReferee project page: https://ffengc.github.io/drivereferee/
- MM-Future: https://arxiv.org/abs/2609.20377
- ReWAM: https://arxiv.org/abs/2609.39245
- DriveUniTok: https://arxiv.org/abs/2610.35044
- WA-JEPA project page (arXiv:2608.20974): https://wa-jepa.github.io/ ; https://arxiv.org/abs/2608.20974
- ReWorld: https://arxiv.org/abs/2606.27504
- DriveWorld-VLA: https://proceedings.mlr.press/v306/jia26f.html
- Bench2Drive self-reported method table (version 0.0.3; last updated 2026-09-24): https://github.com/autonomousvision/Bench2Drive-Leaderboard

## 3. Industry and measurement-science reference

### NIST's four-tier black-box programme

NIST's Measurement Science for Automated Vehicles programme, updated 2026-04-24, proposes progressive measurement tiers that do not require access to proprietary ADS internals:

| NIST tier | Measurement question | ROAD/EGCI mapping |
|---|---|---|
| **1. Surrogate safety** | What was the observed safety margin during the scenario? | TTC, PET, DRAC, minimum clearance, impact severity, right-of-way interaction; retain sensor/measurement provenance |
| **2. Forward outcome prediction** | What outcomes were likely under the selected action? | Action-conditioned rollouts, calibrated probabilities, horizon-dependent prediction error and uncertainty |
| **3. Counterfactual comparison** | Was a better supported action available? | Supported candidate interventions, outcome difference, regret bounds, counterfactual support and abstention |
| **4. Systematic diagnosis** | Where does the ADS fail repeatedly? | Scenario-family-clustered failure taxonomy, worst-group analysis, confidence intervals and targeted next-scenario selection |

This is an alignment with NIST's direction, not an endorsement or claim of implementation by NIST. The external measurement programme distinguishes measurement method development from regulators' and industry's acceptance thresholds.

Source: https://www.nist.gov/programs-projects/measurement-science-automated-vehicles-0

### Production evidence is a different league

Production AV evidence is not just a higher NAVSIM score. Waymo's live safety dashboard currently reports **271.3 million rider-only miles through June 2026**. It reports about **95% fewer serious-injury-or-worse crashes**, **82% fewer injury-causing crashes**, and publishes location-specific rates, 95% confidence intervals, event definitions and a matched human benchmark methodology. This is company-published retrospective fleet evidence, not an independent certification.

A separate IIHS analysis published 2026-07-23 reported a **68% lower police-reportable crash rate** for its studied Waymo sample, while noting limits in federal AV crash reporting. Its study population and outcome definition are not the same as the Waymo dashboard's matched-market dashboard.

These numbers must not be merged: they use different event thresholds, denominators, samples and human baselines. They show the evidence scale and methodological discipline a credible L4 claim eventually needs. A simulator result, finite oracle, synthetic interaction or receipt hash cannot establish fleet-level safety.

Sources:
- Waymo safety dashboard: https://waymo.com/safety/impact/
- IIHS analysis (2026-07-23): https://www.iihs.org/news/detail/waymos-driverless-cars-crash-less-often-than-people

## 4. Paradigm shifts worth pursuing

### A. From predicting futures to measuring action consequences

A visually plausible future is not necessarily decision-useful. Evaluate *candidate-conditioned outcomes*: for each proposed action, predict occupancy, drivable area, collision/clearance, progress, comfort and rule interaction. Keep one logged future as an observational anchor, but use simulator or measured outcomes for alternatives where the environment model supports them. This is the lesson to reproduce from outcome-grounded scorers, not a claim that a simulator makes every counterfactual true.

### B. From learned safety verdicts to predicted state + explicit rules

Where the rule is known (footprint overlap, signed clearance, drivable boundary), do not spend a model learning the rule. Learn the missing state, propagate its uncertainty, then execute a deterministic geometric test. Keep the analytic verdict independent from the learned perception readout and audit both. DriveReferee is a high-value comparator for this separation.

### C. From passive-agent world models to reciprocal policies

Other road users are decision-makers. A useful model must represent how their response changes when the ego action changes, not just animate a prerecorded trajectory. Use role-conditioned policies, Level-k response or bounded-rational response models, and paired scene/action hypotheses. Evaluate response calibration and unilateral-deviation regret, not only visual prediction or ego imitation.

### D. From a single predicted future to a calibrated distribution of futures

Multi-modal futures are not a decorative ensemble. Preserve distinct interaction modes (yield, continue, hesitate, turn, accelerate, occluded) and their probabilities. Report mode coverage, calibration, tail-risk and the consequences of selecting each mode. Never collapse uncertainty to one mean trajectory when the modes imply different right-of-way outcomes.

### E. From frame/tick timestamps to causal event order

Integer simulation ticks cannot prove whether a cue arrived before a policy snapshot. Use branch-local event ordinals for observation generation, delivery, shared policy-input cutoff, policy decision, command latch and physics commit. This is the v0.2 handoff change: the exact order is committed into policy-history and episode hashes. Producer-declared ordinals still require an independently authenticated event log.

### F. From scalar benchmark scores to evidence-qualified claims

Each conclusion should carry its evidence class: observed, simulated, predicted, counterfactual, replayed, or externally authenticated. A counterfactual advantage cannot be promoted when the action is outside model support. The correct output in that case is an explicit abstention or an interval too wide to rank actions.

### G. From one future mode to a structured distribution of reciprocal futures

The literature is now testing three related, distinct ideas: planning-shaped future representations (PlanWAM), V-JEPA-native latent futures and flow-matched action coupling (WA-JEPA), and multiple paired scene/action hypotheses with bidirectional evolution (MM-Future). ReWAM adds an explicit Level-k strategic response hierarchy. These should be treated as complementary axes, not a single architecture ladder: representation quality, mode coverage, reciprocal response and candidate scoring need separate ablations and matched inference budgets.

The new producer work should therefore not stop at one deterministic TTC feedback loop. The next ROAD-DIR experiment should freeze the ego/other policy profiles and compare (a) passive-agent replay, (b) a single reciprocal mode, (c) a small calibrated response-mode set, and (d) a Level-k best-response approximation. Measure mode coverage, response calibration, interaction regret, oscillation/deadlock and action selection under identical rollout and compute budgets.

## 5. Proposed new benchmark: ROAD-DIR v0.1

**ROAD-DIR** (Road Decision Integrity and Reciprocity) is a proposed complementary evaluation suite, not a replacement for NAVSIM, Bench2Drive, HUGSIM or NIST's programme.

### Unit of evaluation

A unit is a *scenario-family cluster*, not an individual frame. Each scenario package should freeze:

- road geometry, map and coordinate frame;
- initial world snapshot and physics profile;
- actor identities, motion envelopes, policy profiles and action domains;
- sensor/visibility profiles and observation latency/drop/occlusion model;
- traffic-control and right-of-way rules;
- environment and execution protocol;
- scenario family and split assignment.

The sealed test split must be separated by scenario family / source event, not by adjacent clips or nearly identical seeds.

### Required evidence graph

For each branch, retain this causal chain:

`world snapshot -> generated cues -> recipient deliveries -> shared input cutoff -> per-agent policy decisions -> command latches -> exact ordered physics commands -> physics transition -> next snapshot -> outcome measurements`

Each edge has a digest and source identity. A branch must replay in a fresh process from a separately constructed initial world. Hashes are integrity commitments, not signatures; production use needs authenticated producer/build provenance.

### Benchmark tracks

| Track | Primary question | Core measurements |
|---|---|---|
| **DIR-1: Observed safety** | What happened? | TTC, PET, DRAC, clearance, contact/collision, severity, rule compliance, speed/jerk/comfort |
| **DIR-2: Outcome calibration** | Did the model predict outcomes reliably? | Brier/log score for events, CRPS or proper score for continuous outcomes, reliability curves, calibration by horizon and risk stratum, interval coverage |
| **DIR-3: Supported counterfactuals** | Was another action better in this state? | Paired outcome delta, regret interval, candidate support, ranking accuracy, abstention rate and selective-risk curve |
| **DIR-4: Reciprocal response** | Did the model represent other agents as conditional responders? | Response-distribution calibration, interaction-mode coverage, response-conditioned collision risk, ego/other unilateral deviation regret and joint outcome |
| **DIR-5: Integrity and replay** | Can the decision be audited? | Zero admitted future-cue leaks, complete per-agent state chains, one-to-one command/physics mapping, fresh-process replay equivalence, tamper rejection |
| **DIR-6: Robustness and transfer** | What breaks outside the development distribution? | Held-out scenario-family performance, occlusion/delay/weather perturbations, OOD detection, worst-family CVaR, selective fallback |
| **DIR-7: Efficiency** | What does the system cost? | Parameters, training data, GPU-hours, inference latency p50/p95/p99, candidates/second, memory, energy per decision and replay cost |

### Reciprocal-game measurements

For a finite declared action set, define a unilateral deviation regret for agent (i):

`R_i = max_{a_i' in A_i} U_i(a_i', a_-i) - U_i(a_i, a_-i)`

Report each agent's regret and a declared aggregation (max or sum); never call it Nash equilibrium without an explicit utility model, supported action set and sufficiently converged response process. For stochastic policies, compute expected regret with uncertainty intervals. Report opponent-model calibration separately from policy utility.

### Counterfactual support gate

A counterfactual advantage is eligible only when:

1. the initial state and relevant observation histories match under a declared coupling;
2. the candidate action is within the supported action/physics domain;
3. the simulator or outcome model is qualified for the scenario family and horizon;
4. the branch is replayable and its command/event history is intact;
5. uncertainty and model discrepancy are included in the reported outcome difference.

A common random seed or same frame index alone is not evidence of a valid shared counterfactual. If coupling is absent or sensitivity-only, report the comparison as unidentified / sensitivity analysis, not a primary causal effect. CMAQ-018's finite oracle is useful as a test of selected-candidate headroom, but it does not itself measure a trained D3 selector or validate a driving simulator.

### Statistical protocol

- Use paired comparisons on the same held-out scenario-family clusters and same candidate/compute budget.
- Bootstrap or randomise at the family/source-event level; do not treat frames from one route as independent samples.
- Report paired effect size, 95% confidence interval, family-level worst decile and CVaR.
- Pre-register candidate count, inference budget, post-processing, seeds, hardware, simulator version, split and stopping rules.
- Keep calibration and model selection data separate from sealed evaluation.
- Publish every failed, censored, unsupported or abstained scenario; no silent removal of hard cases.
- Report a vector of metrics. Do not hide a collision-rate regression behind better progress or a weighted aggregate.

## 6. Initial qualification targets

These are proposed engineering gates for ROAD-DIR, not established industry safety thresholds:

1. **Integrity:** 100% of accepted decisions have complete event order, policy-state chain, command mapping and replay receipts; zero future-cue leaks in the hostile timing corpus.
2. **Replay:** 100% bitwise same-runtime replay on the frozen binary/profile for the qualified fixture set; all intentional tape, command, context and snapshot mutations rejected.
3. **Counterfactual honesty:** 100% of published action-advantage claims pass the support gate; unsupported claims abstain rather than being assigned a favourable score.
4. **Calibration:** outcome probabilities and intervals are reported with proper scoring rules, reliability plots and family-stratified coverage; target nominal 90% interval coverage on an independent matched-distribution test, then measure degradation under shift.
5. **Planner improvement:** first reproduce named baselines exactly; then seek at least +0.5 absolute points on a named NAVSIM metric with paired 95% CI above zero under a matched input, training-data and inference budget, while showing no statistically credible regression in collision/safety submetrics or worst-family performance. This is a research aspiration, not a safety acceptance threshold.
6. **Reciprocity:** report opponent-response calibration and both agents' deviation regret; demonstrate improvement on a predeclared interactive family split against passive-agent and reciprocal baselines.
7. **Efficiency:** publish p50/p95/p99 latency, candidate count, model size, memory, FLOPs/energy where measurable; compare under the same hardware and decision deadline.

## 7. Reproduction order

1. Freeze a public baseline table for NAVSIM-v1 PDMS, NAVSIM-v2 EPDMS, HUGSIM and Bench2Drive version 0.0.3 / 220-route results as separate tracks.
2. Reproduce World4Scorer's released NAVSIM model/checkpoint first because its model weights and verification hashes are available; separately reproduce its adapted Bench2Drive setup.
3. Reproduce PlanWAM and DriveReferee next: they test planning-shaped future representations and the analytic-referee paradigm.
4. Reproduce MM-Future and ReWAM on interactive scenario families to test paired modes and reciprocal response.
5. Run the same frozen candidate sets through ROAD-DIR outcome calibration and support audits.
6. Only then compare learned selectors/world models under matched budgets and sealed scenario-family splits.

## 8. Current Luminous implementation map (2026-11-06)

| Component | Current artifact | Evidence level |
|---|---|---|
| Finite repeated-continuation headroom oracle | Symthaea #10735 | Exact-head rustfmt, 11 tests and strict Clippy passed on the recorded head. Evaluator reference only; D3 selector not measured |
| Phase-aware event-order receipt adapter | Symtropy #3886 | Exact-head rustfmt, 21 tests, strict Clippy, fixture/schema and SAM DTO checks passed on the recorded head |
| Actor path-to-pose adapter | Symtropy #3748 | Exact-head rustfmt, 10 tests and strict Clippy passed on the recorded head |
| Full kinematic pose physics replay | Symtropy #3892 | Implementation candidate; exact-head format/test/Clippy qualification pending |
| Reciprocal ego/actor producer | Symtropy #3532 | Still missing; synthetic conformance fixture is not a production policy simulator |
| Physics replay provenance | Symtropy #3459/#3539 | Existing generic replay receipt plus ongoing full-pose extension; caller profile digests are not engine attestation |

### Immediate next implementation tranche

Compose one 2-tick synthetic intersection crossing using the existing actor-policy core (#3489), route-to-pose adapter (#3748), and physics replay (#3892). Both ego and actor policies must make decisions from only their own delivered cues; their state chains must remain separate; latches must precede physics; and each physics transition must join one-to-one to the exact ordered kinematic command set and post-step snapshot. Replay in a fresh process. Include a pre-decision positive cue, a same-integer-tick post-physics hostile cue, and a dropped/occluded cue. The producer should emit the v0.2 event ordinals accepted by #3886. Do not write a second physics engine or duplicate the actor policy core.

## 9. Claim ceiling

Passing ROAD-DIR establishes reproducible evidence for the defined scenario set and protocol. It does not by itself establish real-world human-behaviour validity, physical fidelity, legal compliance in every jurisdiction, public-road safety, operational design domain completeness or SAE Level 4/5 capability. Those require external validation, measured vehicle data, independent audits, deployment safety cases and sustained operational evidence.
