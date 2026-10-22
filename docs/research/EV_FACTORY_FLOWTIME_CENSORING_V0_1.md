# EV Factory Flow-Time and Right-Censoring v0.1

## Why this matters

Throughput alone hides item-level delay. A finite-horizon simulator observes completed jobs and jobs that remain in supplier queues, source buffers, interstation buffers, active processing or blocked output at the horizon. Treating only completed jobs as the full population can bias apparent lead-time tails downward.

This increment records a release timestamp for every job and reports **completed-job** flow-time summaries separately from the still-live, right-censored release cohort. It does not pretend that completed-job P95/P99 are population lead-time quantiles when censored jobs remain.

## Cohort contract

For each scenario and replication, the measured release cohort contains jobs introduced at or after warm-up and strictly before the simulation horizon. This avoids counting pre-warm-up arrivals as if they were fully observed and excludes horizon-time arrivals with zero exposure.

Each cohort job must be in exactly one terminal class at the horizon:
- completed good output;
- terminal scrap;
- still live at the horizon and therefore right-censored.

The invariant is:

`release_cohort_jobs = completed_good_jobs + completed_scrap_jobs + right_censored_jobs_at_horizon`

A duplicate job ID in the live system fails closed. Live-job enumeration includes supplier backlog, source buffer, every interstation buffer, busy servers and servers holding completed output while blocked. Quality rework remains part of the same product's total flow time; a scrap is terminal only when the existing DES discards the job after its last configured attempt.

## Report contract

Each scenario has a per-replication `flow_time` record containing cohort counts, completed-good and completed-scrap summaries, censored count and the cohort-accounting result. Completed observations expose sample count, mean and P50/P90/P95/P99/max in seconds.

The root `flow_time_statistics` aggregates **per-replication statistics across the declared seed panel**. It does not pool every job and act as if jobs from one line and time series are IID. It reports seed-level completed-good P50/P90/P95/P99, mean, completed-scrap mean and P50/P90/P95/P99 where available, release/completion/censor counts, censor fraction, and number of replications with usable observations.

For right-censored jobs it also reports observed age at the horizon—mean and P50/P90/P95/P99, aggregated across replications. Each observed age is a lower bound on the eventual flow time, not the missing completion time. We do not yet emit per-job survival rows; these aggregates are evidence for workload diagnostics, not sufficient input for a full Kaplan–Meier or competing-risk estimator.

When a replication has no completed good output, its good-flow summary is `null`, not zero. With few observations, P99 is still shown descriptively but is not evidence of a well-resolved tail. The current implementation is **not** a Kaplan–Meier/survival estimator; right-censoring is explicit, not silently imputed.

## Frozen controls

- Exact interpolation test for P50, P90, P95 and P99 on the known sample `[1,2,3,4]`.
- Negative controls reject negative or non-finite flow times.
- A 500-second synthetic horizon forces unfinished jobs and checks cohort conservation, positive censor counts and unavailable completed-good quantiles.
- The existing 20-seed baseline checks per-replication cohort invariants, seed-level aggregation shape and deterministic replay.
- Flow-time output remains synthetic and does not change any release or safety gate.

## Comparison and research context

Shi & Gershwin (September 2026) derive the steady-state lead-time distribution for a three-machine/two-buffer production line with unreliable machines and finite buffers, then validate it against discrete-event simulation. They find that lead-time distributions can be multimodal and heavy-tailed, with tail percentiles responding non-monotonically to buffer capacity. This is a suitable analytical target *only after* we match its process, failure, repair and buffer assumptions; the current EV simulator's bounded cycle times and after-cycle failure semantics are not the paper's analytical model by default.

Reference: https://doi.org/10.1016/j.ijpe.2026.110047

NIST's Simantha 1.0.0 is an independently developed DES for asynchronous manufacturing lines with finite buffers, machine degradation and maintenance, and supports parallel replications. It is a reference implementation—not assumed ground truth; a parity test must pin version/config and match semantics: https://www.nist.gov/services-resources/software/simantha-simulation ; https://doi.org/10.18434/mds2-2530

NIST AMS 100-85 recommends risk-proportionate verification, validation and uncertainty quantification grounded in measurement and model influence: https://doi.org/10.6028/NIST.AMS.100-85

## Next gates

1. Add an explicitly matched analytical micro-model rather than claiming parity from the current synthetic EV fixture.
2. Compare the complete empirical CDF and P50/P90/P95/P99 against an independent analytical/discrete-event reference, with tolerances declared before execution.
3. Implement a censoring-aware quantile estimator only after stating and testing its assumptions; otherwise keep population-tail disposition as `not_estimable`.
4. Measure system-wide WIP across supplier backlog, source buffer, interstation buffers and active/blocked servers without double-counting, then test Little's Law only for compatible steady-state boundaries.
5. Use rights-cleared, held-out measured traces before any plant-performance or delivery-service claim.

No SOTA, production, ROI, safety or capital-decision claim follows from this synthetic increment.