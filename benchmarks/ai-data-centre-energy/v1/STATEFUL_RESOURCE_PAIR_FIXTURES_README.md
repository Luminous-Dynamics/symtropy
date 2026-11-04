# Paired Stateful Resource Trajectory — Known-Truth Fixture

**Fixture class:** synthetic known truth  
**Scope:** two-hour battery dispatch with matched initial contracts and terminal state  
**Not:** workload service equivalence, physical site measurements, grid deliverability or capacity accreditation

## Purpose

This fixture verifies the paired comparison adapter layered on the existing stateful storage/thermal verifier. It checks that a baseline and flexible resource trajectory use the same scenario, PCC boundary, initial storage/thermal contracts, time axis, ambient temperature and non-flexible background load/supply.

## Known-truth battery arithmetic

Both trajectories begin at 90 kWh in a 100 kWh battery. Charge and discharge efficiencies are both 95%.

The flexible trajectory discharges 5 kW for one hour in slot 0. Storage energy falls by:

    5 kWh / 0.95 = 5.263157894737 kWh

To restore the identical initial 90 kWh state, it must charge in slot 1 by:

    5.263157894737 kWh / 0.95 = 5.540166204986 kWh

The baseline grid-import trajectory is [32, 32] kW. The flexible grid-import trajectory is [27, 37.540166204986] kW. The signed reduction is [+5, -5.540166] kW. Over one-hour slots:
- gross reduction = 5 kWh;
- rebound/recharge = 5.540166 kWh;
- net energy reduction = -0.540166 kWh.

So the resource shifts demand away from the first slot but loses energy to round-trip inefficiency. Because the paired terminal SOC is identical, the difference is visible in the modeled horizon rather than hidden as unaccounted energy debt.

The thermal inputs are identical in both trajectories, so their terminal zone temperatures must match. The RC model is the existing first-order single-zone verifier; this fixture does not validate the model against real HVAC telemetry.

## What the pair adapter verifies

- Same scenario, site, PCC measurement boundary and facility/grid limits.
- Identical storage and thermal contracts, including initial conditions.
- Identical time windows, background facility power, other facility power, non-storage onsite supply and ambient temperature.
- Independent physical feasibility of both resource trajectories.
- End-of-horizon storage energy and thermal temperature equality within explicit tolerances.
- Per-slot signed grid import reduction, gross reduction energy and rebound energy.

The adapter pins optional workload-service receipt IDs but does not itself prove workload equivalence. That remains the role of the paired workload schedule verifier/receipt. Caller-supplied configuration digests and receipt IDs are provenance labels; the library does not cryptographically authenticate them.

## Negative controls

The tests reject terminal SOC debt, terminal thermal debt, ambient/weather mismatch, time-window mismatch, initial storage contract mismatch, physical gate failure, missing identity and invalid tolerances. This is a claim gate: no paired trajectory is emitted when either physical path fails or terminal state is not reconciled.

## Validation status

Eight Rust unit tests are authored. The fixture arithmetic has been independently recomputed from the declared efficiencies and power balance. Rust compilation and tests have not been executed in this environment.
