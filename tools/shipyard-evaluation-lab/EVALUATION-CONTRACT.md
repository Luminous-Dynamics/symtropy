# Shipyard Recovery Evaluation Lab Contract v0

Status: research protocol / not yet an empirical benchmark result.

This contract defines how Symtropy should compare shipyard layout and disruption-recovery policies. It intentionally does not define one weighted leaderboard score. A method that trades a safety violation for higher utilization is not a better method.

## 1. Unit of comparison

The primary unit is a **policy episode**:

- a canonical initial yard state;
- a declared inventory of scenarios and immutable disturbance tapes;
- a source-observation timeline with delivery ticks;
- a policy binary/configuration and declared action interface;
- a fixed decision clock, compute budget, hardware profile and termination rule;
- independently evaluated actions, schedules, outcomes and raw trace;
- a digest-bound receipt connecting all inputs and outputs.

The episode's scenario ID and evaluator ground truth must not be candidate-controlled. Candidate-visible observations must be a strict projection of source truth at the decision tick. Do not let the candidate provide the event history used to judge its own actions.

## 2. Mandatory baselines

| ID | Baseline | Role / limitation |
|---|---|---|
| B0 | Original plan, no recovery | Quantifies the cost of doing nothing; not an adaptive policy. |
| B1 | Earliest due date | Transparent dispatch baseline. |
| B2 | Critical ratio | Transparent urgency baseline; define zero/negative remaining time behavior. |
| B3 | Exact-polygon fixed-order first fit | Geometry baseline; failure to place is not a proof of global infeasibility. |
| B4 | Rolling-horizon replanning | Strong online comparator; freeze horizon, update cadence and compute budget. |
| B5 | ALNS / ABC hybrid | Literature-aligned comparator when implementation and assumptions can be reproduced. |
| B6 | Bounded exact MILP/CP or finite-grid oracle | Small-instance reference; report solver status, bound and budget. |
| B7 | Finite-scenario robust plan | Robustness comparator over the same declared scenario set. |
| B8 | Non-anticipative adaptive policy | Candidate class under evaluation. |
| O0 | Clairvoyant hindsight optimizer | Upper-bound diagnostic only; never ranked as an online competitor. |

Every baseline must consume the same observable information at the same decision ticks. Baseline implementations, parameter files and random seeds must be pinned. Do not silently tune one baseline on the test set.

## 3. Frozen scenario corpus

Each corpus release must include:

- schema version and canonical serialization rules;
- unique scenario IDs and explicit expected scenario inventory;
- geometry, dimensions, units, coordinate frame, polygon vertices and contact policy;
- work identity, mandatory/optional status, processing duration and precedence;
- reservations, resource identities/capacities, availability windows and outages;
- event IDs, canonical payload keys, occurrence ticks, policy delivery ticks and visibility class;
- disturbance-family tags for stratified evaluation;
- source, generator, config, and dataset SHA-256 digests;
- declared train/tune/validation/held-out partition with no cross-partition seed reuse;
- license/provenance and any redactions;
- expected evaluator version and output schema.

Missing, duplicated, unknown, or extra scenarios are a corpus-integrity failure, not a smaller benchmark run. Candidate traces must not define or change the expected inventory.

## 4. Disturbance families

At minimum, include independent and correlated combinations of:

1. workstation delay or outage;
2. vehicle/crane outage and resource recovery;
3. late arrivals and duration overruns;
4. yard-zone closure or reduced usable area;
5. outbound urgency that is blocked by inbound placement;
6. narrow access corridors and contact/clearance edge cases;
7. multiple simultaneous failures and delayed observation delivery;
8. unannounced disturbance families in held-out tests;
9. missing, duplicate, reordered and conflicting observations;
10. compute timeout or interrupted replanning.

Use deterministic tapes for conformance tests and separately seeded stochastic tapes for performance evaluation. Do not call an arbitrary synthetic distribution “realistic” without calibration evidence.

## 5. Information and causal integrity

At each decision tick t:

- only policy-visible events delivered by t may enter the policy's information set;
- evaluator-only future outcomes must remain unavailable to the candidate;
- identical canonical visible histories at the same tick must imply the same canonical current action;
- after a new event becomes visible, adaptation is allowed;
- candidate actions and candidate-reported observations must be separate streams;
- an action-trace audit is not proof that the process did not read hidden data; production evaluations need an instrumented observation API or sandbox, and a pinned policy binary/configuration.

Include positive controls for legitimate post-observation adaptation and negative controls for future-event leakage, payload substitution, missing scenarios and manipulated timestamps.

## 6. Independent acceptance gates

Report each gate separately:

- **G0 Corpus integrity:** schema, hashes, exact scenario coverage, no duplicate identities.
- **G1 Geometry:** exact polygon intersection/contact semantics; no positive-area overlaps; clearance constraints explicit. Add swept-path collision checks before claiming transport-path safety.
- **G2 Schedule/resource feasibility:** mandatory work, precedence, reservations, resource capacities, outage windows and time semantics.
- **G3 Causality:** non-anticipativity and observation access audit.
- **G4 Outcome accounting:** independently recompute metrics from action/schedule traces.
- **G5 Reproducibility:** exact source SHA, configuration digest, seed/tape digest, environment, compiler and raw trace are retained.

A missing evaluator result is **incomplete**, not a pass. A timeout is an outcome, not a row to drop. Budget exhaustion is not infeasibility.

## 7. Metrics — separate dimensions, no blended score

### Hard gates
- number of geometric, swept-path, precedence, resource and causal violations;
- mandatory work omitted;
- incomplete/corrupt scenario episodes;
- unverified actions or unverifiable receipts.

Target: zero hard violations on accepted schedules and zero silently dropped episodes. Any violation blocks the candidate from the performance leaderboard.

### Efficiency
- makespan and weighted tardiness;
- vehicle travel distance/time, empty travel and idle time;
- area utilization computed from actual block polygons (not bounding-box approximations);
- geometric representation inflation, reported separately from utilization;
- throughput and schedule stability.

### Recovery quality
- recovery feasibility rate by disturbance family;
- time to first safe feasible action and time to stable feasible plan;
- number of moved blocks, broken commitments and changed reservations;
- objective regret against exact/bounded oracles on small instances;
- tail performance and worst-case outcomes. Use CVaR only when scenario probabilities are justified and published.

### Systems cost
- wall-clock runtime and p50/p95/p99 recovery latency;
- peak memory and resource use;
- timeout/incomplete rate;
- scaling curve by block count, polygon complexity, scenario count and disruption count.

Never merge these into a single score for the primary result. Pareto dominance and transparent per-metric tables are preferred.

## 8. Statistical and compute protocol

- Pair methods on the exact same initial state, disturbance tape and seed.
- Match hardware, action cadence, wall-clock/CPU budget, termination and observation access.
- For stochastic methods, target at least 30 matched runs per test stratum when feasible; state deviations and power limitations.
- Report all runs, including timeouts and failures, plus median, interquartile range and paired confidence intervals.
- Keep held-out disturbance combinations and instances untouched until configuration freeze.
- Publish raw traces, configs, manifest hashes, scripts and exact commands.
- Report per-case and aggregate metrics; avoid averages that conceal catastrophic tail cases.
- On tiny cases, exhaustively compare against an exact oracle and publish the gap and solver status.

## 9. Publication anchors and fair comparison

The 2026 paper “Spatial-Scheduling Strategy for Irregularly Shaped Blocks Under Disruption in Shipbuilding” reports a hybrid ABC + ALNS + recovery method and Case9_50_4 anchors of utilization 0.775, algorithm time 308.0 seconds, and vehicle time 2,339 minutes over a 15-run study. These are **paper-reported anchors, not reproduced Symtropy results**. The article states data are available from the corresponding author on request. Obtain the original case, constraints, parameters, and timing setup before claiming a direct reproduction.

Reference: https://doi.org/10.1049/cim2.70066

A 2026 cruise-shipyard study argues that block placement, temporal sequencing, obstruction and flatcar scheduling should be coupled rather than treated as isolated problems. Reference: https://doi.org/10.3389/fmars.2026.1843859

A 2026 review notes that optimization and AI methods have differing strengths, while sustained yard-wide validation remains limited. Reference: https://doi.org/10.3390/jmse14040396

Adjacent terminal research already includes adversarial RL for integrated berth/yard/shore-power planning. “Uses RL” is not itself a novelty claim. Reference: https://doi.org/10.1016/j.oceaneng.2026.125850

## 10. Release criteria for a credible superiority claim

A result may claim a measured improvement only when:

1. all hard gates pass on the exact tested candidate revision;
2. the baseline and candidate use identical cases, observations, compute budgets and stop rules;
3. the exact public input/protocol is available, or the result is clearly labeled as a new synthetic benchmark;
4. paired uncertainty estimates and all failed/time-out runs are disclosed;
5. the metric and superiority threshold were declared before test evaluation;
6. no regression is hidden by a weighted score;
7. the claim is scoped to the tested instance family and model assumptions.

Suggested first milestone: exact agreement on the full tiny-instance corpus; zero false-feasible and false-infeasible classifications in conformance controls; detect 100% of injected clairvoyance controls; then report paired efficiency comparisons on a frozen corpus. These are proposed gates, not achieved results.

## 11. Known exclusions

This protocol alone does not provide a physical digital twin, calibrated disturbance probabilities, certified swept-path dynamics, crane/load-chart validation, floor-bearing/stacking safety, operator qualifications, or real-time production integration. Those require explicit models and domain validation before operational safety claims.

## 12. Next implementation increments

1. Implement the versioned corpus manifest and deterministic disturbance-tape generator.
2. Add B0/B1/B2 baselines with identical input/output interfaces.
3. Add per-episode trace and receipt format, reusing shared canonicalization only after API review.
4. Integrate independent geometry/resource/causality evaluators as separate gates.
5. Add paired-run analysis and Pareto reports.
6. Request Case9_50_4 data and pin reproduction instructions.
7. Run held-out, adversarial and shadow-mode replay before considering operational use.
