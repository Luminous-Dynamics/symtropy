# Maritime schedule-level simultaneous coverage v0.1

A standalone, dependency-free Rust benchmark for the difference between **per-vessel uncertainty coverage** and **whole-schedule simultaneous coverage**.

This is a follow-on to:
- [PR #7219](https://github.com/Luminous-Dynamics/symtropy/pull/7219): per-vessel joint arrival/handling coverage, temporal drift and missing-label partial identification.
- [Issue #7224](https://github.com/Luminous-Dynamics/symtropy/issues/7224): DCT schedule-level simultaneous uncertainty and decision feasibility.
- [PR #7188](https://github.com/Luminous-Dynamics/symtropy/pull/7188): multi-vessel scarce-reserve recovery.

## Why this benchmark exists

A 90% interval for each vessel does not imply that all vessel calls in a multi-vessel schedule are simultaneously covered. Under an illustrative independence assumption, if 25 calls each have 90% joint coverage, the probability that all 25 are covered is:

```text
0.9^25 ≈ 7.18%
```

Dependence changes the exact probability, but the logical point remains: per-call marginal coverage is not a schedule-level guarantee.

Three approaches are compared, including two calibration sizes for the Bonferroni method:
1. **Schedule-maximum conformal score**: calibrate one score per entire historical schedule, where that score is the maximum normalized arrival/handling residual across all 25 calls. This directly calibrates the schedule as the exchangeable unit.
2. **Bonferroni family-wise allocation at n=120**: allocate total alpha 0.10 across 25 vessel calls, i.e. alpha 0.004 per call. At 120 calibration schedules no finite split-conformal quantile exists.
3. **Bonferroni family-wise allocation at n=249**: the same alpha 0.004 per call, but with the minimum calibration count needed for a finite split-conformal order statistic. If each per-call guarantee is valid, the union bound provides at least 90% schedule coverage without assuming independence.

The benchmark also includes ordinary per-vessel joint 90% split conformal as a deliberately insufficient baseline.

## Finite-sample calibration result

The frozen calibration corpus contains 120 complete schedules. For split conformal, the required order statistic is:

```text
k = ceil((n + 1) * (1 - alpha))
```

For per-call alpha 0.004 and n=120:

```text
k = ceil(121 * 0.996) = 121 > 120
```

There is **no finite distribution-free split-conformal quantile** at that sample size. The benchmark reports `NO_FINITE_SAMPLE_QUANTILE` instead of inventing a bound.

The minimum calibration sample count for alpha 0.004 is 249. The benchmark now evaluates this second sample-size arm as well as the 120-schedule no-finite-quantile arm:
- n=248 gives k=249 > n;
- n=249 gives k=249, which is finite.

The schedule-maximum method calibrates one score per whole schedule at alpha 0.10; with 120 calibration schedules, its rank is 109 and a finite quantile exists. Its intervals may be wider, but its calibration unit matches the schedule-level claim.

## Methods

| Method | Calibration unit | Target |
|---|---|---|
| PER_VESSEL_JOINT_90 | One vessel call at a time | 90% per-call joint arrival/handling coverage |
| BONFERRONI_SCHEDULE_90_N120 | One vessel call at a time | 0.4% per-call miscoverage; no finite quantile with 120 calibration schedules |
| BONFERRONI_SCHEDULE_90_N249 | One vessel call at a time | 0.4% per-call miscoverage; 90% schedule family-wise lower bound when per-call assumptions hold |
| SCHEDULE_MAX_JOINT_90 | Whole schedule maximum across all 25 calls | 90% simultaneous whole-schedule coverage |

Every call has two uncertainty dimensions:
- arrival residual, normalized by 5 hours;
- handling residual, normalized by 20 hours.

The vessel joint score is the maximum of the two normalized absolute residuals. The schedule score is the maximum of the vessel joint scores across all 25 calls.

## Frozen synthetic experiment

- 120 calibration schedules for ordinary per-vessel and schedule-maximum methods; 249 calibration schedules for the finite Bonferroni arm. Each schedule contains 25 vessel calls.
- 120 test schedules in each of three regimes: stationary, temporal drift and burst-tail.
- Deterministic SplitMix64 seeds.
- Heterogeneous call positions with different residual scales.
- Burst regime injects a large residual in one call in selected schedules.
- The test reports both per-call coverage and whole-schedule simultaneous coverage, with a 95% Wilson interval calculated over the 120 whole-schedule test units (not 3,000 clustered call outcomes).
- Bonferroni at n=120 intentionally has no finite quantile; Bonferroni at n=249 is evaluated with a finite quantile. Both appear separately in the 12-row receipt.

These are synthetic structural results, not DCT performance estimates or calibrated operational probabilities.

## Required evaluation

A schedule-level uncertainty set must be evaluated against:
- whole-schedule simultaneous coverage with a 95% Wilson interval over schedules;
- per-call coverage (descriptive; not an independent-sample confidence interval);
- interval widths and efficiency;
- schedule-level false-safe hard-constraint claims;
- berth/quay-crane/yard feasibility;
- critical-connection and receiver-deadline misses;
- participant displacement and reserve consumption;
- schedule recovery latency, observation costs, energy and emissions.

The current crate stops at joint arrival/handling coverage. It does not yet model berth geometry, quay-crane sequencing, yard constraints, TOS semantics or receiver-side cargo delivery.

## Reproduction

From the repository root:

```sh
cargo fmt --manifest-path research/maritime-schedule-level-coverage/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/maritime-schedule-level-coverage/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/maritime-schedule-level-coverage/Cargo.toml --all-targets
cargo test --locked --manifest-path research/maritime-schedule-level-coverage/Cargo.toml --all-targets
cargo run --quiet --locked --manifest-path research/maritime-schedule-level-coverage/Cargo.toml --bin schedule-coverage-audit
```

## Next gates

1. Reproduce the DCT 241-call dataset from #7221 under explicit access and use terms.
2. Replace the synthetic 25-call set with matched DCT call vectors and exact historical chronology.
3. Compare schedule maximum, Bonferroni, SAOCP, PACE and online-optimization conformal baselines.
4. Add schedule-level missing-label partial-identification bounds from #7219.
5. Propagate the whole-schedule set through BAP/QCAP/yard constraints; use exact small-instance optimization as an oracle.
6. Couple admissible schedule recourse to #7188's reserve capacity and participant-consent constraints.

## Claim ceiling

Synthetic finite-state software research only. The benchmark does not establish 90% DCT schedule coverage, operational schedule feasibility, a berth reservation, a TOS command, physical cargo movement or receiver delivery.
