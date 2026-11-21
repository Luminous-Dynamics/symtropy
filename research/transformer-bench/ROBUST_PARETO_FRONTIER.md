# PWR-TX-RPF-001 — Robust multi-objective transformer design frontier

**Status:** executable uncertainty-aware screening tool; synthetic fixture only. This is not a transformer design solver, compliance checker, or physical validation result.

## Why this changes the optimization question

A scalar score chooses one candidate by embedding priorities and exchange rates between objectives. A nominal Pareto frontier improves on that by retaining trade-offs, but it can still discard a design because one midpoint estimate appears worse even when the uncertainty bounds overlap materially.

This protocol keeps six engineering objectives separate and classifies candidates using declared lower/upper bounds:

1. lifecycle cost in one declared currency, price year and real/nominal basis;
2. no-load loss (W);
3. load loss (W);
4. maximum hotspot temperature (°C);
5. active-part mass (kg);
6. embodied carbon (kgCO2e) under one declared carbon-boundary identifier.

Every objective is minimized. The tool outputs a conventional midpoint Pareto frontier and a conservative interval-robust frontier. A candidate is removed from the robust frontier only when another candidate is guaranteed no worse across every objective throughout the full declared bounds, and strictly better on at least one. This makes uncertainty visible instead of replacing it with an unqualified point estimate.

## Research context

Transformer design and planning research increasingly combines multiple objectives, operating scenarios and lifetime effects:

- Hashemi and Kilic (2024), [Multi-objective design optimization of hermetically sealed core-type distribution transformer considering current harmonics of power grid using NSGA III](https://doi.org/10.1016/j.jestch.2024.101745), include harmonics and a broad set of design variables in a transformer-specific multi-objective formulation.
- Rijal, Fang and Wang (2026), [Thermal-Aging-Constrained Distribution Transformer Sizing for Demand-Side Resource Integration](https://doi.org/10.1109/OAJPE.2026.3733304), jointly optimize rating and replacement timing with demand-side resources, ambient scenarios and IEEE C57.91 ageing. Their reported case reduces present-value cost by approximately 16.5% relative to a conventional sizing rule.
- Muñoz-Pilco et al. (2026), [Life-Cycle Cost–Optimal Right-Sizing and Replacement Assessment of Distribution Transformers Under Demand Uncertainty](https://doi.org/10.3390/en19081983), connect discrete commercial ratings, investment, losses and replacement timing under uncertain demand growth.

This tool does not reproduce those papers. It provides a transparent frontier contract on which comparable, evidence-backed evaluation can be built.

## Run

```sh
cargo run --locked --manifest-path research/transformer-bench/Cargo.toml --bin robust-frontier -- \
  research/transformer-bench/fixtures/robust-frontier-synthetic.csv \
  /tmp/robust-frontier.csv
```

## Input schema

Exact header:

`candidate_id,profile_id,currency_code,price_year,price_basis,carbon_boundary_id,lifecycle_cost_lower,lifecycle_cost_upper,no_load_loss_w_lower,no_load_loss_w_upper,load_loss_w_lower,load_loss_w_upper,max_hotspot_temp_c_lower,max_hotspot_temp_c_upper,mass_kg_lower,mass_kg_upper,embodied_carbon_kgco2e_lower,embodied_carbon_kgco2e_upper`

All candidates in one input must have exactly the same profile ID, currency code, price year, REAL/NOMINAL basis, and carbon-boundary identifier. Currency validation is syntax-only; ISO registry membership and legal tariff applicability are not verified. Bounds must be finite, non-negative, and ordered. Quoted CSV values are rejected.

The bounds are caller-declared ranges. They are not automatically calibrated prediction intervals, probability distributions, confidence intervals, measurement uncertainty, or guarantees. The tool cannot determine whether they are defensible.

## Dominance definitions

For candidate A and candidate B, with all six objectives minimized:

- **Nominal dominance:** A's midpoint is no worse than B's midpoint on all six objectives and strictly better on at least one.
- **Certain interval dominance:** A's upper bound is no greater than B's lower bound on every objective, and strictly lower on at least one. A robust-frontier candidate is one not certainly dominated by any other candidate.
- **Possible interval dominance:** A's lower bound is no greater than B's upper bound on all objectives and strictly lower on at least one. This is a possibility test over an axis-aligned interval box, not proof that the combination is physically realizable.

The tool treats objective intervals as independent rectangular bounds. If objective uncertainties are correlated, this assumption can be overly conservative: it may retain more candidates than a scenario/covariance-aware analysis. It does not falsely convert a midpoint advantage into guaranteed dominance. The robust frontier is therefore a screening superset pending richer joint uncertainty and hard-constraint analysis.

## Output

The report contains one row per candidate with:

- `nominal_pareto`: not dominated by any midpoint candidate;
- `robust_pareto`: not certainly dominated across all declared interval bounds;
- lists of nominal, certain and possible dominators.

A candidate may be nominally dominated but remain on the robust frontier when uncertainty intervals overlap. The synthetic fixture demonstrates this distinction: candidate C is certainly dominated by A (and other candidates), whereas D is midpoint-dominated by A but remains conservatively robust-nondominated because the bounds overlap.

## Next benchmark gates

These are proposed research acceptance gates, not current results or standards:

1. Reproduce the nominal Pareto set against a pinned independent implementation on randomized test matrices; verify permutation invariance and deterministic output.
2. Add a scenario-aware comparison where objective vectors come from the same joint demand/weather/material scenarios, preserving correlations instead of independent boxes.
3. Measure robust-frontier conservatism: retained candidate count, hypervolume loss only when defensible reference fronts exist, and decision regret under held-out scenario realizations.
4. Compare scalar weighted sums, epsilon-constraint optimization, NSGA-II/III and robust interval/scenario methods under matched objective evaluations and compute budgets.
5. Include a conventional utility sizing/replacement rule as an explicit baseline; where applicable, reproduce a published lifecycle-planning case before claiming improvement.
6. Keep non-negotiable engineering constraints as independent hard gates, not tradeable objectives: insulation, impedance, short-circuit withstand, temperature limits, dielectric clearances, sound, safety, manufacturability and applicable utility/regulatory requirements.
7. Evaluate with traceable FEA/lab/field evidence, group-held-out by geometry, manufacturer, material lot and measurement campaign. Bind exact inputs and outputs with the split and evidence receipt tools.

## Limitations

- No electrical, thermal, mechanical or electromagnetic solver is invoked.
- No test result, Pareto improvement or economic saving is claimed from synthetic fixture data.
- There is no hard-gate qualification logic in this binary.
- Lifecycle cost comparability depends on caller-provided metadata and correct external tariff/FX/inflation calculation.
- Carbon-boundary IDs must be meaningfully defined and consistently applied upstream.
- A robust frontier is not a certified feasible design set or manufacturing recommendation.
