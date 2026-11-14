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

Pressure traces, signed physical defect flow and laboratory tracer samples are not model outputs. In this T0 fixture, all observation rows are explicitly synthetic_fixture: observation_origin=measurement_channel describes the future channel required, not an acquired measurement; observation_origin=modelled is the negative control. The field raw_data_digest_marker contains a deterministic 64-character synthetic marker, not a raw-data hash. Production interfaces must bind raw_data_digest to the actual SHA-256 of raw bytes, together with common timebase, calibration state, method and custody. Modelled pressure or predicted tracer concentration cannot be relabelled as sensor or lab observations.

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

## 5. Chemical-fate comparators and executable T0

The dated source/paper scope register is [chemical-fate-comparator-register-v0.1.json](chemical-fate-comparator-register-v0.1.json); the executable arithmetic/scope suite is [chemical-fate-t0-cases-v0.1.json](chemical-fate-t0-cases-v0.1.json), with a separate [frozen oracle](chemical-fate-t0-oracle-v0.1.json). The full comparison and qualification protocol is [CHEMICAL_FATE_BENCHMARK_PROTOCOL_V0_1.md](CHEMICAL_FATE_BENCHMARK_PROTOCOL_V0_1.md).

The initial comparator set comprises:
- **EPA EPANET-MSX 2.0**, source-pinned to `85a303dca05859389eced63fabd5ea145c096333`, MIT, with EPANET 2.2 submodule `1e2fc3d909c0df654ead95e6e98c3307ce1375b1`. It supports multiple mobile bulk and immobile wall species, kinetic/equilibrium reaction systems, optional dispersion and OpenMP. It assumes complete instantaneous junction mixing; storage nodes mix completely; wall species do not exist in tanks. Its hydraulic engine is EPANET 2.2, not a water-hammer or pressure-driven defect-ingress model. Its current workflow builds four OS targets and runs a packaged example; its CMake unit-test option is commented out.
- **EPyT-C 1.0.0**, source-pinned to `e86750c27b4da66c752c2916715707904040a7f9`, MIT. It delegates hydraulics to EPANET 2.2 and supplies a separate MOC/RK4 reactive-transport layer with chlorine/THM, bacterial regrowth, arsenic oxidation/attachment and PFOA modules. It has fixed-network regression scripts and a 3-OS × 3-Python-version workflow; this is software regression evidence, not physical transferability of reaction kinetics.
- **EPANET-PMX (2017)**, a paper-only comparator for integrated pressure-driven multispecies analysis. No maintained public source/build/test artefact was verified in this snapshot. Its reported dynamic water-quality scope is not presumed to include high-frequency water hammer, cavitation or column separation.

The 13-case Rust `chemical-fate-t0` evaluator checks first-order decay, sequential A→B→C reactions (including the equal-rate limit), Langmuir bulk/wall equilibrium, conserved-moiety closure, missing-residual handling, missing matrix/temperature/pH, missing kinetic parameters, MSX tank wall-species scope, MSX pressure-driven transient limitations, dispersion requirements, invalid negative parameters, and explicit stoichiometric molecular-weight conversion. Sequential reaction inputs declare a conserved-moiety basis; parent loss is not mislabeled as measured product concentration. The cases independently replay **13/13** against the separate oracle at source level. Fourteen unit tests are authored; native Cargo/rustfmt/Clippy and hosted Actions remain pending.

A successful arithmetic vector is not real chemical qualification. Reaction coefficients must not transfer between waters without matrix, temperature, pH, disinfectant residual, pipe material/surface and independent laboratory evidence. Missing chemistry is not assumed inert; missing streams are not set to zero.

## 6. Benchmark requirements

- exact model/source revisions and stage input/output digests;
- no stage silently receives a different upstream result than the one declared;
- common timebase and explicit clock uncertainty;
- measurement-channel requirements independently identified; actual observations must be provenance-bound when acquired;
- missing chemical/reaction scope is marked unmodelled/indeterminate;
- validity-domain and uncertainty states propagate downstream;
- mass-balance residuals are reported with measurement/model uncertainty; missing streams are unresolved, not zero;
- held-out physical runs and separate model calibration/validation data;
- no automatic potability, notification, isolation, restart, flushing or release authority.

## 7. Metrics and research targets

Proposed research targets (not achieved):
- at least 90% coverage of nominal 90% prediction intervals on held-out physical conditions, with interval width reported;
- zero false-safe outcomes on the frozen positive-ingress / critical-negative corpus;
- time-alignment uncertainty less than one tenth of the shortest event feature the system claims to resolve;
- mass recovery reported with uncertainty, with unresolved residuals preserved;
- accuracy–compute–memory–instrument-cost frontier rather than runtime-only ranking.

## 8. Standards and South African deployment boundary

Sydney Water Guideline D0002125 describes 20–500 pressure samples/s as typical for transient validation and requires event-specific sampling decisions: [guideline](https://www.sydneywater.com.au/content/dam/sydneywater/documents/transient-analysis-guideline.pdf). This range is a comparator, not a universal minimum; bandwidth, sampling, anti-aliasing and sensor spacing must be selected from event features, wave speed, geometry and uncertainty.

South Africa's Government Gazette 52814 (6 June 2025) requires water-services institutions to maintain water safety plans and monitoring programmes under applicable SANS 241, specify sampling points/frequency/parameters, and use ISO 17025-accredited methods or equivalent credible systems: [official Gazette](https://www.dws.gov.za/Projects/wueew/documents/webinfo/20250606_GazettedNor.pdf). The licensed SANS 241:2025 edition must be obtained and interpreted by competent practitioners before any compliance claim. This T0 contract is not a SANS compliance certificate.

## 9. Nonclaims

No component model is qualified by passing this interface contract. No actual contaminant ingress, pathogen presence, chemical toxicity, consumer dose, potability, SANS compliance, restoration or release is established. The executable rejects inconsistent evidence chains; it does not authorize operational actions.