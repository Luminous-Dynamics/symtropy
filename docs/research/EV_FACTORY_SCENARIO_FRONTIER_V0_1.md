# EV factory scenario-family frontier v0.1

**Benchmark:** `EV-FACTORY-SCENARIO-FRONTIER-001`  
**Related issue:** [#2492 — scenario and parameter uncertainty](https://github.com/Luminous-Dynamics/symtropy/issues/2492)  
**Status:** synthetic sensitivity frontier, not a calibrated factory twin

## Paradigm shift: seed uncertainty is not model uncertainty

The paired 20-seed simulator estimates variability under one fixed input configuration. A narrow confidence interval can be precise for the wrong model if cycle times, product mix, quality, supplier availability, buffer capacity or failure assumptions are misspecified.

This runner mutates a declared base fixture into nine named scenario families, keeps the exact seed list fixed, executes the existing Rust DES binary, then compares station interventions with the paired effect-of-effects evaluator. Every family output gets input and report SHA-256 digests, and the family summaries show whether the nominal intervention choice remains preferred, whether pairwise results become unresolved, and the regret of carrying the nominal selection into a different synthetic regime.

The family cases are **stress bounds**, not claimed industry values or estimated probabilities:
1. `nominal`: unchanged fixture.
2. `supplier-stress`: supplier inter-arrival mean ×1.30 and arrival variability ×1.20.
3. `body-preparation-stress`: body-preparation mean cycle ×1.35.
4. `eol-test-stress`: end-of-line test mean cycle ×1.40.
5. `changeover-small-batches`: changeover duration ×3 at every station and variant batch size reduced to 5.
6. `yield-rework-stress`: good yield reduced by 0.02 at each station, with 0.90 floor.
7. `tight-buffers`: source and inter-station buffer capacities halved, with minimum 1.
8. `downtime-stress`: MTBF halved and MTTR ×1.5 across stations.
9. `mixed-stress`: supplier interval ×1.20, body preparation cycle ×1.15, changeover ×1.5, yield −0.01 and buffers halved.

All families retain the exact base seed catalog and synthetic evidence classification. Same-seed pairing applies within a family. We do not pool different parameter families as if they were interchangeable repeated draws from one real factory.

## CLI and outputs

```sh
cargo fmt --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml -- --check
cargo test --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml
cargo build --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bins

cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_scenario_frontier -- \
  tools/ev-passenger-cost-frontier/fixtures/ev_factory_simulation_v0_1.json \
  ev-factory-scenario-frontier-artifacts
```

The runner creates one fixture, one simulator output, and intervention-ranking reports at practical margins 0.01, 0.05 and 2.5 good units/hour for each family. The summary reports:
- hashes of the exact base fixture, each mutated fixture, and each resulting simulation/ranking output;
- outcome-row and replication counts, explicit seed IDs, and release status;
- point-estimate best station and a robust-best/abstention result for the default 0.05 margin;
- whether each family changes the point-estimate leader from nominal;
- regret from using the nominal robust-best intervention in each family, where a nominal robust selection exists;
- robust-best family counts and unresolved family count, explicitly *not* interpreted as probabilities;
- sensitivity to a smaller and a deliberately large practical-equivalence margin.

The aggregate report is emitted to stdout and to `ev-factory-scenario-frontier-report.json` in the output directory. Source fixture, family inputs, each simulation report, each ranking report and the summary are retained so the analysis can be independently replayed and diffed.

## How to read the frontier

- **Rank stability:** a point-estimate leader that changes under modest fixture perturbations is a fragile recommendation, even if its within-family seed interval is narrow.
- **Unresolved cases:** if a paired effect-of-effects interval does not clear the margin, abstention is a valid result. It is not a software failure.
- **Scenario regret:** the loss in modeled units/hour from carrying the nominal station choice into a different family. This is still a model-relative sensitivity metric; it is not dollars, NPV or a field intervention effect.
- **Margin sensitivity:** if the best choice disappears only when the margin is increased, show that the decision depends on the declared materiality threshold. The threshold itself needs a business/capacity rationale before measured evaluations.
- **Family frequency:** the number of constructed families in which a strategy wins is a descriptive stress-suite count. It is not a probability that a real factory will behave that way because the families are neither sampled nor weighted from an empirical distribution.

## Public research and industry comparison

NIST's July 2026 manufacturing digital-twin report identifies interoperability, VVUQ, cybersecurity and workforce readiness as persistent challenges: https://doi.org/10.6028/NIST.IR.8620. Its October 2026 report on simulation UQ recommends risk-proportionate credibility assessment linked to metrology: https://doi.org/10.6028/NIST.AMS.100-85. This frontier addresses only one component of credibility—input-family sensitivity. It does not validate the model against measured line data.

The 2026 automotive-parts bottleneck study reports ABAP F1=0.89 against expert-labelled shop-floor bottleneck windows, not against model-output regret: https://doi.org/10.3233/ATDE260693. That is a different task, so the frontier does not claim to exceed that reported result.

Ranking-and-selection literature studies how to select the best among a finite set of DES design alternatives without letting stochastic noise dominate the decision: https://experts.illinois.edu/en/publications/discrete-event-simulation-optimization-using-ranking-selection-an/. The current paired intervals are a transparent first step; for a much larger design space, use predeclared selection procedures and account for multiple comparisons rather than choosing from many candidates with unadjusted intervals.

## Current limitations and next gates

- Family parameters are plausible *stress knobs chosen for sensitivity*, not values learned from measured manufacturing data. The full frontier may not contain the actual world.
- It varies inputs, but it does not yet model all uncertainty in topology, shifts, product demand, maintenance teams, power interruptions, part shortages, process drift or labor constraints.
- The DES failure process remains a simplified per-job/station hazard and fixed repair duration; it does not simulate a machine-specific age process or mid-cycle breakdown.
- Model-form uncertainty and measured-trace validation remain open. The release gates stay false regardless of the preferred intervention.
- Before capital guidance, specify the true business objective and constraints (good units, cost/unit, working capital, safety, quality, energy, emissions, lead-time, employment and fairness) and validate them independently.

Everything in the initial frontier is generated from local synthetic fixture mutations. No manufacturing records were collected; no supplier, manufacturer, plant, integrator, landlord or agency was contacted; no quote requested.
