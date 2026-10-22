# EV factory lifecycle-cost frontier v0.1

**Issue:** [EV-FACTORY-LIFECYCLE-001 #2444](https://github.com/Luminous-Dynamics/symtropy/issues/2444)  
**Implementation:** `tools/ev-passenger-cost-frontier/src/bin/ev_factory_lifecycle_frontier.rs`  
**Input:** `tools/ev-passenger-cost-frontier/fixtures/ev_factory_lifecycle_v0_1.json`  
**Status:** synthetic sensitivity model, not a project budget or factory ROI

## Paradigm shift: a factory is a reconfigurable capability platform

A fixed-layout cost model asks "what does it cost to buy and install this station?" A lifecycle model asks how the resource portfolio performs when the product family changes, volume rises or falls, a product is retired, or equipment can be redeployed. Acquisition route and end-of-life recovery are first-class decisions, alongside station capacity.

The evaluator compares four explicit strategies:
- **Purchase new** — capital acquisition, installation, maintenance, idle carrying cost, retooling and terminal residual value/disposal.
- **Long lease** — installation, fixed-horizon lease payments, maintenance/retooling and terminal exit cost, without assuming the lessee owns salvage.
- **Short-term rental** — rental payment scaled by declared active fraction, setup/return costs and retooling.
- **Refurbish/redeploy** — existing-asset acquisition, refurbishment and installation, with explicit reuse-success and fallback replacement assumptions plus terminal residual/disposal terms.

The strategy names and economics are only categories in this model. No vendor-specific contract or price is implied.

## Model contract and calculations

The catalog is strict JSON with `schema_version=0.1.0`, `benchmark_id=EV-FACTORY-LIFECYCLE-001`, evidence class `synthetic`, an explicit cost unit, annual discount rate, five product-evolution family profiles, and four lifecycle strategies.

All costs are denominated in **synthetic_cost_unit**, not ZAR, EUR or USD. Values are deliberately unanchored sensitivity inputs. Every report includes the SHA-256 digest of the exact input JSON bytes.

For each family and strategy, discounted total cost is:

- upfront acquisition + refurbishment + installation, with expected reuse-failure fallback shown as a separate cost component and counted exactly once;
- discounted annual contract cost using the declared full-horizon or active-fraction payment basis;
- discounted maintenance, scaled by active fraction;
- discounted idle carrying cost, scaled by the idle fraction;
- discounted expected retooling, calculated from product-change events per year;
- discounted terminal disposal cost;
- less terminal salvage value credited only in proportion to declared salvage realisation, redeployability and reuse success.

The **stranded-asset exposure** metric is separate from NPV: expected up-front owned-asset investment multiplied by one minus effective recovery fraction. It is not added a second time to NPV, preventing double-counting of stranded capital.

The family frontier reports NPV by family, equal-weight mean NPV, worst-family NPV, best-family NPV, regret against the cheapest strategy in each family, average/maximum stranded-asset exposure and a three-objective Pareto frontier. A family is a distinct future-product scenario, not an IID replication of one scenario.

The lease-vs-buy calculation searches the first integer tenure where modelled purchase NPV is no higher than modelled long-lease NPV, separately per family. It holds that family’s active fraction, product-change rate and recovery assumptions constant as tenure varies.

## Sensitivity grid

The current deterministic one-at-a-time sensitivity grid includes 11 cases:
- discount rate at 0%, the declared base rate and base +5 percentage points (capped at 50%);
- shorter and longer useful-life horizons;
- no product changes and twice the declared change frequency;
- no idle years and 30 percentage points more idle time;
- zero and full salvage realisation.

The break-even table separately tests lease-vs-buy tenure through each family’s maximum planning horizon. These calculations are sensitivity diagnostics, not a probability distribution over future market states. One-at-a-time perturbations do not capture every interaction; a later version should add factorial/space-filling coverage and adversarial search without tuning against any held-out family.

## Public comparison points

The October 2026 paper *Adaptive robust optimization for sustainable assembly line design under product evolution* studies scenario-based robust resource selection/replacement, column-and-constraint generation, and reuse-oriented options such as leasing and salvaging: https://doi.org/10.1016/j.ijpe.2026.110136. The study reports savings under its own model and cases; those percentages are **not** a target result to claim here.

A 2026 *International Journal of Production Economics* paper studies robust mixed-model line balancing and dynamic task assignment under uncertain product mix: https://doi.org/10.1016/j.ijpe.2025.109756. That is a related production-configuration optimization problem; this small evaluator does not reproduce its MILP/adversarial algorithm or comparable benchmarks.

NIST AMS 100-85 (October 2026) discusses risk-scaled verification, validation, uncertainty quantification and harmonization with metrology for simulation-based decisions: https://doi.org/10.6028/NIST.AMS.100-85. A repeatable cost model is only one stage; inputs and model assumptions still need evidence commensurate with the consequences of the decision.

## Run

```sh
cargo fmt --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml -- --check
cargo test --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_lifecycle_frontier -- \
  tools/ev-passenger-cost-frontier/fixtures/ev_factory_lifecycle_v0_1.json
```

## Decision and evidence limits

- The benchmark is intentionally synthetic and cannot estimate local equipment prices, terms, energy prices, labor cost, tariff treatment, tax effects or financing.
- Active fraction is a coarse annual utilization assumption, not a discrete shift/production calendar.
- Product-change events are modelled as an average annual rate; the current contract does not represent exact dates, downtime, correlated changes or path-dependent retooling.
- Reuse success, redeployability and salvage realization are separate explicit fractions; their uncertainty is not inferred from real resale data.
- Costs are equal-weighted across the five named families for the mean metric. That weighting is a design choice, not an estimate of industry scenario frequency.
- The short-term-rent and lease cash-flow rules are simplified. Contractual deposits, indexation, minimum terms, tax treatment and financing are omitted.
- The cheapest synthetic option is not a buying recommendation. No measured source data or quotes support these values.
- Economic recommendation authorization is false; production release, quality, safety, electrical/battery and jurisdictional compliance remain separate gates.

No supplier, manufacturer, plant, integrator, landlord or agency was contacted and no quote was requested.
