# MFB-SOTA-CICEK-2026-SUMMARY-001 — published truck–drone benchmark summary integrity v0.1

**Evaluator:** `tools/material-flow-benchmark/src/published_summary.rs`  
**CLI:** `mfb-verify-published-summary`  
**Fixture:** `fixtures/cicek_2026_shared_fleet_published_summary_v0_1.json`  
**Programme owner:** [MFB-SOTA-REPRO-001 #6247](https://github.com/Luminous-Dynamics/symtropy/issues/6247)  
**Claim ceiling:** publication-summary arithmetic and provenance consistency only

## Why this exists

Cicek, Simsek and Kantarci (23 June 2026) present an empirically calibrated shared-fleet truck–drone allocation model using three electric trucks and ten DJI Matrice 100 drones. Their Table 8 reports a 474→358 minute makespan change and a 51,338→43,017 Wh total-energy change.

There is a critical reproduction boundary: the paper's Table 3 sets `Battery feasibility enforced` to **0 (not enforced)**. Drone mission energy is estimated and optimized, but the published selected point is not an aircraft battery-capacity-feasibility-qualified result under that flag. It is therefore invalid to describe the headline result as proving battery-feasible drone missions.

This first executable increment prevents the summary from being copied into our benchmark records without the qualifier.

## Published values encoded by the fixture

| Metric | Truck-only | Truck–drone |
|---|---:|---:|
| Makespan | 474 min | 358 min |
| Truck energy | 51,338 Wh | 30,984 Wh |
| Drone energy | 0 Wh | 12,033 Wh |
| Total energy | 51,338 Wh | 43,017 Wh |
| System distance | 218.23 km | 578.80 km |
| Customers served by truck | 456 | 12 |
| Customers served by drone | 0 | 444 |
| Energy per delivery | 112.60 Wh | 94.30 Wh |

The resulting Table 8 changes are 24.5% lower makespan, 39.6% lower truck energy, 16.2% lower combined energy and 165.2% greater system distance. The evaluator recomputes these with integer arithmetic and checks the combined-energy and delivery-count balances.

### Source discrepancy intentionally retained

The paper's Table 8 reports collaborative truck energy as **30,984 Wh**, while the narrative paragraph reports **30,983 Wh**. The fixture records both. The consistency check uses the Table 8 value because 30,984 + 12,033 = 43,017 exactly. It emits a warning for the 1 Wh text/table difference rather than silently normalizing it.

### Empirical-model boundary

The paper uses regression models based on DJI Matrice 100 test data and parameters including payload, speed and horizontal distance. Its stated scenario includes 40 m ascent/descent, a 30 s launch overhead and 10 km maximum radial distance. The paper also states that longer-distance regression samples were extended by proportional scaling to cover 4 km missions; those scaled samples are projections, not new direct flight measurements. The model holds fixed truck route sequences and treats the work as a planning/allocation tool rather than high-frequency dispatch or real-time control.

The fixture does not reconstruct the underlying training regressions, candidate assignments, MILP or raw routes. This is why the CLI is called a **published-summary verifier**, not a reproduction.

## Data licence and rights boundary

The Amazon Last Mile Routing Research Challenge dataset contains 9,184 route instances from 2018 across five US metropolitan areas. AWS identifies the dataset licence as **Creative Commons Attribution-NonCommercial 4.0 International (CC BY-NC 4.0)**.

The fixture records:
- dataset name, year, route count, metro count and licence;
- `raw_dataset_loaded=false`;
- `commercial_use_permitted_by_dataset_license=false`.

Do not commit raw data into the repository, include it in commercial training or use it in a commercial Luminous service under this licence without a separate rights review and permission. The current evaluator consumes only scalar values published in the article, which itself is cited by DOI.

## Reproduction must have two separate tracks

### Track A — paper-faithful

Reproduce the paper's own setup, including the battery-feasibility flag left off. Freeze the paper version, objective weights, route preprocessing, regression version, launch-stop candidates, fleet allocation limits, solver version, time limit, gap and hardware. Reconcile all reported metrics and publish deviations. Do not claim this verifies battery feasibility.

### Track B — battery-feasible extension

Run the same routes and comparable planning objective, but add a separate, explicit mission-energy feasibility boundary:
- conservative energy upper bound for each outbound + return sortie;
- return/abort reserve and unusable energy;
- payload and mass-coupled aircraft energy;
- battery condition, temperature and end-of-life state;
- launch/recovery/handling and turnaround time;
- concurrent aircraft, pad, crew, charger and carrier availability;
- weather and permission as hard gates;
- mission energy at aircraft battery terminals, depot energy and any charging losses separately.

Report how many assignments become infeasible, whether service is moved to the truck or other modes, and the resulting makespan, truck energy, drone energy, total energy, distances and successful deliveries. This must be labelled a **new battery-feasible extension**, never silently substituted as the paper-faithful reproduction.

The MFB shared-fleet oracle's new prequalified mission-envelope input is designed to consume a future qualified output from [MFB-DRONE-CARRIER-COOPT-001 #6083](https://github.com/Luminous-Dynamics/symtropy/issues/6083), not to reimplement aircraft physics or rendezvous.

## Executable integrity rules

The verifier checks:
- exact DOI, title, publication date, source URL and dataset metadata;
- the table's truck + drone energy equals its combined energy;
- truck-served + drone-served customers reconcile to the baseline workload;
- integer-recomputed makespan, truck-energy, total-energy and distance percentages match the paper's one-decimal published values;
- the 1 Wh narrative/table discrepancy is exposed;
- the published battery-feasibility flag is preserved;
- battery feasibility is never claimed by this summary-only verifier;
- raw dataset is not loaded and noncommercial licence is not treated as commercial permission.

The CLI exits nonzero if the summary arithmetic is internally inconsistent. It still does not run the paper's model.

## Run

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml \
  --bin mfb-verify-published-summary -- \
  tools/material-flow-benchmark/fixtures/cicek_2026_shared_fleet_published_summary_v0_1.json
```

The workflow validates the schema/fixture, runs the verifier, pins the arithmetic and non-claim fields, and hashes the verifier source, CLI, schema, input and report.

## What would count as better than the paper

Do not compare only headline makespan and combined Wh. The full tournament in #6247 must report, separately:
- truck kilometres and traction energy;
- drone kilometres, flight time, battery-terminal energy, payload and sortie count;
- successful/on-time, late, failed and unserved deliveries;
- charger/site-meter energy and energy-transfer losses;
- makespan p50/p95/p99, reposition time and energy;
- number of candidate sorties rejected by battery, weather, permission, capacity, rendezvous and turnaround gates;
- exact optimality gap, runtime, memory and hardware;
- out-of-sample demand/wind/weather performance.

Use matched instances and identical objective weights before comparing. The paper's 24.5% makespan and 16.2% total-energy figures are a paper-specific planning result, not a universal target or deployable guarantee.

Luminous' differentiator should be an evidence-qualified whole service network: vehicle/pod design, factory throughput, prequalified aircraft mission envelopes, shared allocation, cargo handling, depot/site energy, maintenance availability and lifecycle costs. Symthaea can propose/learn policies; exact oracles and engineering gates judge them; Mycelix maintains asset/configuration lineage; Sol-Atlas supplies sourced regional context.

## Non-claims

This evaluator does not use the Amazon dataset, fit the drone regressions, reproduce the MILP, establish physical battery feasibility, validate a South African operating area, estimate local demand, prove commercial savings, grant aviation authorization or demonstrate superiority to the published work.
