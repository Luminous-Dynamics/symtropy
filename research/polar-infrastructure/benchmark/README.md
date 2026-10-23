# Polar resilience scorer v0

**Evidence ceiling: synthetic-only. Not a physical simulator, site assessment, safety case, or controller certification.**

This small Rust crate scores already-produced time-series outcomes for a deliberately tiny synthetic episode. It does not generate dispatch decisions, model thermodynamics/electrical dynamics, model actual polar weather, or recommend equipment. The four policy traces in the fixture are hand-authored examples for checking metric calculations and hard-gate behavior; they are not benchmark results.

## Paradigm shift: compare delivered services, not nominal asset uptime

A generator, battery, wind turbine, or network can report available while residents still lack heat, potable water, communications, or sanitation. The evaluation unit here is a declared **service contract over an episode**: required service, an owner-specified minimum fraction, a criticality weight, and delivered service at each time step. The scorer reports each policy's outcomes separately and never lets lower fuel use compensate for a service-floor breach.

The service weights and minimum fractions in the fixture are invented for software tests. They are not engineering thresholds. A real study must obtain limits from the accountable system owner and qualified domain reviewers, justify units and measurement boundaries, and use a site-specific design basis.

## Run

From the repository root, with a Rust toolchain and dependency access:

```sh
cargo test --locked --manifest-path research/polar-infrastructure/benchmark/Cargo.toml
cargo run --locked --quiet --manifest-path research/polar-infrastructure/benchmark/Cargo.toml --bin polar-resilience-score -- research/polar-infrastructure/benchmark/fixtures/toy-episode.json
```

The CLI emits a JSON report. Invalid input fails closed: unknown service identifiers, incomplete step series, duplicate policy/service identifiers, negative/non-finite measurements, absent demands, and unsupported evidence claims are errors.

## Metrics

The scorer supports different physical units per service (for example, heat delivery rate in kW, water flow in litres/hour, communications as a normalized service rate) without adding those physical amounts together.

Each step's demand and delivery values are rates in the declared service unit per hour; the scorer integrates each service's rate over the timestep to produce unit-hours (for example, kW × h = kWh, or litres/hour × h = litres). `service_value_semantics` must be `rate_per_hour_integrated_to_unit_hours`.

For service \(s\), let \(D_s\) be total demanded unit-hours and \(S_s\) be delivered unit-hours, both computed only within that service's declared rate unit. The per-service fraction is \(r_s = S_s / D_s\) when \(D_s > 0\). The headline metric is a dimensionless weighted mean of those per-service fractions:

\[
F = \frac{\sum_{s:D_s>0} w_s r_s}{\sum_{s:D_s>0} w_s}
\]

- Delivered values are capped at demand; over-delivery cannot inflate a score.
- An inactive service with zero demand throughout the episode is explicitly excluded from the headline fraction and reports a missing per-service fraction, not a zero.
- Physical demanded/delivered unit-hours are preserved separately per service; unlike units are never summed into one physical total.
- Weighted delivered/unserved service-fraction-hours integrate each active service's dimensionless delivered/unmet fraction over time and criticality weight. These are coverage metrics, not physical energy or water quantities.
- Floor-breach service-hours sum the timestep duration for **each service** below its declared floor; two services breaching in one hour count as two service-hours.
- `gate_pass` is false if any service-floor breach occurs.
- Each policy declares an explicit `fuel_unit`; all policies in an episode must use the same fuel unit, and the ensemble/robust-set evaluators reject unit drift across scenarios. Fuel intensity divides declared fuel units by weighted delivered service-fraction-hours. It is only comparable between runs that share the fuel unit and service contract/weights.
- The scorer fails on non-finite/negative measurements, missing/extra service IDs, duplicate IDs, invalid floors, mismatched series lengths, and unsupported evidence claims.

All outputs remain a vector. The scorer does not collapse critical-service admissibility, coverage, fuel and repair burden into one scalar, does not choose a universal winner, and does not infer a life-safety condition from a service fraction. Floor values and weights are fixture values for software tests, not engineering thresholds.
## Benchmark protocol before making SOTA claims

1. **Freeze the design basis:** geography (Arctic land, Antarctic station, and mobile ice platforms stay distinct), service contracts, units, episode boundaries, data versions, thresholds, missing-data policy, and hard gates.
2. **Pre-register the experiment:** primary outcomes, candidate and baseline families, scenario splits, tuning budgets, compute budgets, retries, exclusions, and uncertainty method before looking at candidate results.
3. **Use fair comparison arms:** documented incumbent; transparent rules baseline; LP/MILP or model-predictive control where a valid model supports it; robust/stochastic optimization where uncertainty is characterized; proposed controller; and ablations. Include defective controls to prove the harness detects known failures.
4. **Hold out conditions:** do not tune on final storm, cold-start, icing, prolonged resupply delay, correlated component failure, communications loss, and sensor-drift episodes. Use only episodes with a defensible data/model basis.
5. **Report a frontier:** critical-service delivery and shortfall; floor breaches; recovery time; fuel and resupply mass; water/sanitation continuity; lifecycle cost/emissions; repair time/spares burden; runtime, memory and platform energy; and uncertainty. Publish per-episode results and tail outcomes, not only an average.
6. **Advance evidence in stages:** scorer/fixture tests → deterministic model tests → calibrated scenario sweeps and sensitivity analyses → software/power/controller HIL as applicable → independently reviewed site data. Synthetic scorer success proves only software behavior.
7. **Fail closed on claims:** queued/skipped checks are not passes. Standards are references for designing tests, not badges of compliance. A standard-conformance claim requires the appropriate suite, a justified test boundary, and qualified review.

For a project-specific test-family crosswalk and explicit limitations, see [IEC TS 62898-3-5:2026 alignment map](../IEC_TS_62898-3-5_ALIGNMENT.md). It is a gap analysis only, not conformance evidence.

Industry anchors for protocol design:
- [IEEE 2030.8-2018: Standard for the Testing of Microgrid Controllers](https://standards.ieee.org/ieee/2030.8/6169/) and the [active P2030.8 revision project](https://standards.ieee.org/ieee/2030.8/12543/).
- [IEC TS 62898-3-5:2026](https://webstore.iec.ch/en/publication/84175), published 2026-06-11, covers hardware-in-the-loop, commissioning, and periodic testing for microgrid monitoring, control, and energy-management systems.
- [NREL/NLR, Hardware-in-the-Loop Test Bed and Test Methodology for Microgrid Controller Evaluation](https://www.nrel.gov/docs/fy20osti/71338.pdf).
- [MLCommons benchmark programme](https://mlcommons.org/benchmarks/) for fair, reproducible comparisons.
- [UFC 3-130-05 Arctic and Subarctic Utilities (2025)](https://www.wbdg.org/FFC/DOD/UFC/ufc_3_130_05_2025.pdf) for cold-region engineering context.

These are method/design references, not evidence that this crate is compliant with them or that any policy is better than state of the art.

Before any real comparison, fill out the [benchmark preregistration template](PREREGISTRATION_TEMPLATE.md) to freeze thresholds, scenario splits, tuning budgets, exclusions, baselines and falsification conditions.

## Probability-free robust stress-set evaluation (synthetic-only)

Not every remote-infrastructure failure mode has defensible occurrence probabilities. The `robust_set` binary is a second evaluation path for an **explicitly enumerated stress set** that reports:
- number of scenarios breaching at least one declared service floor, and whether all enumerated scenarios pass;
- worst-case service shortfall and worst floor-breach duration;
- the unweighted mean over the enumerated stress suite, explicitly **not** an expected-value or probability claim;
- maximum fuel intensity across the included scenarios where defined;
- Pareto non-dominance across breach-scenario count, worst shortfall, worst breach duration and maximum fuel intensity; gate eligibility remains a separate field;
- the full outcome for every scenario so that failures stay inspectable.

Run it with:

```sh
cargo test --locked --manifest-path research/polar-infrastructure/benchmark/Cargo.toml
cargo run --locked --quiet --manifest-path research/polar-infrastructure/benchmark/Cargo.toml --bin robust_set -- research/polar-infrastructure/benchmark/fixtures/toy-robust-set.json
```

The fixture enumerates three toy scenarios without probabilities. The cases and policy deliveries are deliberately fabricated to test the report contract. The policy IDs are shared across every scenario; the scorer rejects changed service contract semantics, duplicate scenario/episode IDs, mismatched policy sets, unknown JSON fields and evidence-claim escalation. Pareto status means only that no other included option is at least as good on every declared objective and strictly better on one; it is **not** a safety approval or winner label.

This model of reporting is consistent with recent reliability research highlighting time-coupled energy adequacy, correlated renewable scarcity and the limits of compressing reliability into single summary indicators. See the 2026 review [Reliability as a design principle: A systematic review and integrated framework for renewable-based microgrids](https://www.sciencedirect.com/science/article/pii/S2590174526003612). For a real study, scenario selection must be justified by site-specific hazards, operating envelopes and stakeholder requirements; do not confuse a finite stress suite with completeness, frequency estimation, or a formal proof of safety.

## Scenario-tail evaluation (synthetic-only)

The companion binary `resilience_ensemble` evaluates **the same declared policy set across multiple episodes** and adds:
- probability-weighted expected service shortfall;
- upper-tail conditional value at risk (CVaR) of service shortfall at a declared confidence level;
- worst-case service shortfall across the included episodes;
- probability of at least one service-floor breach;
- per-scenario service fraction, shortfall, gate outcome and breach duration so aggregate metrics never discard which episode failed;
- expected service-floor breach hours;
- probability-weighted fuel intensity when defined for every included episode.

Run it with:

```sh
cargo test --locked --manifest-path research/polar-infrastructure/benchmark/Cargo.toml
cargo run --locked --quiet --manifest-path research/polar-infrastructure/benchmark/Cargo.toml --bin resilience_ensemble -- research/polar-infrastructure/benchmark/fixtures/toy-ensemble.json
```

The fixture contains only two invented scenarios with artificial probabilities (0.9 and 0.1), just to exercise aggregation and tail calculations. It is not a forecast, an empirical frequency estimate, or an Antarctic/Arctic failure distribution. For a real evidence-backed risk estimate, probabilities must be justified using an explicit data/model basis and uncertainty analysis. If credible probabilities are unavailable, report enumerated scenario outcomes and worst-case results without describing them as event probabilities; CVaR over invented weights is not decision-grade evidence.

The aggregator requires probabilities to sum to one, unique scenario IDs, synthetic-only evidence status, and exactly the same policy IDs/method families in every episode. It does not impute missing scenarios or silently omit failures. CVaR and gate-failure probability are supplementary metrics; neither replaces per-scenario results, hard gates, calibration, or engineering judgment.

Tests check fractional probability mass at the CVaR tail boundary, reject incoherent probability/policy sets and claim escalation, and ensure the adverse synthetic episode increases candidate tail risk while leaving the output explicitly synthetic-only.
