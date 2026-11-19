# Port Nexus Objective-Specific Flexibility v0.1

**Protocol:** PORT-NEXUS-OBJECTIVE-FLEX-001  
**Date:** 19 November 2026  
**Status:** synthetic known-answer fixture and benchmark contract; not a production scheduler.

## Why this increment exists

A study available online on 14 November 2026 (Yang et al., *Advances in Applied Energy*, DOI [10.1016/j.adapen.2026.100322](https://doi.org/10.1016/j.adapen.2026.100322)) analyses 600,245 Alibaba AI jobs and finds that power-intensity and cumulative-energy job populations differ. In its scenarios, independently optimizing time-of-use or carbon signals can create new peaks; a no-new-peak admission rule preserves a substantial share of the objective value.

The result changes our design requirement: workload flexibility must be **objective-specific and aggregate-aware**. A single "percent flexible" number hides which jobs are useful for which grid service and whether moving them creates a new system peak.

Caprara et al.'s earlier LAD-Flex study (IJEPES 178, May 2026, DOI [10.1016/j.ijepes.2026.111940](https://doi.org/10.1016/j.ijepes.2026.111940)) is a separate comparator. Do not combine the two papers' results.

## Fixture

`PORT_NEXUS_NO_NEW_PEAK_FIXTURE_V0_1.json` is a deterministic eight-slot synthetic example:
- four one-hour batch jobs, each 2 kW / 2 kWh;
- fixed residual port/facility PCC import profile;
- different TOU price and carbon-intensity signals;
- objective-only greedy placement versus the same policy with a full-horizon no-new-peak admission cap.

The expected outputs show:
- TOU-only shifting creates a 22 kW peak (+22.22%); the guard caps the trajectory at the original 18 kW and retains 78.57% of the objective improvement.
- Carbon-only shifting creates a 23 kW peak (+27.78%); the guard caps the trajectory at 18 kW and retains 55% of the objective improvement.

These exact figures are solely the output of the small synthetic fixture. They do **not** reproduce the November 2026 paper, represent measured port or workload data, demonstrate a global optimum, or establish achievable savings.

## Independent verifier requirements

The first Rust implementation should be an independent admission verifier, not another scheduler:
1. Verify complete job identity/energy conservation and half-open deadline windows.
2. Reconstruct each candidate's aggregate PCC trajectory from the same frozen baseline.
3. Apply full-horizon peak and time-varying PCC envelope checks.
4. Preserve the objective signal identity/vintage and report objective improvement separately from grid admission.
5. Include rebound/recovery in the evaluation horizon.
6. Abstain when the baseline itself violates the declared cap, the objective denominator is zero/negative, source identity differs, or critical fields are missing.
7. Keep per-site, feeder and system constraints separate; a historical site-peak cap is only the first synthetic guard.

The verifier must not claim voltage/frequency stability, N-1 deliverability, system adequacy, market accreditation, or grid-code compliance. Those require separate network, dynamic, adequacy and operator qualification.

## Benchmark sequence

Compare inflexible baseline, FIFO/EDF, LAD-Flex-style deferral, unconstrained TOU, unconstrained carbon, guarded TOU, guarded carbon, PCC-envelope-aware dispatch and constrained MPC/MILP on identical traces and service obligations.

Later evaluation should use the cited public Alibaba trace (after dataset/version/license verification), and measured H100/B200 traces on held-out hardware. Add South African port loads only when rights-cleared, site-authorized time series and actual PCC boundaries are available.

## Project boundaries

- Symtropy: deterministic replay and independent grid-impact admission.
- Symthaea: proposal generation only; it cannot verify its own schedule.
- Sol-Atlas: spatially resolved port/PCC context and source-rights lineage.
- Mycelix: provenance, permission, approval and evidence lifecycle; DHT publication is not physical delivery evidence.

No live controller or port OT interface is changed by this research fixture.
