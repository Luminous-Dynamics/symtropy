# Maritime Censoring Robustness — Benchmark Protocol v0.1

**Status:** seeded known-truth synthetic stress benchmark. No port/provider observations are included.

## 1. Research question

When vessel events are incompletely observed, which waiting-time estimates remain valid, which require adjustment, and when is the full-cohort probability not identified from the observed feed?

The generator separates physical event occurrence from source publication and the historical as-of snapshot.

## 2. Known-truth event process

Horizon: 10 synthetic ticks. Calls are deterministically split 50/50 across two observable strata. The event process is a discrete-time competing-risk process with constant per-tick cause probabilities while the call remains event-free.

| Stratum | Berth | Diversion | Cancellation | Total event hazard |
|---|---:|---:|---:|---:|
| 0 | 0.12 | 0.03 | 0.02 | 0.17 |
| 1 | 0.07 | 0.06 | 0.04 | 0.17 |

For stratum (x), cause (k), and horizon (H):

`F_k(H\mid x)=\frac{p_{k,x}}{\sum_j p_{j,x}}\left(1-(1-\sum_j p_{j,x})^H\right)`

The population analytic CIF is the 50/50 standardized sum of stratum CIFs. The synthetic full-cohort empirical CIF is separately computed from all latent outcomes.

PRNG: SplitMix64 with frozen per-scenario seeds in `data/manifest.json`. Latent-event and administrative-censor processes use distinct deterministic streams derived from the scenario seed. Population size: 60,000 calls per scenario. The generator is dependency-free.

## 3. Observation mechanisms

### A. Independent administrative censoring

Administrative observation dropout follows a per-tick Bernoulli hazard of 0.10 in both strata, generated independently of latent event time and cause. Events at the exact same tick as censoring are treated as observed before censoring.

### B. Measured-stratum-dependent censoring

Censoring probability per tick is 0.03 in stratum 0 and 0.22 in stratum 1. The stratum label is observable. Within a stratum, censoring is independent of event time/cause under the synthetic generator. Because the strata have different cause distributions, pooled risk sets can shift as high-censoring strata disappear.

Compare pooled A-J against:
- stratum-specific A-J, standardized back to the declared 50/50 population mix;
- conditional IPCW using the known synthetic (G(t\mid X)) as an oracle reference.

### C. Outcome-dependent reporting lag

Berth outcomes are published four ticks after occurrence; diversions and cancellations are published without delay. A berth event is visible in the cohort only when its publication tick is at or before the observation cutoff. A berth that physically occurred but was published after the cutoff is right-censored at the cutoff in the historical snapshot.

This violates the non-informative censoring assumption because visibility depends on latent cause and event time. The observed-only A-J estimate must not be described as calibrated. The benchmark reports the broad finite-cohort bounds rather than inventing a correction.

### D. Positivity failure

For stratum 1, berth events are delayed by 11 ticks and are never observable within the 10-tick horizon. The full-truth oracle knows these events; the observed estimator does not. Any inverse-probability weight would require division by zero. The stratum's berth risk is NOT_EVALUABLE from the feed at this horizon.

## 4. Estimators

1. **Pooled Aalen–Johansen** from observed records only.
2. **Stratum-standardized A-J**, fitting separate non-parametric curves per observed stratum and averaging them at the declared population shares.
3. **Known-G conditional IPCW incidence reference**, weighting each observed cause event at tick t by (1/G(t\mid X)) and dividing by the full cohort size. This is a synthetic oracle-method comparator; it does not claim to estimate G in a real deployment.

For independent and measured-stratum-dependent censoring, (G(t\mid X)=(1-q_X)^{t-1}) given event-before-censor ties. The minimum horizon support and largest realized weight are reported.

## 5. Non-identification and bounds

For outcome-dependent lag and positivity failure, report the observed cause share as a lower bound and add all unresolved calls for the broad upper bound. This uses the complete frozen cohort frame and assumes each unresolved call can have at most one first competing outcome. It is intentionally conservative; it is not a confidence interval, and we do not label it a sharp bound.

A tighter interval must state the additional dependence model, explain why its sensitivity parameter range is plausible, and validate its coverage on separate known-truth simulations. Model-dependent ranges must not be called identified probabilities.

## 6. Acceptance gates

- Analytic generator CIF matches a separately calculated formula.
- Independent administrative censoring: pooled A-J and known-G IPCW remain inside frozen absolute-error limits against full latent truth.
- Measured stratum dependence: pooled A-J exposes the expected mix bias; standardized A-J and known-G IPCW recover empirical truth within the declared Monte Carlo tolerance.
- Outcome-dependent reporting: naive A-J visibly underestimates berth CIF, status is NOT_IDENTIFIED, and the broad bound contains latent truth.
- Positivity failure: stratum 1 has latent berths but zero observed berths; status is NOT_EVALUABLE and the full truth remains inside the broad bound.
- Event-before-censor tie order is frozen and tested.
- Repeated runs with identical seed reproduce the exact result.

## 7. Fixed-seed preflight results

A separate JavaScript implementation, using the same published generator equations, independent SplitMix64 streams and estimator definitions, produced:

| Scenario | Full empirical berth CIF | Pooled A-J berth CIF | Standardized A-J | Known-G IPCW | Unresolved |
|---|---:|---:|---:|---:|---:|
| Independent admin | 0.47135 | 0.47070 | 0.47097 | 0.47041 | 36.82% |
| Measured-stratum dependent | 0.47125 | 0.50954 | 0.47229 | 0.47133 | 37.67% |
| Outcome-dependent lag | 0.47157 | 0.37450 | 0.37450 | NOT_IDENTIFIED | 25.15% |
| Positivity failure | 0.47325 | 0.29823 | 0.29823 | NOT_EVALUABLE | 33.13% |

Maximum absolute errors across all three cause incidences:
- Independent admin: pooled A-J 0.00151; known-G IPCW 0.00160.
- Measured-stratum dependent: pooled A-J 0.03829; standardized A-J 0.00167; known-G IPCW 0.00137.
- Outcome-dependent lag: pooled A-J berth underestimation 0.09707.
- Positivity failure: pooled A-J berth underestimation 0.17502.

For the outcome-dependent and positivity scenarios, the observed-share-plus-unresolved-fraction bounds contain the full latent CIF in the finite simulated cohort. The bounds are intentionally broad.

The separate JavaScript run is a semantic preflight only. It does not compile or execute the Rust code.

## 8. Same-latent-cohort reporting-delay sweep

Reuse the exact same latent event cohort and latent PRNG stream for berth publication delays of 0, 1, 2, 3, 4, 5, 6, 8 and 10 ticks. Only event visibility changes. This prevents different latent samples from being mistaken for a delay effect.

Independent preflight results:

| Delay | Full-cohort berth CIF | Observed pooled A-J | Underestimation | Unresolved |
|---:|---:|---:|---:|---:|
| 0 | 0.47157 | 0.47157 | 0.00000 | 15.45% |
| 1 | 0.47157 | 0.45358 | 0.01798 | 17.24% |
| 2 | 0.47157 | 0.43190 | 0.03967 | 19.41% |
| 3 | 0.47157 | 0.40523 | 0.06633 | 22.08% |
| 4 | 0.47157 | 0.37450 | 0.09707 | 25.15% |
| 5 | 0.47157 | 0.33730 | 0.13427 | 28.87% |
| 6 | 0.47157 | 0.29147 | 0.18010 | 33.46% |
| 8 | 0.47157 | 0.17267 | 0.29890 | 45.34% |
| 10 | 0.47157 | 0.00000 | 0.47157 | 62.60% |

The Rust tests pin that the latent cohort is unchanged, the berth underestimation is monotonic for these frozen delays, and the four-tick result matches the principal outcome-dependent scenario. These are synthetic mechanism results, not a real source-delay model.

## 9. Interpretation

This simulator demonstrates assumption failure. It does not establish which mechanism applies to Durban. In a real feed, source delay may depend on terminal, outcome, congestion, provider availability, communications loss or data-cleaning rules. Those mechanisms must be empirically diagnosed using event/publication timestamps, source coverage and independent references.

## 10. Industry / methodological baselines

- World Bank / S&P Global CPPI methodology uses AIS/geofence-verified call events and size-stratified comparisons. It reports a 95% match rate in its methodology note. This is a valuable data-validation baseline, not proof that any locally built receiver has the same coverage.
- The 2026 edge-AIS FSM study demonstrates low-cost vendor-independent port-call KPI monitoring but shows sensitivity to dwell/gap parameters; compare exact service-state definitions, validation labels, detection recall/precision, timing error and parameter robustness.
- 2026 dependent-censoring IPCW reviews and competing-risk sensitivity methods are methodological references. The maritime benchmark should reuse the censoring-aware scorecard work in #3763 / PR #3791 and the RUL benchmark #4028 / PR #4035 rather than duplicate a generic survival scoring engine.

## 11. Non-claims

No real port-performance estimate, data-source authentication, causal effect, SOTA claim, terminal capacity validation or operational authorization is produced.
