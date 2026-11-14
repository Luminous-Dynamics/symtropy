# Cyber-physical resilience stress-test protocol

**Status:** proposed defensive benchmark protocol. No live controller, operational microgrid, or external network should be targeted. Fault injection belongs in deterministic replay, simulation, software-/processor-/controller-/hardware-in-the-loop (SIL/PIL/CHIL/HIL), or an explicitly isolated lab testbed.

**Current project evidence ceiling:** the Rust scorer evaluates precomputed synthetic service traces. It does not detect attacks, authenticate telemetry, enforce controller policies, or simulate physical dynamics.

## Why add this workstream?

Power resilience should be evaluated across repeated events and changing futures, not just a single severe event. A 2026 review argues that event-by-event resilience alone becomes difficult to interpret as climate risks and their uncertainties accumulate over decades; the planning target should be long-horizon resilience across plausible futures. [Nature Reviews Electrical Engineering (2026)](https://www.nature.com/articles/s44287-026-00322-6)

Renewable-dominated grids also rely on fleets of digitally connected generators and storage. A 2026 perspective highlights how cyber compromise can interact with reduced inertia and physical-control dynamics, making the relevant failure end-to-end: data, decision, actuator, and delivered service—not just network availability. [Nature Reviews Electrical Engineering (2026)](https://www.nature.com/articles/s44287-026-00334-2)

The proposed paradigm shift is to benchmark **the integrity of the whole service-delivery loop under injected faults**. A controller can have healthy network uptime yet make the wrong decision from stale or biased measurements. Conversely, a communications outage need not cause a critical-service outage if safe local fallback operates as intended.

## Lab-only fault families

| Family | Controlled test input | What must be observed |
|---|---|---|
| Stale telemetry | Freeze selected measurements while the simulated plant continues changing | Staleness detection latency, fallback transition, command validity, service degradation |
| Replay / out-of-order telemetry | Replay previously valid samples with old timestamps or wrong sequence order in a test harness | Rejection or explicit stale-state handling; no silent refresh of stale values |
| Sensor bias and drift | Inject a step offset, bounded ramp, stuck-at value, or disagreement between redundant sensors | Detection/fusion behavior, uncertainty flags, degraded-mode trigger, resulting service continuity |
| Communication partition or delay | Add controlled latency, packet loss, and temporary partition | Deadline misses, local autonomy, consistency of mode transitions, restoration behavior |
| Command/actuator mismatch | Simulate an actuator that fails to follow an otherwise valid command | Command-versus-measured-state discrepancy, retry/fallback policy, safe operating envelope |
| In-range but inconsistent setpoints | Inject commands that are syntactically valid but conflict with current measured state, limits, or approved policy | Independent constraint enforcement and safe rejection; no reliance on syntax alone |
| Common-cause dependency | Simultaneously remove supposedly redundant telemetry, control power, shared communications, or shared cooling in the model | Whether redundancy assumptions collapse and critical-service floors are breached |
| Recovery after disruption | Restore communications/sensors after a fault while old buffers or queued commands remain | Re-synchronization, safe state reconciliation, no blind replay of stale queued actions |
| Compound / repeated events | Combine a telemetry fault with fuel delay, inverter loss, cold derating, or prolonged resupply constraints; carry state between events | Residual reserves, fault interactions, time-to-first-floor-breach, longest breach duration, and recovery time |

A test report must identify injected fault, affected signal/component, start time, duration, ground-truth signal, controller-visible signal, active fallback, resulting service trace, and any safety gate outcome. Where a signal is not observable or a model cannot represent the failure, mark it **not covered** with a reason rather than assuming it passed.

## Comparison arms

Use the same physical/model boundary, initial reserve state, forecast access, service contracts, scenario sequence, runtime deadline, and tuning budget for each arm.

1. Documented incumbent with its published/default fallback.
2. Transparent rule-based baseline with explicit staleness, plausibility, and actuator-mismatch guards.
3. Conventional MPC or optimization baseline where the model and input assumptions support a fair comparison.
4. Candidate controller/planner with its proposed integrity checks and fallback.
5. Deliberately defective negative controls: stale values treated as fresh, a stuck sensor trusted indefinitely, or commands accepted without checking current state/limits.

Never let a test that mainly measures a new detector be compared against an incumbent that was deliberately stripped of its normal safety guards. Publish each baseline's version, configuration, guards, and allowed inputs.

## Metrics and proposed gates

These are **research targets to preregister**, not present results or universal engineering limits.

| Metric | Proposed target / reporting rule |
|---|---|
| Unsafe-command emission in the defined fault suite | Zero observed commands violating the test harness's independently encoded constraints; report suite coverage and known omissions |
| Known-invalid/stale evidence handling | 100% rejection or explicit fail-safe/degraded-mode handling for the predeclared negative-control cases |
| Detection and fallback | Report latency distributions and compare against a deadline set by the responsible controls engineer; never invent one universal deadline |
| Service degradation | Report per-service shortfall, critical-floor breach probability when valid probabilities exist, breach-window count, longest consecutive breach, and time to first breach |
| False fallback cost | Report safe-but-unnecessary fallback duration, service loss, extra fuel, operator interventions, and reset/recovery burden |
| Recovery correctness | No unverified stale-state replay after restoration in the specified tests; demonstrate explicit state resynchronization |
| Robustness under compounds | Improve held-out critical-service shortfall without a new service-floor failure against the strongest eligible baseline |
| Reproducibility | Pin exact tested commit, controller/model configuration, fault trace, seed, runner/tool versions, service contracts, and report hashes |

Use the service scorer as a reporting layer after producing a trace from a suitable simulator or lab harness. Keep detection outcomes and physical-service outcomes separate: a detector firing is not itself evidence that the fault was contained, and a valid service trace does not prove authentication or cybersecurity properties.

## Test ladder

- **Contract tests:** malformed, stale, out-of-order, missing, and inconsistent measurements; verify fail-closed behavior and explicit unknown/not-applicable states.
- **Deterministic replay:** fixed input trace, fixed seed and timestamped ground truth; compare each controller's actions and delivered-service trace.
- **Digital-twin sweeps:** calibrated component models, repeated/compound events, sensitivity analysis, and hold-out traces not used in tuning.
- **SIL/PIL/CHIL/HIL as appropriate:** document testbed boundaries, signal fidelity, clocking, physical components and what the test cannot represent.
- **Commissioning/field stage:** only after qualified review, a site-specific hazard basis, staged approval and safe test plan. This protocol alone is not authorization to test live infrastructure.

Use [IEC TS 62898-3-5:2026](https://webstore.iec.ch/en/publication/84175) as one reference for applicable HIL, commissioning, and periodic microgrid controller tests; build an applicability crosswalk rather than claim compliance. [NREL/NLR's HIL microgrid-controller methodology](https://www.nrel.gov/docs/fy20osti/71338.pdf) is another methodology reference. A complementary planning metric, mean survivable outage duration, is described in [McClone, Khurram and Kleissl (IEEE Access, 2025)](https://doi.org/10.1109/ACCESS.2025.3626579); it is useful context, but should not replace per-service floors or compound cyber-physical testing.

## Current limitations and falsification criteria

This protocol is not implemented as an attack detector, physical simulator, or real controller. The existing scorer only accepts deliberately synthetic precomputed traces. Do not claim operational resilience or cyber protection from scorer tests.

The proposed controller advantage is falsified if, on a frozen held-out suite, it produces a new critical-service floor breach, emits an invalid command under a defined negative control, has slower or less reliable fallback than a competent baseline without compensating service benefit, or only wins after tuning on the hold-out set. Publish negative results, not just favorable averages.
