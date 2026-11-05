# Paired Workload Flexibility Trajectory — Known-Truth Fixture

**Fixture class:** synthetic known truth  
**Scope:** two-slot workload deferral with identical service and facility contracts  
**Not:** physical site evidence, battery/thermal flexibility, network deliverability or capacity accreditation

## Known-truth arithmetic

One job requires 10 useful-work units and has release slot 0 and exclusive deadline slot 2. It can execute in either one-hour slot. IT power is 0.5 kW per work unit; incremental cooling power is 0.1 kW per work unit. Fixed background facility input is 100 kW and all other slot contracts are identical.

The baseline executes in slot 0:
- slot 0 import = 100 + (10 × 0.5) + (10 × 0.1) = 106 kW;
- slot 1 import = 100 kW.

The flexible schedule executes the exact same job in slot 1:
- slot 0 import = 100 kW;
- slot 1 import = 106 kW.

Both schedules complete exactly 10 useful-work units inside the same deadline and use the same frozen slot contracts. The resulting signed grid-import reduction is +6 kW in slot 0 and -6 kW in slot 1. Over one-hour intervals:
- gross reduction = 6 kWh;
- rebound = 6 kWh;
- net grid-import energy reduction = 0 kWh.

This demonstrates why dispatch-window-only reporting can overstate the benefit of load shifting: the service-equivalent schedule moves demand; it does not save energy in this fixture.

## What the evaluator guarantees

It independently verifies both schedules with the existing workload schedule verifier, requires identical service contracts, work-unit definitions, ordered job contracts and ordered slot/facility contracts, and computes the complete signed import difference for every slot. Missing or invalid work, deadline violations, slot constraints, job-manifest differences or facility-contract differences fail closed.

The paired schedule digest values are caller supplied and pinned in the receipt; they are not generated or cryptographically verified by the library.

The evaluator does not model dynamic cooling, state of charge, nonlinear power curves, migration, job-arrival uncertainty, network constraints, communication failures or stochastic dispatch availability. Its output is one workload-only time-shift trajectory and should be supplied to the chronological adequacy model only after the same system load and PCC boundary have been reconciled.
