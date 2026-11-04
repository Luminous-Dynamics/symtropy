# Paired Stateful Resource Flexibility Trajectory Protocol

**Protocol:** GRID-STATEFUL-PAIR-001  
**Version:** 0.1  
**Status:** paired battery/thermal/site verifier adapter; not workload or grid accreditation

## 1. Objective

Compose the existing stateful storage and first-order thermal verification primitive with a paired baseline/flexible counterfactual. This is the next link in the evidence chain:

1. Workload schedule verifier establishes the same useful-work obligations.
2. Stateful resource verifier establishes battery reserve/energy, thermal-state and site-power feasibility.
3. Paired adapters establish the signed per-slot PCC import difference and prevent state debt from being hidden beyond the horizon.
4. Grid response verification establishes dispatch delivery/ramp/latency.
5. Repeated event reliability bounds event success within a frozen stratum.
6. Chronological adequacy evaluates the system-level load increment under a pinned scenario set.

Each layer has a separate claim ceiling. Passing one does not imply passing the next.

## 2. Existing physics authority

The adapter intentionally reuses `stateful_resource.rs`, which verifies:
- initial and terminal battery energy trajectory from charge/discharge power and efficiency;
- maximum charge/discharge power, forbidden simultaneous charge/discharge, storage capacity and protected reserve floor;
- critical-load ride-through reserve;
- facility and grid-import limits;
- first-order single-zone RC thermal state, cooling heat-removal/electric power bounds and COP envelope.

The thermal verifier assumes constant ambient temperature, heat injection and cooling extraction within each interval. It is not a CFD, multi-zone, coolant-flow or plant-calibration model. The stateful adapter does not replace the existing state verifier or alter its physical equations.

## 3. Paired counterfactual invariants

A pair is admissible only if baseline and flexible cases have:
- the same scenario ID, site ID and declared PCC measurement boundary;
- identical full storage contracts, including capacity, initial energy, reserve, power limits and efficiency;
- identical full thermal contracts, including initial temperature and RC/COP constraints;
- identical site facility-input and grid-import limits;
- the same number and exact start/end Unix timestamps for every slot;
- identical fixed background facility demand, other facility demand, non-storage onsite supply and ambient temperature per slot.

IT power, cooling electric load/heat extraction and charge/discharge actions may vary because those are candidate flexibility levers. Site identity, PCC boundary, weather, fixed background demand and non-storage supply may not vary within a paired comparison.

Both trajectories are independently passed through `verify_stateful_resource_trajectory`. If either report fails any physical slot gate, the pair is rejected and no flexibility trajectory is returned.

## 4. Terminal-state closure

The final stored energy and final thermal-zone temperature must match within explicit absolute tolerances:

    |SOC_flexible(T) - SOC_baseline(T)| <= ε_SOC
    |T_flexible(T) - T_baseline(T)| <= ε_thermal

This is intentionally stricter than checking that each final state remains above minimum reserve or below maximum temperature. The flexible path cannot borrow battery energy or thermal headroom from beyond the modeled horizon. The current implementation does not permit an unmatched energy/thermal debt with a monetary or future-period adjustment. Future work may support explicit terminal value/continuation models, but that would need independently validated carry-forward constraints.

## 5. Signed PCC trajectory

For each interval:

    R(t) = P_grid,baseline(t) - P_grid,flexible(t)

- Positive R(t): lower grid import in the flexible trajectory.
- Negative R(t): increased grid import (e.g. recharge/rebound).
- Gross reduction energy: sum max(R(t), 0) × Δt.
- Rebound energy: sum max(-R(t), 0) × Δt.
- Net import reduction energy: signed sum R(t) × Δt.

The report returns per-slot baseline/flexible import, signed reduction, gross reduction, rebound, storage energy, thermal state and original evidence IDs.

The same PCC boundary must later be reconciled with the power-system load node in the adequacy model. Do not sum nested facility, UPS, IT, cooling and PCC changes; the net site-boundary signal is the candidate trajectory. Do not count this trajectory again as both battery capacity and flexible workload capacity.

## 6. Known-truth battery cycle

The frozen fixture uses a 100 kWh battery with 90 kWh initial energy, 95% charge efficiency and 95% discharge efficiency. Both paths begin from exactly the same physical initial contract.

Baseline does not dispatch the battery. Flexible dispatch discharges 5 kW for one hour and later charges 5.540166204986 kW for one hour. The first interval reduces import by 5 kW; the second increases import by 5.540166 kW. At equal terminal state:
- gross reduction = 5 kWh;
- recharge/rebound = 5.540166 kWh;
- net import reduction = -0.540166 kWh.

That is a useful result, not a failure of the evaluator: storage can provide time-specific adequacy even though round-trip losses increase net energy use. The adequacy contribution depends on whether the reduction occurs in scarcity hours and the recharge avoids scarcity hours. Cost, carbon, energy and capacity should stay distinct.

## 7. Benchmark programme

These are proposed engineering targets, not regulatory or market standards:
- 100% agreement with independently calculated synthetic storage and thermal transitions.
- Zero accepted trajectories with terminal SOC or thermal-state mismatch beyond the configured tolerance.
- Zero accepted trajectories where either baseline or flexible physical-state report fails.
- Complete signed per-slot trajectory receipts for all scenario intervals.
- Independent replay from frozen inputs reproduces every per-slot grid import and state transition.
- Back-to-back stress-day tests preserve chronological SOC/thermal carry-forward; no daily reset.
- Weather correlation tests retain identical ambient traces between paired arms.
- Common-mode tests include cooling derate, storage inverter failure, site power/telemetry loss and grid unavailability.
- Compare against a validated independent battery/thermal simulator; report model discrepancy, calibration source, step size and uncertainty rather than declaring first-order RC physics universally accurate.
- No production capacity claim until the site boundary is reconciled with a credible nodal grid model and a probabilistic system adequacy study.

## 8. Limitations and next work

The pair adapter does not establish:
- workload completion, deadlines, latency or customer service equivalence by itself; optional service receipt IDs only pin upstream evidence;
- nonlinear battery degradation, cell temperature, C-rate dependence, inverter standby/start time or AC/DC topology beyond the existing simplified energy model;
- multi-zone thermal, evaporative cooling, wet-bulb/water use, plant minimum turndown or cooling COP under transient conditions;
- generator fuel/ramp/start/failure modes;
- migration bandwidth, network constraints, job arrival uncertainty or control/communications failure probability;
- system-wide transmission constraints, unit commitment, economic dispatch, LOLE/EUE scenario calibration or ISO/TSO accreditation.

The highest-value next work:
1. Pair this adapter with the workload schedule receipt so the same work obligations and terminal equipment states are enforced together.
2. Add state carry-forward across multiple weather years and consecutive stress events.
3. Add measured ambient/IT/cooling/charge/discharge trace calibration against physical site telemetry.
4. Model correlated failure modes and dispatch response uncertainty.
5. Map the declared PCC site to a nodal network model and integrate the signed trajectory into chronological adequacy.

## 9. Validation status

Eight unit tests are authored for terminal SOC reconciliation, terminal thermal reconciliation, initial-condition mismatch, time-window mismatch, ambient/weather mismatch, physical-gate failure, identity and tolerance validation, and round-trip efficiency arithmetic. The fixture JSON was independently checked. Rust build, rustfmt, Clippy and test execution must come from an exact-head local or CI run; none is claimed here.
