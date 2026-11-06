# ROAD Decision-Quality Benchmark Protocol v0.1

**Status:** research protocol and reproduction plan; not a qualified safety standard  
**Owner:** Luminous Dynamics / Symtropy + Symthaea (EGCI/SAM)  
**Date frozen:** 2026-11-06  
**Purpose:** compare autonomous-driving planners and interactive world models without collapsing open-loop prediction, closed-loop driving, multi-agent realism, and black-box decision evidence into one score.

## 1. The paradigm shift

The field should stop treating “predict the next scene well” or “score well on one driving leaderboard” as sufficient evidence of decision quality.

A useful evaluation system must answer four different questions:

1. **Did the system observe and act safely?** Measure realized surrogate safety and physical feasibility.
2. **Were the likely consequences of its action predicted correctly?** Measure action-conditioned outcome prediction and calibration.
3. **Was a better action available among actions the evidence actually supports?** Compare the chosen action with supported counterfactual alternatives, not arbitrary simulator branches.
4. **Where does it systematically fail?** Diagnose weakness by scenario family, interaction type, sensor condition, and operating envelope.

This aligns with NIST's 2026 Measurement Science for Automated Vehicles plan: four progressive tiers from TTC/PET/DRAC, through forward simulation, counterfactual action comparison, and systematic weakness diagnosis. NIST describes the interface as black-box and states that its role is to develop measurement methods and reference baselines; acceptable thresholds remain for regulators and industry to determine. This protocol is an engineering alignment, not NIST endorsement or certification.

**Core separation:** benchmark-native scores remain native. EGCI adds evidence qualification, supported counterfactuals, event lineage, and decision diagnosis; it does not manufacture a composite “safety score”.

## 2. Benchmark tracks and current reference points

Numbers below are protocol-specific reference points, not a single ordered ranking. Before claiming a win, reproduce the exact split, sensors, data, simulator, route adaptation, candidate count, post-training, inference budget, and score implementation.

| Track | Reference point (2026-11-06) | What it measures | Reproduction rule |
|---|---|---|---|
| NAVSIM-v1 navtest | World4Scorer: 94.0 PDMS; PlanWAM: 93.8; ReWAM: 90.6; DriveVLA-W0's published one-camera row: 90.2 | Standardized planning score / trajectory outcomes; ReWAM also reports 88.72 PDMS on its 553-token critical-interaction subset | Keep PDMS separate from EPDMS; record sensor count, checkpoint, post-training, response depth, candidate count, and scoring code |
| NAVSIM-v2 navtest | World4Scorer: 93.0 EPDMS with inertial re-ranking, 91.4 without; PlanWAM: 90.9 EPDMS | Extended predictive driver metrics | Report each EPDMS subscore (including lane keeping, comfort/history comfort, extended comfort, DDC, TLC), not just the aggregate |
| Bench2Drive-220 / Bench2Drive 0.0.3 | World4Scorer reports 73.27 Driving Score / 46.36% success on its adapted Bench2Drive-220 subset. The 0.0.3 self-reported leaderboard's non-privileged section lists BridgeDrive (LEAD-trained) at 96.34 / 89.25 SR and TFv6 at 95.28 / 86.81 SR; a separate privileged section lists PDM-Lite 97.02 and LEAD 96.78 using ground-truth perception | Closed-loop full-stack driving on CARLA routes | These are not a matched comparison: the leaderboard says methods differ in data, expert, training and inference compute; its rows are copied from papers. Keep World4Scorer's adapted subset separate and ground-truth-perception entries in a privileged track |
| HUGSIM zero-shot | PlanWAM paper: 53.7 Route Completion / 38.7 HD-Score on 436 public scenarios; a current public aggregator lists DriveZero-Scale at 46.6 HD-Score | Closed-loop generalization in a neural rendering simulator | Resolve split, public/private scope, checkpoint, and simulator version before direct comparison. First reproduce PlanWAM's 436-scenario zero-shot setting |
| ReactSim-Bench | 2,636 test scenarios across three deviation categories | Reactive agent response to AV trajectories that depart from logged behavior | Evaluate collision, map adherence, and kinematic feasibility separately; preserve the distinction between one-sided reactive simulation and a reciprocal ego+agent policy loop |
| Waymo Open Sim Agents Challenge (WOSAC) | Available leaderboard snapshot lists SMART-R1 at 78.58 Realism Score, with 49.44 Kinematics, 81.1 Interactivity, and 92.01 Map Adherence | Realistic multi-agent trajectory simulation | Use as an agent-realism track, not a driving-policy score. Report each component and the exact challenge year |
| NIST decision-quality tiers | Four-tier black-box measurement plan | Decision quality and systematic weaknesses | Map our outputs to the tiers below; do not claim NIST conformance without a separately defined mapping and validation |

### Important protocol notes

- NAVSIM-v2 EPDMS extends PDMS. Its additions include lane-keeping and extended-comfort weighted metrics, history comfort, DDC/TLC multipliers, and false-positive filtering. PDMS and EPDMS are not interchangeable.
- World4Scorer is an outcome-grounded candidate scorer: simulator outcomes supervise generated candidates while the logged future anchors the shared predictor. Its 93.0 EPDMS, 94.0 PDMS, and 73.27 adapted Bench2Drive-220 scores belong to separate evaluation tracks.
- The official Bench2Drive 0.0.3 repository says its leaderboard is self-reported and copied from publications, and that methods vary in datasets, experts, pre-training, training compute and inference compute. It separates ground-truth-perception privileged methods below a divider. Its last-updated date is 24 September 2026; treat 96.34 (BridgeDrive/LEAD) as a published leaderboard reference, not a controlled apples-to-apples architecture comparison.
- DriveVLA-W0's official NAVSIM table lists a 90.2 PDMS one-camera row and a separate dagger-marked 93.0 row. This matrix records the 90.2 row only until the symbol/configuration note is pinned in the reproduction manifest.
- PlanWAM uses (K=64) trajectory candidates, 8 waypoints and a 4-second planning horizon in its project-page configuration. This makes (K=64) a required candidate-count operating point for any selector reproduction, not an optional scale-up.
- - World4Scorer generates 64 candidate trajectories and uses a separate 16-candidate bank for additional simulator-labelled training states. Its released NAVSIM checkpoint is 35.5M parameters. It reports 94.0 PDMS on NAVSIM-v1, 93.0 EPDMS with inertial re-ranking on NAVSIM-v2, and 73.27 Driving Score on the adapted Bench2Drive-220 track. These scores remain distinct protocol tracks.
- PlanWAM's key research idea is *planning-shaped future representation*: trajectory-planning objectives shape a future latent, then a history-only prior predicts it at inference. Its NAVSIM and HUGSIM results should be reproduced independently.
- DriveVLA-W0 is a dense-supervision/data-scaling comparator: future-image prediction supplies self-supervision to a VLA, with a lightweight action expert for inference.
- ReWAM is the interaction-modeling comparator: a Level-k response hierarchy uses role-specific ego/other action DiTs and cross-agent attention over a shared future-world representation. It reports 90.6 NAVSIM PDMS; its 553-token critical-interaction subset scores 88.72 vs 84.13 for DriveLaW and 86.18 for DiffusionDrive. Reported reasoning depth increases PDMS from 90.03 at Level-1 to 90.64 at Level-4, with no further PDMS gain at Level-5. Its evaluation uses four front-camera frames sampled at 2 Hz, eight predicted poses at 0.5-second intervals, and up to ten other agents within 80 m. These are paper-reported results and must be reproduced under its exact configuration.
- A 2026 world-model benchmark survey catalogs 102 benchmarks and argues that evaluation claims need four coordinates: target capability, protocol, metric family, and data source. This matrix adopts that discipline.

### Reproduction implications from the current leaders

The leading methods already cover several important paradigms:

- **Planning-shaped representation (PlanWAM):** the future latent is shaped by trajectory-planning losses, then distilled to a history-only prior. The published configuration uses 64 candidate trajectories and eight waypoints over a four-second planning horizon.
- **Outcome-grounded candidate scoring (World4Scorer):** a shared latent predictor scores 64 generated candidates; simulator outcomes supervise the candidate bank and the logged future anchors the predictor during training. Its released NAVSIM checkpoint has 35.5M parameters.
- **Reciprocal policy generation (ReWAM):** a finite Level-k hierarchy makes ego and other-agent action distributions conditional on preceding-level strategies. Its own depth ablation shows diminishing returns by Level-4.
- **Anchor-guided closed-loop planning (BridgeDrive):** a diffusion bridge refines coarse expert anchors into context-aware plans and reports 96.34 Driving Score / 89.25 success rate in the Bench2Drive 0.0.3 public leaderboard.

The EGCI contribution must be different: take any of these planners as a black box, preserve the exact observation/action/event lineage, test whether their selected-action advantage survives repeated held-out continuations and reciprocal responses, and expose when SAM support or identification is insufficient. Architectural novelty is not a substitute for this measurement contribution.

## 3. The Luminous comparison architecture

The intended advantage is the composition of complementary paradigms, not a claim that any one component has already beaten the field.

| Layer | Industry approach to reproduce | Luminous research direction | Falsifiable contribution |
|---|---|---|---|
| Future representation | PlanWAM; DriveVLA-W0; DriveUniWAM | Planning-sensitive latent state with explicit uncertainty and temporal memory | Better supported action ranking, not merely lower video reconstruction error |
| Candidate evaluation | World4Scorer generate-and-select | Candidate-conditioned outcome models with calibration and support sets | Lower regret on held-out supported continuations under matched compute |
| Reciprocal interaction | ReWAM; ReactSim-Bench; WOSAC | Ego and other-agent policies each act on their own delayed/partial observations | Demonstrated ego→agent and agent→ego effects; no same-tick or hidden-state leakage |
| Execution evidence | CARLA/HUGSIM/WOSAC replay | Scenario-bound pose commands, event ordinals, per-tick physics receipts | Exact replay and auditable action→command→physics→observation lineage |
| Decision evaluation | PDMS/EPDMS, Driving Score, HD-Score | NIST Tier 1–4 mapped EGCI/SAM certificates | Evidence-qualified safety/outcome assessment and systematic weakness diagnosis |

## 4. NIST-aligned measurement tiers

### Tier 1 — observed surrogate safety and feasibility

For every episode retain time series and provenance, not just minima:

- minimum TTC and time-above/below threshold;
- PET for crossing conflicts;
- DRAC / required deceleration;
- minimum separation and collision/contact;
- road-boundary / drivable-area violations;
- acceleration, jerk, steering-rate and other actuator/kinematic envelope violations;
- cue age, sensor dropout, occlusion, and time synchronization.

Report event count, exposure denominator, percentile distribution, and scenario-family breakdown. Never infer physical safety from a single TTC threshold.

### Tier 2 — action-conditioned forward outcomes

For the chosen action and each candidate action, report:

- forecast collision / near-miss probabilities and reliability curves;
- Brier score or log loss for binary outcomes;
- calibration error with confidence intervals;
- predicted-vs-realized minimum separation, TTC, PET, route progress, and comfort;
- horizon-dependent discrepancy and divergence time;
- sensitivity to exogenous disturbances and continuation seeds.

The model is not credited for accurate video if it misranks the consequence of an action.

### Tier 3 — supported counterfactual comparison

For each decision, compute regret only over alternatives that pass the declared support gate:

`regret = loss(chosen action) - min(loss(a) : a is supported)`

Retain:
- the supported action set and its evidence;
- chosen action, best supported alternative, and loss gap;
- identified interval where multiple support-compatible outcome models disagree;
- action ranking stability across held-out continuations;
- coverage and abstention rate.

An unsupported counterfactual is **unknown**, not a convenient low-cost oracle. No regret is reported when support, replay, candidate-set, or causal-pairing gates fail.

### Tier 4 — systematic weakness diagnosis

Cluster at the scene/family level (the whole interacting episode is the unit when agents influence one another). Diagnose failure by:
- crossing / merge / cut-in / yielding / occlusion / negotiation;
- actor type and response profile;
- weather, lighting, road geometry, signal state;
- sensor latency, dropout, and asymmetric view;
- policy version, action mode, and ODD slice.

Use paired cluster-level inference, not frame-level pseudoreplication. Pre-register primary outcomes and scenario-family strata. Publish uncertainty, multiplicity handling, and excluded/unsupported episodes.

## 5. Reciprocal EGCI suite

The new reciprocal producer is a protocol conformance start, not a benchmark win. The research suite must grow from synthetic fixtures to frozen scenario families.

Minimum controls:
1. **Fixed-ego tape:** actor response to a frozen ego trajectory; retain as the one-sided baseline.
2. **One-sided reactive:** ego deviates while agents react; compare with ReactSim-style behavior simulation.
3. **Two-sided reciprocal:** both ego and actors re-decide after their own observations are delivered.
4. **No-clairvoyance:** no policy can consume a cue before its delivery event and input cutoff.
5. **Future-command noninterference:** changing a future command cannot change an earlier receipt.
6. **Asymmetric observations:** different actor views, delayed cues, dropout, stale and occluded inputs.
7. **Independent policy forks:** branch-specific state chains cannot import another actor's hidden state.
8. **Common disturbance pairing:** shared exogenous disturbances are authenticated and aligned before variance reduction is claimed.
9. **Receipt tampering:** mutate source cue, decision, joint action, pose command, replay snapshot, or anchor; verification must fail.
10. **Deadline and censoring:** timeout, missing action, censored physics, and incomplete correspondence remain incomplete.
11. **Fresh-world and fresh-process replay:** exact same-runtime bytes first, then separately qualified semantic equivalence.
12. **Family generalization:** hold out entire interaction/scenario families, not just frames from familiar scenes.

Report both directional feedback effects and the no-feedback controls. A system that reacts to other agents but not vice versa is not reciprocal.

## 6. Proposed quantitative targets

These are engineering targets for a controlled reproduction campaign, not external standards or present capabilities.

| Area | Reproduction gate | Stretch target after reproduction |
|---|---|---|
| NAVSIM-v1 | Reproduce PlanWAM 93.8 PDMS and World4Scorer 94.0 PDMS in their exact navtest configurations | ≥94.5 PDMS with no regression in NC, DAC, TTC, comfort, or EP and matched sensor/data/compute constraints |
| NAVSIM-v2 | Reproduce World4Scorer 93.0 EPDMS with inertial re-ranking and 91.4 without; separately reproduce PlanWAM 90.9 | ≥94.0 EPDMS on the same split/configuration, with all subscore deltas disclosed and no hidden post-processing advantage |
| Bench2Drive | Reproduce the current top listed leaderboard configuration (89.07) and World4Scorer's adapted 73.27 / 46.36% result separately | ≥90.0 Driving Score on identical full route set and deployment rules; improve success and reduce collisions, not score alone |
| HUGSIM | Reproduce PlanWAM's zero-shot 53.7 RC / 38.7 HD-Score setting; resolve the 46.6 public aggregate result's exact scope | First ≥55 RC / ≥40 HD on the exact PlanWAM split; then target ≥47 HD only after matching the current public submission protocol |
| ReactSim-Bench | Run all 2,636 test scenarios and report all three deviation categories | ≥15% relative reduction in collision/risky-TTC outcomes against the best reproduced one-sided baseline, with map and kinematic feasibility non-regressing |
| WOSAC | Reproduce the challenge-year baseline and each component score | Improve interactivity while holding kinematics and map adherence at or above the reproduced baseline; do not trade plausibility for realism score |
| EGCI reciprocal | 100% fixture-level no-leakage and event-order conformance; 100% exact fresh-world replay in the declared runtime | Generalize across held-out interaction families; quantify feedback effect, regret, risk/coverage, calibration, latency and energy |
| Efficiency | Publish p50/p95/p99 latency, GPU/CPU memory, FLOPs, and joules per scenario | Beat the reproduced baseline at equal or lower compute; publish Pareto frontiers rather than one hand-picked operating point |

For primary research claims, aim for at least 100 independently defined scenario families, with the episode/family as the inferential cluster. The final sample size must be set by a pre-registered power analysis for the chosen primary endpoint; the number 100 is a planning floor, not a guarantee of statistical power.

## 7. Experimental rules that prevent leaderboard gaming

- Freeze benchmark release, split, scenario list, sensor inputs, action interface, candidate count, replanning frequency, simulation version, route-point/controller adaptation, and scoring implementation.
- Use official code and checkpoints where available. Record commit SHA, checkpoint SHA-256, and artifact provenance.
- Separate model improvement from post-processing, extra supervision, privileged future information, extra data, and simulator-specific fine-tuning.
- Compare at matched compute and report a second unconstrained best-effort result if desired.
- Keep train, calibration, and sealed evaluation continuations disjoint.
- Pair candidates on common exogenous disturbances only when the causal-invariance assumptions are independently justified.
- Use family-clustered intervals and disclose multiplicity, censoring, failed runs, and abstentions.
- Publish raw submetrics and failure slices. A larger aggregate must not conceal worse collision rate, comfort, feasibility, or ODD coverage.

## 8. Matched-compute selector protocol

For the Luminous selector study, the candidate-count sweep must include (K=64), because both PlanWAM and World4Scorer publish 64-candidate configurations. Extend the initial (K=1,2,4,8,16,32) ladder to (K=1,2,4,8,16,32,64). Candidate generation, candidate scoring, repeated continuations, support filtering and post-selection replay must each be included in the compute ledger.

## 9. Immediate execution order

1. Reproduce PlanWAM and World4Scorer NAVSIM-v1/v2 baselines with exact checkpoints and code; include their K=64 operating point.
2. Reproduce current Bench2Drive full-stack leader and the distinct World4Scorer adapted track.
3. Reproduce PlanWAM HUGSIM zero-shot, then investigate the public 46.6 HD-Score result under a matching protocol.
4. Run ReactSim-Bench and WOSAC as agent-response realism tracks.
5. Connect reciprocal episodes to the v0.2 event/physics receipts and SAM support gate.
6. Add Tier 1–4 decision certificates, then perform sealed held-out family evaluation.
7. Only after the above, claim a benchmark improvement or an expanded operational envelope.

## Sources and version anchors

- [NIST — Measurement Science for Automated Vehicles](https://www.nist.gov/programs-projects/measurement-science-automated-vehicles-0) (created 2026-03-24, updated 2026-04-24).
- [NAVSIM — EPDMS metric definition](https://github.com/autonomousvision/navsim/blob/main/docs/metrics.md).
- [World4Scorer paper](https://arxiv.org/abs/2609.36438) and [official release](https://github.com/World4Scorer/World4Scorer).
- [PlanWAM paper](https://arxiv.org/abs/2610.11382) and [project page / 436-scenario HUGSIM table](https://jinchan9.github.io/PlanWAM_project_page/).
- [DriveVLA-W0, ICLR 2026](https://proceedings.iclr.cc/paper_files/paper/2026/hash/0d70423f59c5fdd24f0dd3fa52e34623-Abstract-Conference.html) and [official repository / NAVSIM table](https://github.com/BraveGroup/DriveVLA-W0).
- [ReWAM: Reciprocal World Action Models](https://arxiv.org/abs/2609.39245).
- [ReactSim-Bench](https://arxiv.org/abs/2606.14058).
- [Bench2Drive 0.0.3 self-reported leaderboard](https://github.com/autonomousvision/Bench2Drive-Leaderboard) (last updated 24 Sep 2026; privileged ground-truth-perception methods separated below the table).
- [HUGSIM public leaderboard snapshot](https://scholarcompass.net/en/tasks/autonomous-driving/benchmarks/hugsim).
- [Waymo Open Sim Agents Challenge](https://waymo.com/research/the-waymo-open-sim-agents-challenge/) and [available 2025 leaderboard snapshot](https://www.sota2.com/research/sota/motion-simulation-on-waymo-open-sim-agents-challenge-2025).
- [A Survey of World Model Benchmarks](https://world-model-benchmarks.github.io/), final manuscript snapshot dated 2026-09-15.
