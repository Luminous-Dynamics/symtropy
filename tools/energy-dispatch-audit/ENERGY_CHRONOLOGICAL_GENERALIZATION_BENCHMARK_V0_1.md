# ENERGY-CHRONO-001: Chronological generalization and leakage gate v0.1

## Research question

Do energy-control policies remain credible when they are evaluated on a future contiguous window after a regime shift, with every feature, outcome, and training cutoff timestamped independently?

## Why this benchmark exists

Forecast error does not reliably determine dispatch quality. Published work on multi-building storage management reports different rankings for forecasting and downstream control outcomes, while recent online-storage dispatch research evaluates non-anticipatory policies against MPC and reports regret bounds. Decision-focused power-system research likewise argues for a comparative protocol joining forecast quality, decision speed, and operational cost. The relevant challenge is not to win a synthetic leaderboard; it is to make invalid evaluations fail before any headline performance result is accepted.

References:
- Forecast error versus control value (2025): https://www.sciencedirect.com/science/article/pii/S2666955225000309
- Online dispatch with regret analysis (Applied Energy, 1 June 2026): https://doi.org/10.1016/j.apenergy.2026.127725
- Decision-focused learning benchmark review: https://lacuna.tiptreesystems.com/work/decision-focused-learning-for-power-system-decision-making

## Frozen v0.1 canary

The fixture contains 15 ordered one-step decisions:
- train: indices 0–5, normal regime;
- validation: indices 6–8, transition regime;
- held-out test: indices 9–14, stress regime.

Every row records target time, decision time, signal availability time, and label availability time. Three deployable toy rules are compared: never reserve, persistence using only the previous observed label, and a threshold over a signal available at decision time. A fourth perfect-hindsight rule is deliberately marked non-deployable and excluded from the deployable ranking.

The dispatch-style score vector is non-compensatory:
1. minimize false negatives / essential-service misses;
2. only then minimize false positives / flexible-service curtailments.

A scalar weighted score could hide an increase in essential-service failure behind savings elsewhere. The report preserves both dimensions separately and identifies the physical-dispatch audit as NOT RUN.

## Frozen synthetic known answer

| Rule | Test false negatives | Test false positives | Essential unserved | Flexible unserved |
|---|---:|---:|---:|---:|
| Never reserve | 4 | 0 | 4 | 0 |
| Persistence | 2 | 1 | 2 | 1 |
| Observable signal threshold | 0 | 1 | 0 | 1 |
| Perfect hindsight | 0 | 0 | 0 | 0 |

The hindsight row is a ceiling only, not a deployable method. The observable-signal rule looks strongest on this constructed panel, but this is a protocol fixture with a deliberately informative signal; it is not evidence of general superiority, real-world savings, improved reliability, or SOTA performance.

## Integrity gate

The Rust oracle rejects:
- any row order or split assignment other than the frozen contiguous chronological blocks;
- a feature that became available after the decision deadline;
- a label that is available before or at the decision deadline;
- a deployable model trained beyond the train cutoff;
- any deployable target-label / hindsight policy;
- changed input bytes, altered known answers, missing rows, or reordered model identities.

The raw panel and experiment fixture are SHA-256 pinned. Tests include future-feature injection, split reordering, future-fit cutoff, deployability mutation, label-deadline mutation, train-label perturbation, source digest tampering, exact answers, and deterministic report replay.

## What this does not establish

This v0.1 artifact proves only that the frozen canary and its integrity rules reproduce known answers. It does not measure a real optimizer, forecast runtime, economic value, grid feasibility, stability, field reliability, or comparison against industry methods.

## Promotion ladder toward an industry-grade benchmark

1. **Public data adapter:** versioned demand, PV, weather, tariff, and outage data; record source URL, license, retrieval/version date, source bytes, time zone, daylight-saving treatment, missing-data treatment, and checksums. Keep the imported data immutable.
2. **Rolling-origin testing:** train only on earlier data, tune on a later validation window, then score a frozen test window containing at least one full seasonal cycle. Purge overlapping target horizons and report exactly how windows are formed.
3. **Strong transparent baselines:** seasonal naive, persistence, linear / regularized models, tree-based forecasting, robust MPC, stochastic MPC where probabilities are defensible, and a perfect-information lower bound clearly marked non-deployable. Include relevant open benchmarks (for example CityLearn) where the control formulation matches.
4. **Operational metrics:** essential energy not served, flexible energy curtailed, constraint violations, operating cost, renewable curtailment, emissions, battery degradation proxy, forecast MAE and probabilistic calibration, solver calls, measured p50/p95/p99 wall time, peak memory, timeouts, fallback count, and worst-regime outcomes.
5. **Statistical integrity:** pre-register the primary metric; use blocked/bootstrap resampling by day or week instead of pretending adjacent intervals are independent; report seed and regime distributions, paired confidence intervals, worst-case bounds, and ablations. Do not tune on the final test set.
6. **Independent physical qualification:** send candidate schedules through the independent dispatch auditor and a pinned power-flow / microgrid simulator. Fail closed on essential-service gaps and physical constraint violations.
7. **Replication:** publish the exact data digests, code SHA, dependency lock, hardware, solver configuration, logs, raw per-step actions, and report hashes. A queued workflow is not a passing run.

## Proposed promotion targets (not yet achieved)

- 100% of known future-feature, future-label, post-cutoff-training, random-split, and hindsight-ranking negative controls are rejected.
- 0 accepted physical constraint violations and 0 unreported essential-service violations in the qualified test panel.
- Report critical-service and flexible-service outcomes separately; never use a single weighted scalar as the sole safety metric.
- Compare against matched MPC and robust-MPC baselines on identical chronological windows and scenario seeds.
- Report wall-clock and memory from actual executions; never substitute solver-call counts for runtime.
- Make any superiority claim only if the paired result remains favorable on held-out regimes and the uncertainty interval excludes no improvement, while critical-service gates remain satisfied.

The next meaningful leap is to move this protocol unchanged onto public chronological data and measured closed-loop dispatch. Until that is done, all v0.1 conclusions remain synthetic and methodological.
