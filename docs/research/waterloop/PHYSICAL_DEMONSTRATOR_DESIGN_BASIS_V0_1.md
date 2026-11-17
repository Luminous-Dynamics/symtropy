# WaterLoop Physical Demonstrator — Design Basis v0.1

**Status:** Research design basis; not a released engineering design  
**Date:** 2026-11-15  
**Parent:** Symtropy issue #5335  
**Product authority:** Symthaea WATERLOOP-APPL-001 #10323  
**Test-cell authority:** Symtropy HABITAT-SERVICE-INTEGRATION-BENCH-001 #4531

## 1. Purpose and claim boundary

Define the smallest useful physical demonstrator for a household water-resilience product family. The first demonstrator is intended to establish measured backup-water service, source separation, leak observability, hydraulic performance, resource accounting and repairability.

This document does not approve construction, procurement, occupied use, public-network connection, potable-water treatment, autonomous chemical dosing, wastewater discharge or regulatory compliance. A qualified local plumbing/electrical/building review and a water-quality risk review are prerequisites before any installation outside an isolated unoccupied test cell.

The first build must not try to prove every WaterLoop capability. It should produce a trustworthy measured baseline and interfaces for later treatment and reuse modules.

## 2. First demonstrator recommendation

Build a **storage-first, three-circuit bench** in the following order:

1. **Backup/service circuit:** a declared clean test-water source, metered fill, isolated storage vessel, gravity/pump delivery branch, and fixture-equivalent demand emulator.
2. **Non-potable recovery circuit:** initially a separate rainwater surrogate or separately prepared non-potable feed, with screening/first-flush diversion represented as distinct stages. Route only to an isolated test outlet or contained irrigation surrogate. Do not connect to the backup circuit.
3. **Residual circuit:** collect and meter overflow, purge, reject and drain streams. No residual may disappear into an unmetered sink.

The initial hydraulic test can be completed with the backup circuit alone. Add the non-potable circuit after physical separation, backflow prevention, instrumentation and residual collection have passed review. Add a treatment skid only after source characterization demonstrates a need and the intended use is fixed.

### Why storage-first

- Backup storage and delivery can be tested without making an unsupported treatment or potable-safety claim.
- It creates a direct comparison with a properly specified tank-and-pump baseline.
- Metered service at the outlet distinguishes gross inventory from water that is actually deliverable.
- Leak detection, power dependency, pump duty, pressure stability and recovery can be tested before treatment complexity is introduced.
- It creates reusable instrumentation and evidence infrastructure for later treatment/reuse experiments.

## 3. Functional architecture

```text
DECLARED TEST SOURCE
        |
   inlet meter + pressure + sample point
        |
  isolation / fill protection
        |
  BACKUP STORAGE MODULE
  level + temperature + inspection + drain
        |
  suction isolation + strainer
        |
  PUMP / GRAVITY DELIVERY ALTERNATIVES
  electrical power meter if pump is used
        |
  delivery pressure + flow + sample point
        |
  FIXTURE-EQUIVALENT LOAD
        |
  measured return / contained drain

SEPARATE NON-POTABLE SOURCE
        |
  source identity + inlet meter + sample point
        |
  screen / first-flush diversion (if roof-rain profile)
        |
  NON-POTABLE STORAGE
        |
  optional treatment module (separately gated)
        |
  qualified-for-declared-test-use outlet
        |
  contained test load / irrigation surrogate

ALL OVERFLOW / PURGE / REJECT / BACKWASH
        |
  dedicated, measured residual collection
```

This is a functional diagram, not a plumbing installation drawing. Physical separation, air gaps, backflow prevention, venting, overflow routing, electrical protection and pipe materials require qualified review against the selected test-cell design and applicable local requirements.

## 4. Physical boundaries and accounting

For each isolated circuit, define an explicit control volume and measurement boundary. The basic storage balance is:

```text
V_initial + ΣV_in
  = V_delivered + V_overflow + V_purge
    + V_reject + V_other_measured_out + V_final
    + V_unaccounted
```

Report the residual:

```text
R_V = V_initial + ΣV_in
      - V_delivered - V_overflow - V_purge
      - V_reject - V_other_measured_out - V_final
```

Do not force R_V to zero. Report meter uncertainty, timing alignment, tank geometry uncertainty, unresolved losses and the resulting uncertainty interval. Aggregate and branch meters must not be summed when they measure the same physical flow.

For contaminant or tracer accounting, preserve mass separately from water volume:

```text
M_in = M_product + M_reject + M_residual
       + ΔM_storage + M_unaccounted
```

A concentration reduction is not necessarily a mass removal; dilution, adsorption, precipitation, biological transformation and unmeasured residual pathways must not be conflated.

## 5. Instrumentation minimum

Every channel requires a channel ID, physical location, measurand, unit, range, resolution, calibration/verification evidence, uncertainty, sampling cadence, clock semantics, missing/stale state and data provenance.

| Boundary | Minimum measurement | Notes |
|---|---|---|
| Source inlet | volume/flow, pressure, sample point | source identity and collection interval |
| Storage | level, temperature, geometry relation | level is not volume without calibration |
| Pump suction | pressure, flow where practical | detect starvation and inlet restriction |
| Pump discharge | pressure, flow | bind to pump operating point |
| Electrical input | voltage/current/power/energy | include controller and standby loads |
| Treatment stage | differential pressure, flow, inlet/outlet sample points | only if a treatment module is admitted |
| Product outlet | pressure, flow, delivered volume, sample point | actual service boundary |
| Residuals | volume and sample identity | overflow, purge, reject and backwash separately |

Turbidity, conductivity, optical or low-cost electrochemical sensors may support process monitoring and anomaly detection. They do not independently establish microbiological safety or potable suitability.

## 6. Architecture alternatives to test

| Alternative | Benefit hypothesis | Principal risks / costs | Decision |
|---|---|---|---|
| Gravity-first storage | low energy, simple failure modes | elevation/footprint, variable pressure, limited delivery envelope | test as reference |
| Pump-assisted storage | controllable outlet pressure and delivery | electrical dependency, pump wear, cycling, noise | test against gravity |
| Hybrid gravity + pump | reserve low-energy service and boost when needed | more valves and failure paths | test only after two baselines |
| Recovery without treatment | simplest source substitution for a constrained non-potable use | source contamination and use restrictions | candidate second stage |
| Recovery with treatment | broader use envelope if validated | energy, consumables, fouling, residuals, monitoring burden | do not build until justified |

No architecture is presumed superior. Compare equal initial conditions, source quality, declared demand, delivery pressure/flow, outage duration and measurement boundaries.

## 7. Hydraulic and energy calculations

For measured flow Q and total dynamic head H, hydraulic power is:

```text
P_hydraulic = ρ g Q H
```

Wire-to-water efficiency over a matched operating window is:

```text
η_ww = ρ g ∫Q(t) H(t) dt / E_electrical
```

Use consistent SI units and synchronized samples. Include auxiliary/control/standby energy in the system boundary. Do not substitute nameplate pump efficiency for measured system efficiency.

For each pump candidate, retain the actual pump curve, impeller/drive configuration, speed range, operating point, suction conditions, NPSH requirements, permitted operating envelope and source provenance. Do not extrapolate outside admitted curve/test data.

## 8. Fault and adversarial test families

At minimum, freeze test vectors for:

- source outage and low inlet pressure;
- demand surge and demand schedule shift;
- tank full, near-empty and unusable residual heel;
- level sensor stuck, biased or disconnected;
- flow sensor stuck while real flow continues;
- pressure sensor normal while outlet flow is unavailable;
- pump degradation, air ingress, suction restriction and power loss;
- valve leakage, stuck valve and unintended cross-connection indication;
- overflow blockage and drain blockage;
- stale source identity or unknown feed;
- treatment bypass, expired/unknown media and filter blockage (when treatment is admitted);
- contradictory sample/monitoring evidence;
- common-mode power, controller or sensor failure;
- maintenance/replacement followed by failed return-to-service verification.

A negative control must prove that an unsafe, stale, unobservable or mismatched state cannot be reported as qualified service.

## 9. Benchmark plan

### Stage A — baseline hydraulic service

- Verify measured delivery volume at the outlet and the declared pressure/flow envelope.
- Record time to service loss and time to recovery under a frozen source outage.
- Measure energy, pump cycling, noise if relevant, maintenance and operator intervention.
- Reconcile source, delivered, residual and final storage volumes with uncertainty.

### Stage B — leak observability

Use controlled, metered leak challenges and no-leak demand shifts. Report event-level precision/recall, detection latency, missed-loss volume, false alarms and false isolation burden. Demand changes, pump/valve transients and sensor drift are mandatory negative controls.

### Stage C — non-potable source substitution

Only credit source substitution when recovered water is metered at the outlet and demonstrably displaces a matched demand that would otherwise have been supplied by mains water. Report source, use class, recovered/delivered/rejected volume and energy separately.

### Stage D — treatment

Admit only after the feed envelope, intended use, analyte panel, analytical methods, independent laboratory, process limits and residual route are defined. Report analyte-specific results with uncertainty. Do not convert a single sample or a process sensor into a universal safety certificate.

### Stage E — repeatability and production variation

Repeat the same frozen protocol across multiple runs and production-representative component lots. Report unit-to-unit variation, drift, calibration burden, service time, parts availability and failure modes.

## 10. Competitive baseline rules

- **JoJo-type tank + pump:** outage continuity baseline. Acquire exact model, usable volume, pump curve, plumbing topology and installed cost. Tank + pump + automatic switchover is not a novelty claim.
- **Hydraloop:** greywater reuse baseline only within exact model, current listing/certification scope, feed, rated capacity and permitted end use. Manufacturer potential throughput, certified rated capacity and measured household delivery are distinct quantities.
- **Moen Flo:** leak observability/containment baseline. Leak alerts and automatic shutoff are not novelty claims.

Never compare a WaterLoop storage result to a greywater treatment capacity, or a pump's instantaneous flow to daily recovered volume.

## 11. Design-for-manufacture requirements

- Modular, inspectable, replaceable subassemblies.
- Accessible isolation and drain points; service without destructive disassembly.
- Traceable wetted materials, seals, filters/media, pump and valve revisions.
- Corrosion, chemical compatibility, temperature, UV and stagnation considerations appropriate to the selected source/use.
- Common service parts where technically appropriate; no false economy from incompatible low-quality fittings.
- Clear assembly sequence, torque/sealing instructions, leak-test steps and post-maintenance verification.
- Repair and replacement must preserve configuration and evidence lineage.
- Preliminary BOM categories only until design review; no procurement commitment in this document.

## 12. Applicable external references and limits

- WHO, *Guidelines for drinking-water quality*, fourth edition incorporating first, second and third addenda (17 June 2026): catchment-to-consumer risk management and independent surveillance. This is guidance, not a product certification.
- South Africa, Government Gazette 52814 / GoN 6292 (6 June 2025), compulsory national water and sanitation services standards. Greywater use beyond a household boundary by a Water Services Institution has specific risk-management provisions; obtain qualified interpretation for the actual deployment.
- South African Water Research Commission, *Guidelines for Greywater Use and Management in South Africa* (2018), as referenced by the 2025 regulations.
- EPA EPANET: external reference model for pressurized distribution networks; use it where its model domain applies rather than silently creating a competing hydraulic solver.
- EPA SWMM: external reference model for runoff, stormwater and sewer collection; use it for the applicable drainage/hydrology domain.
- LeakDB and BattLeDIM/L-Town: useful named leak-detection benchmark populations, but synthetic network scenarios and their native splits/labels must remain distinct from physical testbed and field evidence.

## 13. Open engineering decisions

Do not silently resolve these without evidence:

1. Exact first duty profile and served demand boundary.
2. Test-cell source-water characterization and handling procedure.
3. Tank size, geometry, material and usable residual volume.
4. Gravity elevation and/or pump delivery envelope.
5. Required pressure/flow at the test outlet.
6. Meter ranges, accuracy classes, sampling cadence and calibration route.
7. Electrical supply, protective devices and safe isolation design.
8. Rainwater versus separately prepared greywater feed for the second circuit.
9. Whether treatment provides enough measured benefit to justify its complexity.
10. Exact local professional/code review requirements before any real installation.

## 14. Immediate exit criteria for v0.1

- A requirements-to-test matrix exists.
- A reviewed functional hydraulic schematic and measurement-boundary diagram exist.
- Every physical flow has an owner and a measurement or explicit unknown.
- Initial storage-only test procedure and adversarial cases are frozen.
- A preliminary DFM/BOM-category package exists.
- No unsupported potable, health, code, field-savings or manufacturing-readiness claim is made.
