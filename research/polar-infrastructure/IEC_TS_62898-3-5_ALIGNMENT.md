# IEC TS 62898-3-5:2026 alignment map (gap analysis only)

**Status:** design guidance, not a clause-by-clause standards assessment and not a declaration of conformity.

Source: [IEC TS 62898-3-5:2026, Microgrids — Part 3-5: Testing for microgrid monitoring, control, and energy management systems](https://webstore.iec.ch/en/publication/84175), published 2026-06-11. The IEC's public description covers hardware-in-the-loop (HIL), commissioning, and periodic testing of microgrid monitoring, control and energy-management systems, for grid-connected and/or isolated microgrids. The full paid document and site-specific engineering requirements have not been reviewed here. See also [NREL/NLR's microgrid-controller HIL methodology](https://www.nrel.gov/docs/fy20osti/71338.pdf).

This is a project-planning crosswalk based on publicly available scope information. Test-family labels below are paraphrased; consult the licensed current standard and qualified engineering reviewers before designing or claiming an actual standards test.

## Test-family crosswalk

| Industry test concern | Relevant Symtropy research elements | What exists now | Required next evidence |
|---|---|---|---|
| Dispatch optimization | polar-night resource deficit; cold battery derating; prime-power trip; fuel delay | scenario definitions and synthetic service scorer | calibrated component/resource/load models; incumbent, rule-based, LP/MILP/MPC and justified robust baselines |
| Forecast performance | wind icing, polar-night demand/supply mismatch, weather and resupply uncertainty | required-input names, not measured forecast data | forecast lead-time definitions, error distributions, calibrated forecast baseline, temporally held-out episodes |
| Demand-side integration / flexibility | service contracts, critical-load continuity, voluntary demand response | toy episode has invented normalized service units | real demand traces with permissions, transparent flexibility rules, rebound/cold safety bounds and fairness constraints |
| Alarm, locking and anti-maloperation | sensor drift, stale telemetry, communications outage, restart failure | catalog labels only | fault injection, stale/conflicting measurement tests, explicit fail-safe state machine and event log |
| Voltage/frequency regulation and power control | generator/inverter loss, low reserves, renewables derating | no electrical dynamics in current scorer | validated electrical model, declared operating envelope, numerical acceptance limits from responsible engineers |
| Device switching and operating-mode transitions | islanding, cold restart, safe transfer, recovery after component fault | no switching/controller model | mode/state transition tests, interlocks, transient limits and negative-control controller |
| Islanding / black start | prime-power trip, communications loss, cold-start/black-start definitions | catalog and synthetic postulated traces | validated startup dependencies, startup load sequence, protection behaviour and appropriate SIL/PIL/CHIL/HIL |
| End-to-end pre-commissioning | service score + scenario-tail aggregation | synthetic JSON-level score/report path is implemented in source | integration of model, controller interface, test orchestration, signal logging, input/output digests and independent rerun |
| Commissioning and periodic re-test | component degradation, delayed spares, seasonal envelope changes | no field acquisition or operational history | site baseline, approved commissioning tests, versioned periodic regression suite, drift and change-management policy |

## The key conceptual distinction

The Rust service scorer is a **system-level outcome layer**. Its service fractions, floor-breach hours, and scenario shortfall/CVaR do not measure voltage excursions, frequency nadir, harmonics, transient response, relay timing, switching behaviour, forecasting accuracy, or real-time determinism. Those require model- and test-family-specific observables. One high-level metric must not substitute for them.

Likewise, tail aggregation is meaningful only when scenario weights have a defensible interpretation. The checked-in two-scenario fixture uses invented probabilities exclusively to verify arithmetic. For a real site, lock the scenario set and thresholds before evaluating candidates; use measured or model-calibrated probability weights only where justified. If such weights cannot be defended, report per-scenario results and worst-case outcomes without labeling them event probabilities.

## Required test maturity ladder

1. **Schema and metric unit tests:** malformed data, missing data, duplicated IDs, non-finite values, wrong time-series lengths, over-delivery, gate-negative controls and CVaR boundary arithmetic.
2. **Deterministic model episodes:** conservation balances and unit tests for thermal/electrical/storage subsystems against an independent reference.
3. **Calibrated scenario model:** fit and validate against permitted site observations; split by time/site/season, not random neighboring samples alone; report parameter uncertainty and sensitivity.
4. **Controller/software-in-the-loop:** exercise actual candidate control logic; lock interfaces, versions and resource budgets; include fault injection and adversarial timing.
5. **Power/controller HIL as appropriate:** test hardware interactions under an approved envelope, with domain-defined electrical limits and testbed timing.
6. **Commissioning / periodic validation:** only qualified site owners can authorize deployment gates; retain actual logs, configuration, deviations, corrective actions and rerun receipts.
7. **Independent claim review:** publish exact commit, model/data digests, scenario exclusions, all gate outcomes, uncertainty, and what remains untested.

## How to claim progress without gaming the benchmark

- Report service-continuity and cost/resource outcomes as a vector; never trade a critical-service gate against a fuel/cost gain.
- Keep known-defective control cases; publish all failures and retries.
- Pre-register primary endpoints, scenarios, tuning budget, resource budget and exclusion rules before seeing candidate results.
- Report median, tail and worst-case results; avoid turning synthetic fixture values into improvement percentages.
- Never label the current catalog or scorer “IEC compliant”, “HIL validated”, “field ready”, or “better than SOTA”. The target is a repeatable validation path, not a badge.
