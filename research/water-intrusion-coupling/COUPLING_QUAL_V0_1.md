# Water Intrusion Model-Coupling Qualification v0.1

**Tracking issue:** [#5192 WATER-INTRUSION-COUPLING-QUAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/5192)  
**Parent physical programme:** [#5145](https://github.com/Luminous-Dynamics/symtropy/issues/5145)  
**Transient solver qualification:** [#5170](https://github.com/Luminous-Dynamics/symtropy/issues/5170)  
**Snapshot:** 14 November 2026  
**Evidence ceiling:** synthetic interface/provenance semantics only

## 1. Objective

Qualify interfaces among separate models of initial hydraulic state, pressure transient, defect ingress, downstream transport/mixing and chemical/biological fate. A result may only pass the interface contract; it does not qualify the component models or prove contamination.

The 2026 review by Raphael, Akpan and Ilemobade (DOI [10.1007/s41101-026-00568-z](https://doi.org/10.1007/s41101-026-00568-z)) explicitly describes a chain in which transient pressure (for example TSNet) feeds a separate intrusion-flow calculation, followed by multispecies transport/reaction (for example EPANET-MSX). It identifies unresolved pipe-material, disinfectant-residual, biofilm and mixing/dilution effects. This is a research architecture, not a ready-made validated end-to-end solver.

## 2. Five-stage chain

1. **initial_state** — network geometry, operating state, demand, reservoirs/tanks, pump/valve states and calibration.
2. **transient_pressure** — pressure/head and supported signed-flow histories, wave-speed derivation/uncertainty, event time, validity domain and numerical convergence.
3. **defect_ingress** — signed reverse flow and contaminant source mass/concentration under the declared external head, defect geometry, material and porous-media conditions.
4. **network_transport** — advection, dispersion, junction/tank mixing and downstream concentration/arrival-time envelopes.
5. **chemical_fate** — only where data support it: bulk/wall reaction, disinfectant residual, sorption, biofilm and transformation products.

Every stage names its model and exact revision, consumed upstream output digests, output digest, timebase, validity domain, uncertainty state and coverage. The dependency graph is explicit: transport consumes both transient pressure and defect-ingress outputs; chemical fate consumes transport output.

## 3. Independent observation plane

Pressure traces, signed physical defect flow and laboratory tracer samples are not model outputs. In this T0 fixture, all observation rows are explicitly synthetic_fixture: observation_origin=measurement_channel describes the future channel required, not an acquired measurement; observation_origin=modelled is the negative control. Repeated 64-character digest values are synthetic_fixture_marker values, not hashes of real raw records or model outputs. Production interfaces must bind actual SHA-256 digests of raw bytes, common timebase, calibration state, method and custody. Modelled pressure or predicted tracer concentration cannot be relabelled as sensor or lab observations.

The timebase contract is shared across the chain. Missing or inconsistent timebase identity is indeterminate, not silently shifted. Event time and clock uncertainty remain explicit. The clock_uncertainty_ms value is a bound/estimate supplied by a traceable synchronization method, not a default.

## 4. Executable T0 interface vectors

The eight synthetic cases exercise:
- a valid five-stage contract with three required measurement-channel types represented by synthetic fixture rows;
- modelled pressure incorrectly offered as a measurement;
- mismatched observation timebase;
- upstream digest mismatch;
- out-of-domain transient stage;
- missing chemical-fate stage (never assume inert chemistry);
- unbounded ingress uncertainty;
- missing raw observation digest.

The evaluator returns CHAIN_CONTRACT_VALID_FOR_BOUNDED_MODEL_COMPARISON only when the interface is internally consistent. The qualification status remains NotRun. This is not a statement that any upstream model is accurate or safe.

## 5. Benchmark requirements

- exact model/source revisions and stage input/output digests;
- no stage silently receives a different upstream result than the one declared;
- common timebase and explicit clock uncertainty;
- measurement-channel requirements independently identified; actual observations must be provenance-bound when acquired;
- missing chemical/reaction scope is marked unmodelled/indeterminate;
- validity-domain and uncertainty states propagate downstream;
- mass-balance residuals are reported with measurement/model uncertainty; missing streams are unresolved, not zero;
- held-out physical runs and separate model calibration/validation data;
- no automatic potability, notification, isolation, restart, flushing or release authority.

## 6. Metrics and research targets

Proposed research targets (not achieved):
- at least 90% coverage of nominal 90% prediction intervals on held-out physical conditions, with interval width reported;
- zero false-safe outcomes on the frozen positive-ingress / critical-negative corpus;
- time-alignment uncertainty less than one tenth of the shortest event feature the system claims to resolve;
- mass recovery reported with uncertainty, with unresolved residuals preserved;
- accuracy–compute–memory–instrument-cost frontier rather than runtime-only ranking.

## 7. Standards and South African deployment boundary

Sydney Water Guideline D0002125 describes 20–500 pressure samples/s as typical for transient validation and requires event-specific sampling decisions: [guideline](https://www.sydneywater.com.au/content/dam/sydneywater/documents/transient-analysis-guideline.pdf). This range is a comparator, not a universal minimum; bandwidth, sampling, anti-aliasing and sensor spacing must be selected from event features, wave speed, geometry and uncertainty.

South Africa's Government Gazette 52814 (6 June 2025) requires water-services institutions to maintain water safety plans and monitoring programmes under applicable SANS 241, specify sampling points/frequency/parameters, and use ISO 17025-accredited methods or equivalent credible systems: [official Gazette](https://www.dws.gov.za/Projects/wueew/documents/webinfo/20250606_GazettedNor.pdf). The licensed SANS 241:2025 edition must be obtained and interpreted by competent practitioners before any compliance claim. This T0 contract is not a SANS compliance certificate.

## 8. Nonclaims

No component model is qualified by passing this interface contract. No actual contaminant ingress, pathogen presence, chemical toxicity, consumer dose, potability, SANS compliance, restoration or release is established. The executable rejects inconsistent evidence chains; it does not authorize operational actions.