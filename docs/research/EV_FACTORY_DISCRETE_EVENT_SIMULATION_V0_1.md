# EV factory discrete-event simulation v0.1

**Benchmark:** `EV-FACTORY-SIM-001`  
**Status:** research prototype; all fixture parameters are synthetic  
**Reproduces from:** `tools/ev-passenger-cost-frontier/fixtures/ev_factory_simulation_v0_1.json`

## Why this is a paradigm shift

The existing factory and station frontiers estimate static capacity and individual station economics. Real production is coupled: a faster station can just fill a buffer, downstream equipment can block upstream work, upstream shortages can starve downstream work, and variability, changeovers, rework and downtime interact. Discrete-event simulation (DES) makes those interactions explicit instead of treating each station as an independent spreadsheet row.

The runner simulates a line of stations with finite inter-station buffers, variable supplier arrivals, mixed variants, station-specific cycle-time variability, changeover time, imperfect first-pass yield, bounded same-station rework, breakdown/repair events and explicit warm-up exclusion. It reruns the same declared scenario after adding one server to each station, creating an intervention curve rather than naming a bottleneck from utilization alone.

## Run

```sh
cargo fmt --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml -- --check
cargo test --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_simulation -- \
  tools/ev-passenger-cost-frontier/fixtures/ev_factory_simulation_v0_1.json
```

The report is JSON and includes the baseline plus one-server counterfactuals, good output per simulated hour, a simple annualized rate at the declared operating-hours assumption, rework/scrap, peak and average WIP, station time fractions, gate status and conservation/capacity invariants. The event heap orders events by integer time, then by a monotonically increasing sequence for deterministic tie-breaking. A fixed xorshift64* generator makes each run reproducible from the declared seed.

## Metrics to benchmark

| Dimension | Required measure | Initial gate |
|---|---|---|
| Throughput | Measured good units/hour after warm-up; rate uplift from each one-server intervention | Report baseline and each intervention; do not assume added capacity always pays |
| Flow | Average/peak occupancy for every finite buffer; supplier backlog | No buffer capacity exceeded; report full buffer and upstream/downstream interactions |
| Losses | Attempts, rework attempts, scrapped jobs | No output unit without a released input; quality assumptions disclosed |
| Constraint diagnosis | Busy, blocked, starved and down fractions plus marginal output effect of adding one server | Never infer the line constraint from utilization alone |
| Numerical/reproducibility | Identical report for same fixture and seed; integer event time and stable tie order | Determinism test passes; fixture and report get SHA-256 manifest |
| Credibility | Prediction error on a withheld measured production window, distributions and uncertainty intervals | **Not eligible yet:** no measured plant dataset or independent calibration exists |
| Safety/release | Separate local compliance, line safety, product quality and calibration evidence gates | Hard false in this research runner; DES output never authorizes production release |

The first row is intentionally a baseline, not a claimed global state-of-the-art target. Before comparing against a real plant or another DES engine, fix the station/line boundary, product mix, operating calendar, input data, warm-up, replication policy and confidence-interval method. Report average and tail outcomes across multiple seeds after calibration rather than selecting the best run.

## Research comparison and what would make this competitive

- Toyota's production-system description emphasizes Just-in-Time flow and jidoka (detect abnormalities and stop quality escape), not automation for its own sake: <https://global.toyota/en/company/vision-and-philosophy/production-system/>.
- Peer-reviewed manufacturing work uses DES for bottleneck, buffer, reliability, quality and setup analysis, and has shown the importance of injection rate and downstream interactions: <https://doi.org/10.1016/j.promfg.2017.07.033> and <https://doi.org/10.1016/j.procs.2022.01.314>.
- NIST's manufacturing digital-twin work emphasizes interoperability, verification, validation, quantified uncertainty, cybersecurity and workforce readiness: <https://www.nist.gov/publications/digital-twins-workshops-summary-report>.

The differentiator should not be a claim that this small Rust model is already superior to mature DES tools. It is a transparent, version-controlled, open-data benchmark that can be repeated in CI, checked for conservation/capacity invariants, and later attached to evidence-backed station packs. To beat a chosen reference system, run the same calibrated input trace and held-out validation protocol through both, publish the uncertainty and the full scenario fixture, and compare prediction error, runtime, and whether each intervention ranking was correct.

### Explicitly not modeled or validated in v0.1

- Failures are sampled at job completion using a time-based hazard and trigger a fixed repair interval; failures do not interrupt a job mid-cycle, preserve remaining processing time, or model a repair crew shared among machines.
- Supplier delivery variability is represented by bounded inter-arrival variation only. Transit lead time, correlated shortages, minimum order quantities, dock congestion and procurement/working capital costs are omitted; the supplier backlog is unbounded.
- Product variants change in fixed batches. This is not a customer-order schedule, sequenced body shop or full mixed-model assembly optimizer.
- Each station has one independent pass probability; rework repeats at that station and is bounded by the fixture setting. Correlated defects, defect modes, downstream discovery costs and engineering disposition are omitted.
- The annualized output is simulated good units/hour multiplied by declared productive seconds/year. Shifts, meal breaks, holidays, ramp-up and planned shutdown calendars are not simulated.
- The original v0.1 sequential-PRNG implementation allowed event ordering to consume random draws differently between station-server interventions. Follow-on branch `research/ev-factory-common-random-streams-v0-1` introduces entity-keyed station/job/attempt random variates for service times, quality outcomes and post-operation failure uniforms, plus a dedicated supplier-arrival stream. This is CRN variance reduction, not a confidence interval: one fixed seed is still only a screening signal. The job-keyed failure draw is an approximation, not a physical machine-age reliability model. See `docs/research/EV_FACTORY_COMMON_RANDOM_STREAMS_V0_1.md`; use multiple independent seeds and report paired uncertainty before ranking capital interventions.
- Demand, site costs, real product BOM, ergonomic load, skilled workforce constraints, energy, emissions, local law, homologation and battery/HV safety are not inferred from this model.

Do not use this report to purchase automation or declare a plant safe. First record a real station event log, define data rights and privacy controls, calibrate on an explicitly marked training period, and validate on a held-out period. Release stays blocked until an independent, site-specific safety/quality/compliance case exists.
