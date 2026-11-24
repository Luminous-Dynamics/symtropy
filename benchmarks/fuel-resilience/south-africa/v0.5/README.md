# SAFRB v0.5 — fuel-to-useful-energy service accounting

**Status:** standalone research kernel; synthetic fixtures only  
**Research cut:** 2026-11-24  
**Language:** Rust 2024, dependency-free  
**Parent:** SAFRB v0.4 scenario tournament

## Paradigm shift: measure the service, not only the commodity

v0.2 models litres of diesel by grade moving through a fuel network. That is necessary, but a litre in a contract, tank or pipeline does not itself deliver electricity, transport, heat or port throughput.

v0.5 adds an evidence-aware conversion boundary for diesel backup generation:

fuel batch → measured/declared LHV interval → generator efficiency interval → nameplate/run-hour cap → auxiliary loads → net electrical service → priority-qualified demand.

It keeps the commodity ledger (litres by grade and batch) separate from the service ledger (Wh at a named service zone). A contract, fuel receipt or tank gauge is not equivalent to usable generator output.

## Accounting

All quantities are integers:
- Fuel volume: litres (L).
- Fuel energy content: watt-hours per litre (Wh/L), expressed as lower/upper bounds.
- Generator efficiency: basis points (10,000 = 100%).
- Rated/auxiliary power: watts (W).
- Run time: integer hours per day.
- Electrical generation and service demand: watt-hours (Wh).

For each generator run, the model computes:
1. lower/upper chemical input energy from consumed batch litres × batch LHV bounds;
2. lower/upper gross output from chemical energy × efficiency bounds;
3. a nameplate cap (rated W × run hours);
4. auxiliary energy bounds (auxiliary W × run hours);
5. a lower/upper net generation interval.

Net generation is then allocated to service demands within the same declared service zone, by ascending priority. The lower service bound uses conservative energy assumptions; the upper service bound uses optimistic assumptions. Unserved demand is returned as an interval.

## Evidence gates

- Measured custody input requires a structurally SHA-256-shaped digest reference, product quality release and physical accessibility.
- Scenario fuel is accepted only within the explicit `urn:safrb:fixture:` namespace and remains labelled scenario-only.
- Reported, derived and forecast fuel are excluded from firm usable supply in this kernel.
- LHV and generator conversion parameters carry separate evidence references and must independently qualify as measured with a SHA-256-shaped digest reference or as an explicitly namespaced synthetic fixture before a run can calculate electrical output. An otherwise usable fuel batch can remain in the tank inventory if its LHV evidence is unqualified, but a generator run may not consume it for a service calculation.
- v0.5 checks digest shape and evidence class/namespace only. It does not hash raw bytes, authenticate signatures, prove sensor calibration, establish laboratory chain of custody or validate physical truth.
- A generator run that draws from an excluded, not-yet-available, wrong-tank or depleted batch is rejected.

## Deliberate limitations

- No South African terminal, generator or service-zone data are calibrated.
- All executable volumes, LHV bounds, efficiency bounds, generator ratings and demand are synthetic.
- There is no combustion model, thermodynamic cycle solver, load-flow/power-system solver, battery model, fuel-cell model or CHP heat accounting.
- Generator fuel-rate plausibility is not calibrated; the model records fuel use and run hours but does not infer a BSFC curve.
- The supplied generator run schedule is evaluated, not optimized.
- No emissions factor, lifecycle analysis or air-quality result is asserted.
- No PLC/DCS/SIS write path, autonomous dispatch or allocation authority exists.

## Run

```sh
cargo fmt --manifest-path Cargo.toml -- --check
cargo test --manifest-path Cargo.toml --locked
cargo clippy --manifest-path Cargo.toml --all-targets --locked -- -D warnings
cargo run --manifest-path Cargo.toml --release --locked
```

## Competitive comparison and next benchmarks

This crate is not a substitute for validated process or power-system solvers. The benchmark should compare:
- deterministic generator energy-balance calculations against calibrated test-cell / manufacturer performance maps;
- independent power/energy balance and meter reconciliation;
- simple constant-efficiency estimates;
- interval propagation and probabilistic efficiency/LHV models;
- robust service dispatch with storage, grid import, shore power, load shedding and alternative energy carriers;
- PyPSA / power-system LP/MILP scenarios for service scheduling and capacity expansion;
- independent LP/MILP solvers such as HiGHS and Gurobi for optimization, not for measurement truth.

### Proposed qualification targets (not achieved results)

- 100% rejection of invalid or evidence-ineligible fuel draws in the negative suite.
- 0 Wh accounting residual for accepted integer-boundary calculations.
- No optimistic energy claims outside the submitted LHV/efficiency/nameplate envelope.
- Calibrated service interval coverage: 90% empirical coverage for a nominal 90% interval on a frozen test set.
- Generator output prediction error: ≤3% MAPE on a calibrated, within-envelope test set, and separately reported p95 error. This needs a test cell or independently metered field dataset.
- Dispatch tournament: reduce CVaR95 unserved critical-service energy by ≥10% versus a matched deterministic baseline at equal/lower cost, without worsening hard safety/quality constraints. This is a future research gate, not current performance.

## Claim ceiling

A passing Rust workflow qualifies the accounting implementation for synthetic inputs. It does not validate a physical generator, a South African port, national backup-fuel sufficiency, or operational dispatch.
