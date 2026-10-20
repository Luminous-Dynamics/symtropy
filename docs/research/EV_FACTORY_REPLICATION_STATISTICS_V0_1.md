# EV factory paired-replication statistics v0.1

**Benchmark:** `EV-FACTORY-SIM-001`  
**Status:** uncertainty-aware synthetic sensitivity analysis; not field-calibrated  
**Implementation:** `tools/ev-passenger-cost-frontier/src/bin/ev_factory_simulation.rs`

## Paradigm shift: from a single demo run to a distribution of outcomes

A single run demonstrates repeatability for one stochastic realization, not whether an intervention is robust. The fixture now declares 20 unique seeds, beginning with the catalog's primary seed. For each seed, the simulator executes the baseline and one-extra-server counterfactual at every station. The common-random-stream branch supplies keyed service-time, quality and failure uniforms plus an isolated supplier-arrival stream, so differences are evaluated as paired outcomes under the same seed.

The output retains the full scenario detail for the primary seed in `scenarios` for debugging and compatibility, and adds `scenario_statistics` over all declared replications. Each scenario summary reports mean, sample standard deviation, min, P05, median, P95 and max for good units/hour, scrap and rework. Each intervention also reports the paired throughput difference and paired relative output difference against that same seed's baseline.

## Confidence intervals

For (n ge 2) observations, the reported 95% confidence interval for the mean paired difference uses the sample standard deviation and a two-sided Student-(t) critical value (tabulated through 30 degrees of freedom, then approximated with 1.96). With (n=1), the interval fields are null rather than pretending there is statistical confidence. Percentiles use linear interpolation between sorted empirical samples.

This is a conventional small-sample summary, not a complete uncertainty quantification framework. The model is stochastic only through its declared synthetic distributions and simplified process assumptions; intervals do not include uncertainty in the fixture, model structure, unobserved failure modes or measurement bias. The 20-run fixture is a baseline demonstration of the machinery, not enough replication for a universal policy or guaranteed tail estimate. Increase seeds and use scenario families after a declared precision/sequential-stopping policy has been designed.

## Read the results correctly

- `scenarios` contains detailed station/buffer diagnostics for the primary seed only.
- `scenario_statistics` aggregates all `replication_seeds`; use these for multi-run summaries.
- `paired_good_units_per_hour_delta_vs_baseline` compares each intervention to its same-seed baseline; its mean CI is the principal first-pass metric for whether a station intervention's synthetic output effect is distinguishable from run variation.
- `paired_relative_output_delta_fraction_vs_baseline` uses a per-seed baseline rate as the denominator. If a baseline rate is zero in a replication, that replication is omitted from relative deltas; the sample count makes such missingness visible.
- The simulator's `release_ready` flag stays false. Even a paired interval entirely above zero is only evidence under the model assumptions, not evidence that a real line will improve.

## Proposed benchmark gates

| Gate | Initial target | Current status |
|---|---|---|
| Reproducibility | Same catalog and seed list => byte-identical JSON and artifact manifest | Tested by CI |
| Pairing | Baseline and each intervention have identical declared seed list and station/job/attempt random keys | Implemented for synthetic DES |
| Replication transparency | Output enumerates all seeds, count, per-scenario samples and quantile definitions | Implemented |
| Paired effect | Report mean paired delta and 95% confidence interval; null interval for one seed | Implemented |
| Model/input uncertainty | Parameter sweeps, distribution uncertainty and calibrated measurement error | Not yet implemented |
| External validation | Temporal holdout from a rights-cleared measured event trace; no tuning on the holdout | No measured trace supplied |
| Safety and legal release | Independent site/product/process approvals | Hard-false release in the software |

These are internal engineering gates rather than universal industry standards. There is no performance comparison to a mature commercial DES engine here; compare only after using the same boundary, inputs, validation holdout and scoring protocol.

## Run

```sh
cargo fmt --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml -- --check
cargo test --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_simulation -- \
  tools/ev-passenger-cost-frontier/fixtures/ev_factory_simulation_v0_1.json
```

## Remaining model limits

Failure timing is still a job/station post-operation hazard followed by a fixed repair duration, not a physical machine-age or mid-cycle-failure model. Supplier flow is bounded inter-arrival variation with an unbounded backlog; correlated supplier stoppages, inbound lead times, order quantities and inventory cash are not included. Product-specific BOM, staffing/shift calendars, tariff/origin, energy prices and safety remain outside this simulator. Calibrate parameter distributions from authorized measured traces, validate against frozen temporal holdouts, and extend the model only with measured evidence and independently tested invariants.

The fixture values and all 20 replications are synthetic. No manufacturing data was requested, no supplier/integrator/plant was contacted and no quote was requested.
