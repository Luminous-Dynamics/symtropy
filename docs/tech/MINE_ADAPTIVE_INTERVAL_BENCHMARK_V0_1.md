# MINE-ACP-001 — adaptive interval benchmark v0.1

## Purpose and scope

This crate provides a deterministic, replayable synthetic comparison of prediction-interval policies under regime change. It is an evaluation harness, not an operating controller. It does not claim to reproduce or outperform Distribution-informed Online Conformal Prediction (COP), PACE, SAOCP, EnbPI, weighted conformal prediction, or any other published algorithm. The interval-update rules below are transparent baselines to establish a common causal benchmark before faithful reference implementations are added.

The central architectural hypothesis is a dual-loop design:

1. **Forecast loop:** estimates prediction uncertainty and may adapt interval width when mature labels arrive.
2. **Credibility/admission loop:** independently evaluates telemetry freshness, current residual envelope, detectable regime novelty, recovery holdoff, and protected physical bounds.

A calibration update can change interval width. It cannot clear the admission latch or authorize model use. Coverage and interval width are always reported together; one composite score is deliberately not emitted.

## Frozen corpus

- 32 validation seeds and 32 disjoint held-out seeds.
- 12 deterministic stream families per seed: nominal; covariate shift; error-scale shift; abrupt shift; slow drift; reversion; delayed labels; missing labels; stale telemetry; missing telemetry; protected-boundary case; unseen regime.
- 64 timestamps per stream, for 768 streams and 49,152 ticks.
- Calibration uses a separate fixed 64-item synthetic residual sample.
- SHA-256 digests bind the serialized calibration set and generated stream corpus in the receipt.
- A held-out stream is never used to select a policy or tune its parameters.

These counts establish repeatability and test coverage, not statistical power for real mines or infrastructure.

## Policies implemented

| Policy | Interval behavior | Admission behavior |
|---|---|---|
| \`fixed_split_interval_only\` | Finite-sample absolute-residual quantile on the fixed synthetic calibration set | Diagnostic only; cannot authorize model use |
| \`online_residual_feedback_interval_only\` | Transparent online residual-feedback radius update using mature labels | Diagnostic only; not presented as formal ACI or COP |
| \`rolling_quantile_interval_only\` | 90th-percentile residual from a rolling window | Diagnostic only |
| \`multi_horizon_interval_only\` | Maximum of short- and longer-window empirical residual quantiles | Diagnostic only |
| \`fixed_split_trust_gated\` | Fixed interval | Independent credibility latch plus interval-bound rejection |
| \`multi_horizon_trust_gated\` | Adaptively updated multi-horizon interval | Same independent credibility latch and bound rejection |
| \`multi_horizon_unguarded_negative_control\` | Multi-horizon interval | Deliberately unguarded offline negative control; never an authorized policy |

The rolling quantile and residual-feedback methods are proxy baselines. They should not be cited as faithful reproductions of COP, PACE, SAOCP, EnbPI, or a distribution-free online-conformal guarantee.

## Causal timing

An interval for tick \(t\) is computed before labels that mature at that tick are consumed. Delayed labels enter the calibration history only at their declared maturity time. Missing labels do not enter calibration and are excluded from the coverage denominator with an explicit count. Current-state telemetry used by the independent gate is modeled as a separate input channel; this synthetic setup assumes the monitor residual is observable when that channel is fresh. Real deployment must validate sensor independence, timestamp alignment, and measurement latency.

The gate latches on missing or stale telemetry, an out-of-envelope current residual, or declared covariate novelty. It requires three consecutive healthy observations to release the latch, and the release-step itself remains in holdoff. An interval crossing the protected range \([0,100]\) is rejected even after latch release.

## Required interpretation of metrics

Each split × scenario-family × policy cell contains:
- empirical coverage numerator / eligible-label denominator and coverage ratio;
- mean, p95 and maximum interval width;
- model-use, diagnostic-only, abstention and unsafe-rejection counts;
- missing-label count and mature-label updates consumed;
- hard-control acceptance count;
- incident-onset detections and detection-delay metrics for trust-gated policies;
- independent replay mismatches for the gate.

Never compare coverage without the associated width and denominator. Missing labels are not treated as uncovered or covered; they are excluded and counted. Small or undefined statistics must not be imputed as success. A wider interval can restore marginal coverage while becoming too uninformative for a protected service constraint.

## Fail-closed receipt contract

The runner requires an exact 40-character lowercase source commit. It rejects malformed source identity, corpus dimension mismatches, non-finite metrics, independent gate replay mismatches, and any trust-gated acceptance during declared hard controls. It also requires the unguarded negative control to expose at least one hard-control acceptance; otherwise the generator would not be testing the intended boundary.

The receipt is deterministically recomputable from source, fixed calibration values and frozen stream generator. The SHA-256 values bind the serialized bytes; they do not authenticate sensors, source producers, or the truth of physical observations.

## External comparisons and benchmark roadmap

Relevant reference directions:
- **[Distribution-informed Online Conformal Prediction (ICLR 2026)](https://proceedings.iclr.cc/paper_files/paper/2026/hash/5f8241131e6fd428aa49914da76b8ad0-Abstract-Conference.html):** a published approach for using estimated nonconformity-score distributions to reduce excessive interval width while addressing coverage under shift. A faithful, version-pinned implementation and its stated assumptions should be integrated as a distinct comparator, not approximated by the proxy here.
- **[Full-scale industrial process drift](https://proceedings.mlr.press/v329/lee26a.html):** Lee & Lee (PMLR 329, 2026) compare split, weighted and online-adaptive conformal methods plus a rolling-residual baseline for semiconductor wastewater pH forecasting. That provides a stronger evaluation template, but it is not a mine-site validation set.
- **[Reliable water-system control](https://doi.org/10.1016/j.watres.2026.126295):** a 2026 Water Research study reports simulation, laboratory-hardware and real-time system evaluation of data-driven control under tank/pressure constraints. It is a reminder to progress from deterministic simulation to hardware-in-the-loop and measured-data validation, while recognizing that its objective and system are not equivalent to this benchmark.

Prioritized next steps:
1. Port reference algorithms with pinned versions/licences, identical forecast inputs, equal label maturity, and matching nominal coverage targets.
2. Add proper time-series blocking and repeated-seed uncertainty intervals for coverage and width; disclose dependence assumptions rather than treating every tick as independent.
3. Add adaptive coverage-error / regret metrics and per-regime conditional coverage where denominators support them.
4. Add latency, memory and update-cost measurement from repeated local runs; keep them separate from statistical metrics.
5. Replay licensed pump/sump and water-system telemetry with a documented temporal split. Calibrate and test per site; keep an independent time period and independent reviewer.
6. Progress to software-in-the-loop, digital-twin comparison, and hardware-in-the-loop only with a safe test envelope. The benchmark emits no commands and is not a safety case.

## Claim ceiling

The only permitted current claim is deterministic synthetic contract evidence: the declared proxies were evaluated on the declared generated corpus, and the independent synthetic gate/replay checks passed. Do not claim SOTA superiority, field safety, real mine calibration, distribution-free guarantees, or operational readiness from this artifact.
