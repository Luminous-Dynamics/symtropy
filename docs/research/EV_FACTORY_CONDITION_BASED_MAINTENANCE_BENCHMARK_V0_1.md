# EV Factory Condition-Based Maintenance Benchmark v0.1

**Status:** research/benchmark contract; no maintenance-aware policy implementation is claimed by this document.
**Evidence class:** initial fixtures must be synthetic.
**Purpose:** define a reproducible test bed for whether condition information improves the *production system* once sensor errors, action delays, finite buffers, and constrained maintenance resources are included.

## 1. Why this is the next research step

The current EV factory DES models machine unreliability through configured MTBF/MTTR parameters. That is useful for initial sensitivity studies, but it does not yet represent the causal chain required to evaluate condition-based maintenance:

`latent health → imperfect observation → policy → maintenance queue/action → production outcome`

This work should not duplicate adjacent work:

- [EV-FACTORY-LEADTIME-001 #2601](https://github.com/Luminous-Dynamics/symtropy/issues/2601) owns distributional flow time, censoring, and matched finite-buffer analytical parity.
- [EV-FACTORY-DESIGN-001 #2526](https://github.com/Luminous-Dynamics/symtropy/issues/2526) owns joint factory-design search and exact small-space intervention oracles.
- [EV-FACTORY-LIFECYCLE-001 #2444](https://github.com/Luminous-Dynamics/symtropy/issues/2444) owns asset acquisition, reuse, and redeployment economics.
- [EV-FACTORY-CBM-001 #2619](https://github.com/Luminous-Dynamics/symtropy/issues/2619) is the implementation tracker for this benchmark.

The distinctive question here is whether a policy that consumes condition information creates operational value **after** false alarms, missed warnings, lead-time-to-action, maintenance duration, and shared crew/tool constraints are accounted for.

## 2. Research basis and external comparators

NIST's [SimPROCESD](https://www.nist.gov/services-resources/software/simprocesd) explicitly targets asynchronous manufacturing lines with finite buffers, machine degradation, maintenance, and evaluation of condition monitoring together with maintenance policies. Its public [repository](https://github.com/usnistgov/simprocesd) is a candidate independent comparator. Its documentation describes the package as early development, so pin an exact release/commit and dependencies.

[NIST Simantha 1.0.0](https://www.nist.gov/services-resources/software/simantha-simulation-manufacturing) is a second DES comparator for asynchronous finite-buffer lines, degradation, and maintenance; reference DOI: [10.18434/mds2-2530](https://doi.org/10.18434/mds2-2530).

Dadfarnia, Sharp, and Herrmann's 2025 systematic review, [“Comprehensive evaluations of condition monitoring-based technologies in industrial maintenance”](https://doi.org/10.1016/j.jmsy.2025.06.015), notes that studies are hard to compare when condition-monitoring assumptions, policies, operational performance, economic outcomes, and uncertainty analysis are not reported consistently. The contract below makes those dimensions explicit.

These are comparators, not assumed ground truth. A parity result is permissible only for a declared feature subset with matched event, degradation, failure, repair, blocking, and maintenance-resource semantics. Unsupported comparisons must return `not_comparable` with explicit mismatch reasons.

## 3. Model boundary: keep five mechanisms separate

### 3.1 Latent health and failure

Start with one explicitly defined synthetic health state per machine, bounded to a declared range and advanced by operating exposure. The initial implementation must choose one failure convention and name it: threshold-triggered failure, or a declared stochastic hazard conditional on health. Do not mix an age-threshold model with a memoryless failure model without making the regime transition explicit.

Version and hash the degradation law, operating exposure rule, failure threshold/hazard, preventive-maintenance reset effect, corrective-repair reset effect, and maintenance durations separately. Include a constant-hazard baseline so the health-aware model can be reduced to known simpler semantics.

### 3.2 Condition observation

Observed measurements are not the latent state. The observation adapter must independently specify sensor noise, bias, quantization, missing samples/dropout, detection delay, false alarms, missed actionable warning windows, and drift. A health value must not leak directly to a deployed-style policy unless that run is explicitly labeled perfect-information oracle.

The v0.1 benchmark is not a machine-learning leaderboard. Begin with a frozen threshold policy. Forecasting or learned classifiers may be evaluated later against temporal and regime-separated held-out data.

### 3.3 Policy and warning lead time

All policies see the same eligible information at the same decision times:

1. **Reactive:** wait for failure, then request corrective maintenance.
2. **Age/periodic preventive:** request service at a declared operating-age threshold.
3. **Condition threshold:** request preventive service when the observed condition crosses a frozen threshold.
4. **Opportunistic maintenance (later tier):** use a predeclared, modeled production pause or feasible idle window; never treat a convenient future downtime interval as known in advance.
5. **Perfect-information oracle (optional):** a non-deployable bound using latent state/failure knowledge. Label separately; do not compare it as a practical policy.

Do not tune thresholds on held-out seeds or scenario families. If a tuning budget is used, charge it to the policy's total compute/evaluation budget and record the selected parameter and data split.

### 3.4 Maintenance resources and execution

A maintenance request is not a completed repair. The event model must represent request time, queue entry, dispatch, service start/end, machine state while waiting, repair/preventive action, and post-maintenance health. Start with one shared crew and a finite service capacity, then scale only after the small model is validated. Represent tool/spare constraints only when those resources have explicit capacities and state transitions.

A machine cannot receive overlapping maintenance reservations. Crew utilization, idle time, queue length, waiting time, and rejected/deferred requests must be reported. The policy must not have access to future queue entries or the realized failure time.

### 3.5 Production coupling

Maintenance changes machine availability and therefore downstream blocking/starvation, WIP, quality, flow time, and bottleneck location. A maintenance queue must interact with finite buffers through explicit semantics; it cannot be modeled as a standalone post hoc downtime deduction. Preserve per-item cohort accounting and right-censoring from #2601.

## 4. Frozen benchmark ladder

| Tier | Instance | Primary purpose | Required oracle / comparator |
|---|---|---|---|
| A | One machine; constant hazard; fixed repair distribution; unlimited maintenance resource | Validate event ordering, availability, and long-run reward | Hand-computable/renewal-reward result under exact chosen assumptions |
| B | One machine; latent wear plus condition sensor | Separate health truth from sensor quality | Exact trace under deterministic health and frozen sensor sequence |
| C | Two/three machines with finite buffers and one shared crew | Expose blocking, starvation, and repair-queue coupling | Tiny trace oracle first; pinned SimPROCESD subset if semantics align |
| D | Same line under sensor noise, missingness, drift, and failure-mode shift | Evaluate robustness outside ideal sensor assumptions | Held-out regimes with predeclared degradations |
| E | Five-station synthetic EV cell with baseline and interventions | Measure operational value and policy interactions | Same-seed paired simulation; later exact small policy enumeration |

No tier inherits a higher tier's claim automatically. Tier A passing does not prove a multi-station model; Tier C parity does not establish physical validity for Tier E.

## 5. Experimental design

### Independent factors

Begin with a small, versioned factorial or explicitly enumerated design; do not sample arbitrary values and later cherry-pick favorable regimes.

- **Degradation:** slow / nominal / fast operating-exposure accumulation.
- **Observation:** perfect / noisy / missing-at-random / drifted / failure-mode-shifted.
- **Policy:** reactive / age-based / threshold-based.
- **Maintenance capacity:** unconstrained reference / one shared crew / crew contention with a second machine.
- **Production state:** uncongested / near-capacity / finite-buffer bottleneck.
- **Quality coupling:** failure only / explicitly modeled degradation-related yield effect. Keep this optional until its causal rule has a known-answer test.

Each level is a declared synthetic input, not a population-frequency claim. Keep structural worlds separate; do not treat model families as IID replications or invent probabilities for them.

### Seed and information discipline

Use the same seed catalog within each policy comparison and independent streams for arrivals, health/failure, sensor observations, and service/rework wherever practical. Random draws must be keyed or consumed so that adding unrelated telemetry does not silently shift the failure path. Report the stream policy and configuration digest.

Separate calibration/tuning, validation, and held-out evaluation by seed **and** degradation regime. Keep the perfect-information bound separate from deployable policy comparisons. Report the number of replications and the uncertainty method; do not treat every job from one simulated line as an independent replication.

## 6. Required metrics

### Asset health and condition signal
- true failures, warning opportunities, actionable warnings, false alarms, missed actionable warnings;
- warning lead-time distribution and the fraction of warnings that arrive too late for the configured maintenance duration/queue;
- sensor missingness, drift, and failure-mode-specific detection measures;
- probability calibration metrics only if a policy actually emits probabilities and those probabilities are tested.

### Maintenance execution
- planned and corrective maintenance hours;
- time from alarm/request to queue entry, service start, and completion;
- crew busy/idle fraction, queue length/wait distribution, intervention count, deferrals, and capacity violations;
- health state before and after each action, including any residual wear.

### End-to-end production
- good units/hour and terminal good/scrap counts;
- rework, unplanned downtime, planned maintenance downtime, blocked/starved time;
- total system WIP and cohort flow-time summaries with censoring visible;
- bottleneck migration and quality outcomes by scenario family.

### Decision value
- paired policy deltas by seed and world, with uncertainty intervals;
- operating regret relative to the best feasible policy from an exhaustively enumerated small policy set;
- policy rank stability under observation/degradation shift;
- sensitivity to crew capacity, sensor overhead, maintenance duration, and synthetic cost assumptions.

Show the multi-objective frontier across output, delivery tails, quality, crew load, and cost. Do not force these into one hidden weight. Any utility score must have explicit stakeholder-provided weights or hard constraints and must be reported alongside the raw objectives.

## 7. Acceptance gates and proposed targets

The following are proposed gates, not achieved results or universal industry standards:

- **Exactness:** 100% match to frozen hand-computable/analytical micro-oracles for declared supported semantics.
- **Integrity:** zero impossible event orderings, duplicated terminal jobs, overlapping crew reservations, resource-capacity violations, or missing/duplicated cohort outcomes.
- **Determinism:** byte-identical normalized report for identical source, catalog, seed list, and toolchain; source/config/report hashes included.
- **Metamorphic controls:** a policy that never initiates preventive work reduces to the declared reactive behavior; an unchanged constant-hazard configuration matches its baseline; perfect sensing removes sensor-induced false positives/misses without silently making maintenance instantaneous; reducing available crew capacity cannot create more crew capacity or erase already queued jobs.
- **Decision quality:** exact best-feasible-policy agreement on exhaustively enumerable small spaces. Compare fixed-replication against sequential/adaptive allocation at equal simulation budgets using correct-selection rate, false-winner rate with confidence bounds, interval coverage, regret, runtime, and replication count.
- **External parity:** pin tool/revision/config and predeclare tolerances. Require matching semantics first; otherwise record `not_comparable`.
- **Field validity:** require rights-cleared, independently measured and site-separated held-out traces before making any plant calibration, economic-return, safety, or SOTA claim.

A detector should not be declared useful merely because F1 improves. The policy-level acceptance question is whether information reduces relevant production loss or decision regret under the same assumptions and resource budget, with uncertainty and operational costs exposed.

## 8. Failure cases that must be tested

1. Sensor drift creates a large wave of alarms that exceeds crew capacity.
2. False negatives suppress preventive work while the failure hazard rises.
3. A true alarm arrives, but the queue delay means service starts after the failure.
4. Preventive maintenance reduces failure hazard but costs enough downtime to lower good output.
5. Maintenance on a non-bottleneck station does not improve throughput but still consumes scarce crew time.
6. Two machines request service together; queue discipline changes which line constraint becomes binding.
7. Longer repair time increases downstream starvation and flow-time tails even if the mean number of failures is unchanged.
8. A shifted degradation regime reverses the apparent winner between periodic and condition-based policies.
9. Missing sensor observations must remain missing; they are not silently replaced with healthy state.
10. Increasing maintenance crew capacity should not worsen feasible queue service under the same event semantics, unless explicit costs or scheduling changes account for the difference.

## 9. Reproducibility contract

Every run should publish: schema/tool versions; source revision; catalog SHA-256; random-stream policy; seed IDs; scenario family; maintenance and sensor assumptions; oracle/comparator version; comparability status and mismatch reasons; per-seed outcomes; summary/uncertainty method; known-answer and negative-control results; report SHA-256; and the exact claim ceiling.

All first instances remain synthetic. This benchmark does not authorize site data ingestion, vendor outreach, control-system integration, equipment purchasing, or a production-release decision.
