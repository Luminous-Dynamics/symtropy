# Forest Process-Resolved Trajectory Qualification v0.1

**Parent:** [TROPICAL-FOREST-ECO-QUAL-001 #5173](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Child:** [FOREST-PROCESS-TRAJECTORY-001 #5340](https://github.com/Luminous-Dynamics/symtropy/issues/5340)  
**Branch:** `research/tropical-forest-qualification-v0-1`  
**Date:** 15 November 2026  
**Status:** public synthetic known-truth reference; no field observations ingested

## 1. Why this benchmark exists

A mature-forest biomass total is not a sufficient test of a forest process model.

The 2026 ForestPaths multi-model study compared nine vegetation-demography models—BiomeE, BiomeEP, CABLE-POP, EDv3, ELM-FATES, JULES-RED, LPJ-GUESS, ORCHIDEE and SEIB-DGVM—against observations from boreal, temperate and tropical sites. Forests were disturbed and recovery was followed for at least 420 years. The project summary reports that mature forest carbon stocks could be reproduced while growth was overestimated and mortality underestimated in compensating ways. It also reports major divergence in post-disturbance recovery trajectories and identifies early recovery structure as a data constraint.

Source: ForestPaths, 12 February 2026, [study summary](https://forestpaths.eu/news/forestpaths-study-reveals-how-vegetation-demographic-models-improve-understanding-forest-recovery-structure-and-carbon-dynamics).

This is a concrete failure mode for stock-only calibration: different mechanisms can yield the same observed pool at a census boundary. The benchmark therefore treats pool stocks and process fluxes as distinct observables. It is not an attempt to reproduce the nine-model study or its data.

## 2. Paradigm shift: benchmark the flows that create the state

The bounded v0.1 model has two carbon pools:

- live biomass;
- deadwood biomass.

It keeps six non-negative flows separate:

- growth;
- recruitment;
- mortality transfer from live biomass to deadwood;
- live-pool export (e.g. explicitly declared harvest or combustion);
- decomposition;
- deadwood export (e.g. explicitly declared harvest or combustion).

For one declared interval:

`L_{t+1} = L_t + G_t + R_t - M_t - X^L_t`

`D_{t+1} = D_t + M_t - Q_t - X^D_t`

where `M_t` is an internal pool transfer. Summing the equations gives:

`(L+D)_{t+1} = (L+D)_t + G_t + R_t - X^L_t - Q_t - X^D_t`

Mortality cancels from the total ecosystem-pool budget, but it must not disappear from the live/deadwood process diagnostics. A model can match total carbon while placing carbon in the wrong pool, or match both pool states while using incorrect gross fluxes.

The benchmark does not collapse these errors into a composite score.

## 3. Implemented v0.1 reference

New artifacts:

- `validator/src/trajectory_budget_kernel.rs`: offline Rust reference kernel, eight authored unit tests, ordered-period and continuity checks, pool-specific budget residuals, per-period state errors and primitive-flow error metrics.
- `cases/forest-process-trajectory-synthetic-v0.1.json`: four public synthetic known-truth cases.
- `FOREST_PROCESS_TRAJECTORY_V0_1.md`: method, matched benchmark protocol and claim limits.
- `validator/src/lib.rs`: corpus dispatch under `FOREST-PROCESS-TRAJECTORY-001`.
- `validator/src/main.rs`: CLI accepts `PASS_SYNTHETIC_PROCESS_TRAJECTORY`.

Run from the repository root:

```bash
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/forest-process-trajectory-synthetic-v0.1.json
```

The receipt includes SHA-256 of the exact fixture bytes, per-case dispositions, per-period state errors, per-flow errors, pool residuals, and explicit `ecological_validation: NOT_ESTABLISHED`.

### Synthetic cases

| Case | Designed outcome | Why it matters |
|---|---|---|
| PROCESS-AND-STOCK-MATCH | `PROCESS_AND_STATE_MATCH` | Positive control |
| COMPENSATING-FLUXES-SAME-STOCK-TRAJECTORY | `STOCK_FIT_WITH_PROCESS_MISMATCH` | Both live/deadwood trajectories match exactly, yet growth, mortality and decomposition are each wrong by 20 units |
| STATE-TRAJECTORY-MISMATCH | `TRAJECTORY_STATE_MISMATCH` | Candidate budgets close but the predicted live trajectory misses by 10 units |
| POOL-BUDGET-NOT-CLOSED | `BUDGET_NOT_CLOSED` | Candidate deadwood stock is one unit above the value implied by declared flows |

The key counterexample uses the same initial and final live/deadwood stocks for reference and candidate. It changes growth from 20 to 40, mortality transfer from 15 to 35, and decomposition from 8 to 28 in the first period; the compensating values also preserve the next period's pool states. End-state and stock-trajectory errors are exactly zero, but the maximum primitive-flow error is 20. This demonstrates the failure of stock-only calibration on the frozen arithmetic fixture.

## 4. Interpretation rules

- A pool residual above the declared budget tolerance yields `BUDGET_NOT_CLOSED`.
- If the budget closes but any declared pool stock at a census boundary differs beyond tolerance, the disposition is `TRAJECTORY_STATE_MISMATCH`.
- If stock trajectories match but one or more primitive flows differ, the disposition is `STOCK_FIT_WITH_PROCESS_MISMATCH`.
- Only matching both states and all declared flows yields `PROCESS_AND_STATE_MATCH`.

Mortality is represented as a transfer to deadwood, not as an unexplained disappearance from the system. If field semantics require a fraction of mortality that exits the measured ecosystem (combustion, removal, export), that portion must be typed as an external loss rather than silently subtracted from the transfer.

Unknown, unmeasured, or unsupported flows are not numeric zero. The v0.1 fixture requires an explicit numeric value for each declared field; a future real-data adapter must carry measurement/estimation status, source identity, support, uncertainty and missingness separately.

## 5. What industry and SOTA do today

The relevant competitive baseline is a family of vegetation demographic models, not one direct software product. ForestPaths' 2026 intercomparison is particularly relevant because it benchmarks multiple process models against tree growth, carbon turnover, biomass stocks and tree-size distributions across forest biomes. That is a stronger comparison target than an endpoint-only biomass leaderboard.

EFI already operates community ecological forecast challenges with standardized forecast submissions, metadata checks, scoring and public comparison. Its NEON challenges include terrestrial carbon/water fluxes, phenology and population variables; forecast submissions are scored against baselines. Luminous should adopt the same prospective, archived forecast discipline rather than invent a private scoring convention. See [EFI Forecast Challenges](https://ecoforecast.org/forecast-challenges/) and [NEON forecast instructions](https://projects.ecoforecast.org/neon4cast-ci/instructions.html).

The 2025 ecological forecast-limit framework argues that forecast usefulness requires verification data, a scoring function and a reference error tolerance; it distinguishes potential, absolute and relative forecast limits. This gives the forest programme a way to report *how far ahead a forecast remains useful*, not just whether one horizon's score is good: [Wesselkamp et al. (2025)](https://doi.org/10.1111/2041-210X.70049).

The competitive interpretation is therefore:

- FATES / LPJ-GUESS / other VDMs: match their site, initial state, forcing, observed variables and compute budget; compare stocks, growth, mortality, size distributions, recovery and runtime.
- EFI / NEON: adopt archived forecasts, proper scores, climatology/persistence baselines, site/horizon strata and forecast-limit analysis.
- Earth-observation products: use as observation operators with their own support and uncertainty, not as ground truth for unobserved demographic processes.
- Statistical/ML models: include them as matched prediction baselines; do not infer mechanism correctness from lower stock error.

Luminous has not yet reproduced any competitor or demonstrated a performance advantage.

## 6. Benchmark suite to aim for

### Core matched scorecard

| Axis | Metrics | Acceptance discipline |
|---|---|---|
| Live/deadwood states | MAE, RMSE, bias, P50/P90/worst-site error | Score every census boundary and size/functional stratum |
| Growth and recruitment | Flux MAE/RMSE, bias, distribution / count score where appropriate | Match interval length and cohort definition |
| Mortality | Mortality-rate / hazard error, Brier or log score when probabilistic | Separate size-dependent and disturbance-associated mortality |
| Carbon turnover | Decomposition and export flux error | Distinguish internal transfers from ecosystem-boundary losses |
| Budget closure | Live, deadwood and total-pool residual in physical units | No unexplained residual hidden in a composite score |
| Disturbance / recovery | Recovery-trajectory error, time-to-recovery distribution, return-to-reference threshold | Stratify event type, severity and time since disturbance |
| Forecast skill | CRPS, WIS, Brier/log score and skill versus paired baselines | Archive forecasts before observations arrive |
| Reliability | Point/whole-block coverage, ERT conditional diagnostics, calibration-set stability | Independent unit must match plot/campaign/site dependence |
| Transfer | Leave-site, leave-campaign, leave-region and leave-disturbance-regime performance | Report worst stratum and abstention |
| Cost | Runtime, peak memory, energy and observation cost | Same hardware/input budget; measure, do not infer |

No global scalar should be allowed to trade away an ecological process failure for a good endpoint stock score.

### Research targets, not achieved results

1. Every declared pool closes at every time step within tolerance in the synthetic reference and in admitted empirical runs.
2. Every known compensating-error family is detected in a preregistered blind generator set.
3. Model ranking remains stable under leave-site and leave-region holdouts; report where it does not.
4. Forecasts beat climatology and persistence on preregistered proper scores in at least two independent target strata before making a transfer claim.
5. Report an empirical forecast limit against declared error tolerance using independent verification data.
6. Compare process error against measurement uncertainty; do not use a universal percentage threshold without a power/sample-size basis.
7. Runtime / fidelity trade-offs are published beside ecological errors, not as a single speedup headline.

### What would count as doing better than SOTA?

A credible claim would be narrow and repeatable, for example: on the same sites, initial conditions, forcing, observations, holdout protocol and compute budget, a model has lower growth/mortality/recovery error than the strongest VDM comparator while maintaining stock closure, no worse worst-site coverage, and lower or equal computational cost. Alternatively, the evidence architecture could outperform an existing workflow on provenance completeness, leakage rejection, source-dependence detection, audit time or decision value—each on its own benchmark.

The programme should pursue a *qualification advantage* first: make it materially harder for a model to appear right for the wrong reasons.

## 7. Next steps

1. Exact-head Rust format, Clippy, tests and receipt replay.
2. Independently reproduce all four synthetic cases with a separate arithmetic implementation.
3. Extend the public adversarial generator families for growth/mortality, recruitment/growth, decomposition, disturbance loss and post-disturbance recovery aliasing.
4. Freeze an empirical data contract for repeat censuses and process-flux observations; do not pretend single-census RAINFOR metadata contains longitudinal rates.
5. Admit one rights-cleared site/campaign with matching plot identities and measured/derived flow lineage.
6. Reproduce one open VDM benchmark before training a new model; compare outputs on identical support and forcing.
7. Add probabilistic forecast-limit evaluation after the process trajectories and reference measurements are admitted.
8. Appoint an independent blind custodian before using hidden generator cases for any model-selection or qualification claim.

## 8. Claim ceiling

A synthetic pass verifies only the declared fixture arithmetic, state continuity and disposition logic. It is not a model run, field measurement, hindcast, calibrated ecological process, causal mechanism proof, blind test, SOTA comparison, or permission to intervene in a forest. External data remain not ingested, model reproduction remains not run, and empirical process accuracy remains not established.
