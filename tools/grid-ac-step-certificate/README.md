# Per-step AC operating-point certificate contract v0.1

**Owner:** POWER-AC-STEP-CERT-001 #6331  
**Programme:** #5241 Power Fabric benchmark  
**Related:** #6320 / PR #6326 external comparator contract; #6235 service outcome; #3418 electrical resilience scope.

## Purpose

This package defines a per-time-step receipt contract for an independent AC solver to qualify candidate restoration schedules. It is not an AC solver. It prevents a planner's DC/graph feasibility flag from being mistaken for an independently verified AC operating point.

## Three separate qualification axes

1. **Static AC snapshot:** AC power-flow/OPF convergence and declared voltage, thermal, transformer, P/Q and source constraints for one exact topology and dispatch state.
2. **Inter-temporal constraints:** chronological SOC/fuel, ramping, switching and reserve continuity over the horizon.
3. **Dynamic/protection:** transient/frequency/voltage recovery, inverter control, fault ride-through, protection coordination and switching transients.

A static AC pass does not imply dynamic/protection pass. The accepted fixture intentionally leaves dynamic qualification not assessed.

## Receipt binding

Every step and every topology/dispatch-changing boundary requires exactly one receipt bound to:
- step ID and interval;
- network model revision and parameter-set digest;
- topology-state and dispatch-state digests matching the evaluation grid and planner summary;
- exact solver version/build/configuration;
- canonical input and output artifacts with SHA-256;
- model class and declared applicability;
- convergence residual, tolerance and criteria;
- source reachability and all energized island/slack source records;
- voltage, branch thermal, transformer, generator/inverter P-Q and other applicable constraint assessments.

Missing, stale or mismatched receipts are incomplete/inconclusive, not passes. A solver that fails to converge is inconclusive unless its explicit solver contract classifies a proven infeasibility.

## Synthetic fixtures

- **Accepted static AC only:** two steps with distinct state digests, converged static AC receipts and accepted inter-temporal evidence; dynamic/protection remains not assessed.
- **DC-feasible / AC-infeasible:** planner marks both steps feasible under DC OPF, but the independent AC receipt at step s1 reports no admissible operating point and explicit voltage/thermal failures. The schedule must be rejected.

Values and artifact hashes are illustrative synthetic placeholders; they are not outputs from an electrical solver or utility network.

## SOTA boundary

LARA's inspected public environment source includes graph connectivity/radiality and aggregate MPS P/Q/S checks but no AC/voltage model in that audited source. Lin & Xu (Applied Sciences 2026, DOI 10.3390/app162110584) explicitly evaluate AC operating points and report a DC-only schedule without an AC operating point at 8 of 22 steps on IEEE 118-bus. That makes per-step AC receipts necessary for fair comparison. This contract does not reproduce that paper or validate any network.

## Claim boundary

Contract/fixture only. No AC power flow, OPF, transient simulation, protection coordination, energization, service availability or operational authorization is produced.
