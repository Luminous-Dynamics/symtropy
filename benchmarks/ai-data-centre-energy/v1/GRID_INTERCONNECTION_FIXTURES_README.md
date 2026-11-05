# PCC Grid-Interconnection Assurance — Synthetic Fixtures

**Fixture class:** synthetic known truth  
**Scope:** one declared PCC, piecewise voltage/frequency envelope, breaker continuity, P/Q recovery and measurement quality  
**Not:** operator limits, interconnection approval, IEEE P4200 compliance, PHIL results or site measurements

## Purpose

These fixtures exercise the verifier's contract-driven ride-through and post-fault recovery gates. The numeric limits are invented only to make the arithmetic testable. Production contracts must come from the actual jurisdiction, utility/system operator and signed interconnection agreement.

## Passing trace

The event covers 0–2,000 ms. Voltage and frequency remain within the supplied synthetic band; the breaker stays closed. Baseline active/reactive power is 100 kW / 10 kvar. At event end, P/Q return to baseline and remain there for the full 2,000 ms settling window. The evaluator reports recovery at 0 ms after event clearance.

## Hostile controls

The fixture set includes breaker opening, voltage/frequency outside the supplied ride-through band, delayed recovery, oscillatory recovery, excessive active-power ramp, stale samples, excessive time uncertainty, insufficient sample resolution, gapped ride-through bands, non-monotonic sample timestamps and an incomplete settling tail.

## Sampling and claim ceiling

The contract limits the maximum inter-sample gap to the smaller of the ride-through-band duration and the settling window. If the trace is too coarse, the evaluation fails instead of inferring sub-sample behavior. Even with passing samples, sub-sample transients remain unverified.

The report is limited to the measured PCC and the supplied event contract. It does not simulate electromechanical dynamics, relay coordination, fault-current behavior, harmonics or system-wide stability.

## Validation status

Eleven Rust unit tests are authored. The JSON fixture was parsed and source arithmetic reviewed. No Rust compilation, formatting, Clippy or test execution is claimed in this environment.
