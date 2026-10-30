# PWR-TX-SCEN-001 — Joint-scenario robust transformer design frontier

**Status:** executable finite-scenario benchmark scaffold. The checked-in scenario fixture is synthetic. No solver, measured transformer or SOTA result is claimed.

## Why joint scenarios instead of independent bounds

The interval-robust frontier tool uses independent lower/upper boxes. This can be conservative because a high-load, hot-weather scenario jointly raises losses and temperature; independent marginal intervals can combine a high temperature from one scenario with a tariff or load value from another that never occurred together.

This protocol preserves the six-objective vector under the same named scenario for each candidate. A scenario may represent a paired joint realization of demand growth, EV/PV/heat-pump adoption, ambient temperature, tariff and material/property assumptions. Candidates are compared on identical scenario IDs and weights. That makes the scenario ensemble—not the independent marginal box—the unit of uncertainty.

## Research context and comparators

- Rijal, Fang and Wang (2026), [Thermal-Aging-Constrained Distribution Transformer Sizing for Demand-Side Resource Integration](https://doi.org/10.1109/OAJPE.2026.3733304), formulate multiyear rating/replacement planning with IEEE C57.91 thermal ageing, heterogeneous EV charging, PV, heat pumps, load growth and ambient scenarios. Their published comparison reports approximately 16.5% lower present-value cost than a conventional sizing rule in their case. This is a contextual baseline, not a reproduced result here.
- Muñoz-Pilco et al. (2026), [Life-Cycle Cost–Optimal Right-Sizing and Replacement Assessment of Distribution Transformers Under Demand Uncertainty](https://doi.org/10.3390/en19081983), connect discrete commercial ratings, losses, investment and replacement timing over demand-growth scenarios.
- Hashemi and Kilic (2024), [harmonic-aware multi-objective distribution-transformer design using NSGA-III](https://doi.org/10.1016/j.jestch.2024.101745), provide a design-space multi-objective comparison that includes current harmonics.

These studies do not validate this implementation. The benchmark goal is to compare approaches using the same traceable design pool, joint scenario set, hard constraints and compute budget.

## Run

```sh
cargo run --locked --manifest-path research/transformer-bench/Cargo.toml --bin scenario-frontier -- \
  research/transformer-bench/fixtures/scenario-frontier-synthetic.csv \
  /tmp/scenario-frontier.csv
```

## Input contract

Exact header:

`candidate_id,profile_id,scenario_id,scenario_weight,currency_code,price_year,price_basis,carbon_boundary_id,lifecycle_cost,no_load_loss_w,load_loss_w,max_hotspot_temp_c,mass_kg,embodied_carbon_kgco2e`

Each candidate must have exactly one row for every scenario. Scenario IDs and weights must be identical for every candidate; weights must lie in (0,1] and sum to 1. Profile, currency, price year, REAL/NOMINAL basis, and carbon-boundary metadata must be consistent across the whole file. Currency validation checks syntax only. All six objective values must be finite and non-negative.

All six objectives are minimized. The scenario table must have been generated using one consistent definition of each metric and the same paired exogenous scenario inputs across all candidates. The tool cannot authenticate scenario construction or establish that a declared weight represents a real-world probability.

## Statistics and frontier definitions

For each candidate and objective:
- **Expected value:** `sum(weight_s * objective_s)`.
- **Worst case:** maximum objective over the finite declared scenario set.
- **CVaR95:** probability-weighted mean of the worst 5% tail of that objective's scenario distribution. Partial scenario weights are used at the tail boundary; no rounding to whole scenarios.

The output separately computes:
- `expected_pareto`: nondominated on the six weighted expected objectives;
- `cvar95_pareto`: nondominated on the six objective-wise CVaR95 values;
- `worst_case_pareto`: nondominated on the six objective-wise worst-case values;
- `scenario_robust_pareto`: a candidate is removed only if another candidate is no worse on all six objectives in every same-named scenario and strictly better at least once.

The scenario-robust relation preserves the joint pairing of the declared finite scenario table. It does not prove robustness outside that set. Expected/CVaR/worst-case objective-wise Pareto sets can also lose cross-objective scenario pairing when summarized; they are decision summaries, not substitutes for scenario-level constraint checks.

## Proposed benchmark gates

These are research targets, not standards or achieved results:

1. Validate scenario-table invariance to row order and candidate order; compare weighted expectations and CVaR95 against independent hand calculations.
2. Compare independent interval-box robust screening with joint-scenario dominance. Quantify how many candidates each approach removes and inspect cases where interval boxes are overly conservative.
3. Compare expected-value optimization, CVaR95 optimization, worst-case optimization, NSGA-II/III, epsilon-constraint and scenario-robust frontiers on identical scenario tables.
4. Evaluate selected designs on a separate frozen stress-test ensemble that was not used for optimization; report lifecycle-cost regret, thermal constraint violations, loss-of-life error and objective-specific regret.
5. Use paired bootstrap over independent scenario-generation campaigns or real data clusters—not only random seeds on one candidate pool. Publish uncertainty on any claimed saving.
6. Report performance across multiple climates, network profiles, tariffs/currencies and transformer classes; retain regional currency, legal/utility profile and carbon-boundary provenance.
7. Keep insulation, dielectric, short-circuit, impedance, temperature, acoustic, fire, environmental and manufacturability requirements as hard independent gates. No economic or carbon benefit can offset a failed safety requirement.
8. Bind input scenario table, profile, candidate manifest, model/solver versions, split, arguments and report digests with the existing evidence receipt.

## Limitations

- The current fixture is synthetic; it is not a measured load/ambient/tariff dataset.
- Scenario weights are caller-provided and not statistically validated.
- The binary does not generate scenarios, run FEA/CFD, compute transformer ageing or optimize candidate geometry.
- Scenario-wise dominance is only valid over the finite scenario set provided.
- CVaR95 is calculated separately per objective; it is not a multivariate risk measure and does not replace joint chance constraints.
- There is no hard engineering qualification logic in this binary.
- A Pareto candidate is not thereby safe, compliant, manufacturable or economically optimal in the field.
