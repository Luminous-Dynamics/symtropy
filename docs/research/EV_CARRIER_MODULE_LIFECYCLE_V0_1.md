# EV-CARRIER-LIFECYCLE-001 — Module serviceability and whole-life accounting v0.1

**Implementation:** `tools/ev-passenger-cost-frontier/src/carrier_module_lifecycle.rs`  
**CLI:** `ev_carrier_module_lifecycle_screen`  
**Input A:** existing `EV-CARRIER-MODULE-MFG-001` factory report  
**Input B:** lifecycle catalog with source-bound task, frequency and cost observations  
**Claim ceiling:** lifecycle accounting and evidence qualification; not a maintenance authority or production release

## Purpose and boundary

This evaluator extends the vehicle/module manufacturing route with **serviceability, downtime and lifecycle cost accounting**. It consumes the existing module-factory report (site, template, target good vehicles/year, module config, manufacturing capacity and evidence gaps). It does not recalculate station capacity, factory utilization, factory building cost or factory process yield.

It is a benchmark adapter, not a second work-order/maintenance ledger. The generic inspection, isolation, diagnosis, repair, post-test and return-to-service task semantics belong to [AERO-SERVICE-LOOP-001 #4056](https://github.com/Luminous-Dynamics/symtropy/issues/4056). Serialized physical artifact identity, installed configuration and event lineage belong to Mycelix Mobility Commons #3709 and battery-passport #6062/#6076. The responsible authorized people and competent authorities retain return-to-service and regulatory responsibilities.

## Product architectures in the fixture

- Detachable UAV pod: synthetic planned-inspection work content; unplanned event rate unknown.
- AMR handoff module: synthetic planned-inspection work content; unplanned event rate unknown.
- Integrated drone bay: production-intent inspection, task times and event frequencies deliberately unknown.

All three profile/config IDs must match a module configuration in the upstream factory report. The fixture is joined against the existing 36 factory scenarios, producing 108 lifecycle accounting cases. The examples are arithmetic controls, not work-study evidence or a recommendation.

## Task and event model

Each event is either `planned` or `unscheduled`. The evaluator deliberately does not infer a failure rate from a planned maintenance checklist.

### Planned events

Use an explicit annual occurrence count per module-year with a source reference and evidence class. Per-event tasks are multiplied by that planned frequency. A synthetic planned interval supports arithmetic testing only; it is not a measured service schedule.

### Unscheduled events

Unscheduled rates require both:
- a measured or independently tested observed event count;
- an explicit positive module-year exposure denominator.

The rate is `observed_event_count / exposure_module_years`. A measured zero-event observation over a positive exposure is a genuine zero-rate observation. A missing event count or missing exposure remains unknown; it must never be filled with zero. Synthetic unscheduled counts are rejected as failure-rate evidence. Excessive/non-finite annual rates are withheld.

Every unscheduled event also declares a distinct remedy: `repair`, `replace`, `remanufacture` or `repurpose`. Those outcomes are not conflated.

### Service phases and separate time measures

An unscheduled repair/replacement event must account for all eight phases:
1. Safe isolation.
2. Access and disassembly.
3. Diagnose and inspect.
4. Remove.
5. Repair or replace.
6. Reinstall, reconnect and configure.
7. Post-maintenance test.
8. Return-to-service disposition.

For each task, keep separate:
- **elapsed downtime**: wall-clock time during which the module is unavailable;
- **technician minutes**: sum of technician work across all workers;
- crew size;
- parts cost;
- evidence class and source reference.

Crew-hours must not be mistaken for downtime. Concurrent crews, a spare module, task parallelism or a substitute carrier do not create fleet availability in this v0.1 model.

## Whole-life cost boundary

A complete annual per-good-module cost is only calculated when every input below is present:

```text
C_annual =
    acquisition_cost / amortization_years
  + allocated_factory_capex
  + service_parts
  + technician_hours * labour_rate
  + downtime_hours * value_per_unavailable_hour
  + spares_inventory
  + service_logistics
  + module_energy
  + warranty
  + end_of_life_cost
```

This is an **annualized per-module cost boundary**, not a discounted net-present-value calculation. v0.1 does not infer a discount rate, residual value or recycling credit.
## Evidence and release semantics

Evidence references are explicitly namespaced:
- `synthetic://`: arithmetic fixture/control only;
- `unknown://`: required information not supplied;
- other source identities: measured or independent evidence that must be resolved by the responsible verifier.

The prefix is a consistency check, not cryptographic authentication or proof that a measurement is valid. Real source records, article identity, calibration, timestamps, operator competence and independent adjudication remain separate requirements.

A full lifecycle ranking is eligible only if:
- the upstream factory module route is evidence-backed and capacity-qualified;
- all planned and unscheduled event rates and task quantities are known;
- all required cost components and annual operating exposure are known;
- evidence for task times, rates and cost inputs is measured or independently tested;
- all profiles share a comparable currency/year, functional unit and lifecycle boundary.

The evaluator **always** emits `return_to_service_authorized=false` and `module_production_release_ready=false`. It cannot approve maintenance, return a module to service, approve production, generate a battery passport or authorize flight.

## How to run

Generate the existing factory report and the carrier module process report first, then run:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_carrier_module_lifecycle_screen -- \
  ev-carrier-module-factory-report.json \
  tools/ev-passenger-cost-frontier/fixtures/ev_carrier_module_lifecycle_v0_1.json
```

The exact-head workflow runs the evaluator on all 108 factory/module cases. It asserts known synthetic scheduled downtime for the pod and AMR, unknown total downtime and lifecycle cost, an indeterminate integrated bay, no lifecycle ranking, no production release and no return-to-service authority. Inputs, schema, source and output report are retained in the evidence artifact with SHA-256 hashes.

## Qualification benchmarks

Do not set universal time/cost targets before the first measured baseline. First collect comparable physical-article observations and report:
- p50/p90/p95 diagnosis, repair, replacement, reinstall and post-test duration with sample counts;
- technician-hours and elapsed downtime separately;
- planned and unscheduled events per module-year with exposure denominators and confidence intervals;
- repair-vs-replace break-even sensitivity to parts lead time, labor cost and utilization;
- repeat repair, post-test failure, rework and return-to-service rejection rates;
- spare-pool quantity, availability, substitution time and service interruption;
- module removal/installation time across carrier variants and configuration-mismatch/incorrect-serial negative controls;
- lifecycle cost per successful on-time delivery after integration with the matched logistics service benchmark.

Initial engineering acceptance:
- 100% event and task accounting; no silent event or failed-test removal.
- Zero unknown failure rates converted to zero.
- Zero ranking when required inputs, evidence classes, units, cost years or lifecycle boundaries differ or are missing.
- 100% deterministic output under event/task ordering permutations.
- 100% withholding of return-to-service on missing, stale, mismatched or failed post-maintenance evidence.
- Exact agreement with hand-computable synthetic accounting cases.
- Independently measured exposure and cost evidence before a real reliability or cost-superiority claim.

## Research and programme composition

Use [MFB-SOTA-REPRO-001 #6247](https://github.com/Luminous-Dynamics/symtropy/issues/6247) for matched published truck–drone benchmark reproduction; use #6083 for aircraft/carrier time occupancy and #6147 for cargo/load/release semantics. Mycelix remains the physical identity, lineage and custody owner. Symthaea may propose predictive-maintenance priorities and counterfactual strategies, but it cannot authorize maintenance or release an asset. Sol-Atlas supplies sourced regional parts, service-access, route and site context; it does not certify service organizations.

This programme must differentiate integrated, detachable pod, trailer and AMR architectures by measured restoration time, compatible-carrier reuse, module downtime, service burden and total delivered-service cost—not by installation time alone.
