# Power Fabric Dynamic Model Credibility v0.1

**Snapshot:** 16 November 2026  
**Status:** research/engineering contract proposal; not a physical validation result.

Issue: [POWER-FABRIC-DYNAMIC-MODEL-001 #5607](https://github.com/Luminous-Dynamics/symtropy/issues/5607)

## 1. Decision

Power Fabric should not treat a connected module, a declared capability, a converged simulation, or a visually convincing digital twin as proof that the represented equipment is physically qualified.

The missing join is a versioned, independently checkable contract binding:

**physical asset + as-built configuration + model revision + operating envelope + input evidence + validation evidence + output claim**

A model is not simply “validated.” A particular output is validated only for an exact configuration, operating domain, service boundary, reference method and evidence record.

Power-flow feasibility, RMS stability, EMT transient response, frequency-domain impedance stability, local electromagnetic field behaviour and thermal response are different claims. A pass in one lane cannot silently qualify another.

## 2. Why this is timely

Power-electronics-dominated grids are pushing validation across several resolutions. A 19 October 2026 review of grid-forming validation surveys time-domain, RMS, EMT, real-time simulation, CHIL, PHIL, laboratory and field methods; it identifies model fidelity, current/energy limits, multi-vendor interaction and acceptance harmonization as continuing challenges:

- Tan et al., *Review of Testing and Validation Methodologies for Grid-Forming Performance of Inverter-Based Resources*, Energies 19(20), 4966 (2026), https://doi.org/10.3390/en19204966
- NREL / UNIFI, *Specifications for Grid-Forming Inverter-Based Resources, Version 3*, NLR/TP-5D00-98381, https://www.nrel.gov/docs/fy26osti/98381.pdf

A 12 October 2026 review of hybrid EMT/RMS co-simulation and real-time validation describes multi-rate coupling and hardware-in-the-loop as important parts of the emerging workflow:

- https://www.sciencedirect.com/science/article/pii/S0142061526009990

For South African grid connection work, NTCSA publishes software/model requirements that distinguish RMS and EMT studies; model format and software version are part of the study boundary:

- https://www.ntcsa.co.za/grid-code-management/

The implication is not “build one giant Symtropy solver.” It is “make the handoffs between the right specialist models explicit, testable and traceable.”

## 3. Existing owners — compose rather than duplicate

| Concern | Existing owner | This proposal's relationship |
|---|---|---|
| Typed modules and ports | Symtropy #5159 | Bind model profiles to exact typed module revisions and port boundaries |
| Compatibility and service graph | Symtropy #5180 / #5183 | Reject incompatible or stale model-to-module joins |
| Evidence IR and compiler bridge | Symtropy #5206 / #5244 | Preserve claim maturity and exact source identities |
| Benchmark registry and industry comparison | Symtropy #5241 | Add cases to the existing registry; no parallel benchmark catalogue |
| Model calibration, identifiability and uncertainty | Symtropy #3784 | Consume its calibration and uncertainty evidence |
| EM regime selection | Symtropy #3396 | Consume solver-regime decisions; no competing EM selector |
| Grid-forming and microgrid qualification | Symtropy #3418 | Supply the first integrated reference case |
| Energy system twin | Symtropy #3400 | Consume the system-level traces and service boundary |
| GIC and transformer dynamic response | Symtropy #4461 / #4528 | Preserve their quasi-DC and transformer-specific validity boundaries |
| Network import / DC flow / OPF | Sol-Atlas #871 / #874 / #876 / #882 / #888 / #891 / #900 | Consume exact source-bound network evidence; static network results do not qualify dynamic response |
| Search and experiment choice | Symthaea #10119 / #10143 / #10151 and relevant research owners | Generate candidate studies and rank experiments; never self-certify |
| Provenance / authorization | Mycelix identity, supply-chain and manufacturing owners | Bind source, material, asset and review provenance; signatures are not proof of physical truth |

Current Symtropy root workspace inspection (16 November 2026) found the N-dimensional rigid-body core and existing EM/physics research owners, but no dedicated power/electrical domain crate at the checked paths. Keep electrical-network and specialist field solvers outside the rigid-body core.

## 4. Proposed data contracts

### 4.1 PhysicalModelProfile

Minimum fields:

- Profile ID, schema version, owner, model family, solver/tool/adapter version.
- Source/code digest, build configuration, dependencies, licence and provenance.
- Exact target asset/module ID, hardware revision, firmware, parameter set, as-built configuration digest and ServiceGraph revision.
- Physical boundary, topology and port mapping, reference frame, units and sign conventions.
- Formulation: DC/AC power flow, RMS dynamic, EMT, frequency-domain impedance/admittance, circuit/PDN, quasi-static EM, full-wave EM, thermal RC, CFD, electrochemical, structural, etc.
- Operating envelope: frequency, time scale, voltage/current/power, P/Q, temperature, load, grid strength, control mode, SOC and environmental conditions as applicable.
- Numerical envelope: step size, sample interval, coupling step, solver/integrator, tolerance, precision, convergence criteria and solver-health facts.
- Parameter source, calibration ID, uncertainty, assumptions, exclusions and explicit out-of-domain policy.
- Per-output quantity, units, sampling semantics, validity envelope and uncertainty.

No field defaults to “nominal” or “unlimited” when the source is absent. Missing values remain typed UNKNOWN / NOT_APPLICABLE / NOT_EVALUABLE.

### 4.2 ModelValidationReceipt

A validation receipt binds:

- Exact profile, asset/configuration, run and input digests.
- Source identity, rights/provenance, time cut-off and preprocessing digest.
- Independent reference implementation, analytical oracle or measured apparatus identity.
- Test protocol, case, operating point, boundary conditions and exclusions.
- Per-output residuals, uncertainty, interval coverage, calibration state and numerical-convergence evidence.
- Held-out split identity and leakage controls where empirical fitting is involved.
- Negative controls, OOD tests, missing channels and abstention reason.
- Independent review evidence, scope and authority, if review is claimed.

Keep evidence types distinct: ManufacturerDeclared, Modelled, AnalyticallyVerified, ReferenceDifferential, LaboratoryMeasured, HardwareInLoop, FieldObserved and IndependentlyReviewed. These are not a universal maturity ladder. A lab measurement does not automatically validate a model, and a signed review does not replace physical evidence.

Do not emit a single global fidelity score. One model may be valid for steady-state P/Q and invalid for current limiting, harmonic emission or fault recovery.

## 5. Regime map: choose the right physics

| Question | Appropriate model lane | What it does not establish |
|---|---|---|
| Static operating point and dispatch | AC/DC power flow and OPF | Dynamic stability or switching response |
| Slower electromechanical / averaged controls | RMS dynamic simulation | Converter switching detail |
| Fast converter transients, switching and current limits | EMT simulation | Local 3D field fidelity unless explicitly modelled |
| Weak-grid and multi-converter interactions | Frequency-domain impedance/admittance scans plus time-domain tests | Full nonlinear disturbance behaviour from small-signal results alone |
| Core, winding, cable, busbar, enclosure fields | Regime-appropriate FEM/BEM/FDTD or calibrated extraction | Grid-wide service or protection performance |
| Converter/PDN parasitics | SPICE-class circuit models | Complete 3D electromagnetic behaviour |
| Thermal response | Existing thermal RC / CFD owners | Electrical validity outside coupled loss boundary |
| Protection selectivity | Protection-specific studies and qualified tests | Certification from power flow or a generic transient model |

The correct model depends on frequency, geometry-to-wavelength ratio, field mode, material behavior and decision. “EM” is not one solver regime. A full-wave field solve is unnecessary for many low-frequency network questions; a static power-flow result is insufficient for fast converter/protection questions.

## 6. Co-simulation contract

Before implementing a runtime, define:

- Separate clock/timebase, cadence, time step, solver-step and sample/hold semantics for each model.
- Explicit port exchange variables (e.g. voltage/current, P/Q, temperature/heat flow), units and sign convention.
- Coupling algorithm, interpolation/extrapolation limits, event alignment and numerical latency.
- Energy/power balance at every coupling boundary; no hidden energy creation or double counting.
- Initial-condition consistency, restart identity and deterministic replay semantics.
- Step-size and coupling-step refinement checks.
- Stale, missing, contradictory or out-of-domain inputs cause UNKNOWN / NOT_EVALUABLE / REPLAN, not an implicit safe hold.
- Reduced-order/surrogate models bind their training/reference domain and held-out validation evidence.

A co-simulation pass is numerical/interface evidence, not hardware qualification.

## 7. First qualification case: grid-forming inverter + weak-grid equivalent

Start with one small, inspectable topology and progress in fidelity only where the claim requires it.

1. Static AC operating point and P/Q envelope.
2. RMS averaged control/dynamic response.
3. EMT disturbance cases including current limiting and selected fast events.
4. Frequency-domain impedance/admittance scan over a declared band.
5. Multi-inverter / multi-vendor interaction after the single-unit reference is reproduced.
6. Measured/HIL correlation only if a rights-cleared, calibrated test article and qualified facility become available.

Freeze one identical scenario identity across each fidelity tier. Record where the abstractions differ and which outcomes are not comparable. Use NREL/UNIFI and applicable IEEE/utility test profiles for scenario design; derive limits from the exact equipment rating, current standard, grid code, interconnection agreement and test uncertainty. Do not copy one vendor's limits as universal thresholds.

For the South African profile, pin the current NTCSA RMS/EMT model-submission requirements, applicable grid-code edition, model software version, data convention and connection class. A software result is not NTCSA acceptance, SANS conformity, professional sign-off or permission to connect.

## 8. Benchmark plan

### P0 — structural semantics

- 100% of claims bind exact model, asset/configuration, service graph, input and run identities.
- Zero silent default for missing operating-envelope values.
- Zero promotion from DC/AC steady-state flow into RMS/EMT/GFM/field qualification.
- Zero reuse after a material asset, firmware, parameter, topology, calibration or standard-profile change unless dependencies prove requalification is unnecessary.
- 100% rejection of frozen malformed, stale, contradictory and identity-mismatched fixtures.

### P1 — analytical and numerical references

- Hand-calculated one-bus, two-bus and small converter-equivalent cases.
- Unit, sign, base-value, phasor and reference-frame invariants.
- Power/energy balance and passivity where applicable.
- Time-step/coupling-step convergence.
- At least one independent implementation or analytical oracle for every promoted numerical claim.
- Positive controls: valid cases must pass; reject-all is not success.

### P2 — specialist solver differential

- PGLib/MATPOWER cases for the static network lane only.
- Matched RMS/EMT and impedance references for the inverter case where rights-cleared model files are available.
- Per-variable residuals, frequency/phase error, time-domain error, solver tolerance and applicability envelope.
- Compare the producer against an independent reference; two tools sharing the same wrong assumptions do not establish truth.

### P3 — physical correlation

- Calibrated measurement chain, synchronized clocks, as-built topology and firmware/configuration identity.
- Held-out operating points and disturbances; no validation only on calibration points.
- Independent test witness and documented measurement uncertainty.
- HIL, laboratory and field results remain separate evidence classes.

### Required metrics

- Identity/source completeness and unsupported semantics.
- Per-output error and residual distribution.
- Time-domain nadir, overshoot, settling, ROCOF, current-limit duration and recovery.
- Frequency-response magnitude/phase error and stability margin only where defined.
- Energy/power balance, passivity and numerical convergence residual.
- Calibration coverage, sharpness, OOD detection and selective risk versus coverage.
- False-valid, false-invalid and abstention rates.
- Coupling latency, clock error, stale-data exposure and interface residual.
- Runtime/memory after correctness, on pinned hardware.
- Service consequence: essential unserved energy, longest interruption, reserve shortfall and recovery time.

Set numerical tolerances from the applicable test requirement, independently measured reference uncertainty and the engineering decision. Do not set universal physical thresholds in a generic contract.

## 9. How Power Fabric can differentiate from industry

Do not attempt to replace ETAP, PowerFactory, PSCAD, RTDS, MATPOWER, PowerModels.jl, PyPSA, pandapower or OpenDSS across their entire domains.

Run matched bake-offs:
- ETAP / PowerFactory for integrated engineering workflow and equipment/asset libraries.
- PSCAD / PowerFactory EMT / RTDS for transient and real-time model validation.
- MATPOWER / PowerModels.jl / PyPSA / pandapower / OpenDSS for their supported network-analysis problems.
- NREL/UNIFI and relevant IEEE profiles for grid-forming tests.

A credible Luminous advantage would be:
1. Cross-tool semantic integrity: zero silent loss of in-scope model or asset semantics.
2. Exact configuration binding: no validation receipt silently follows a replacement, firmware update or wiring/topology change.
3. Independent two-key computation: solver proposes; a separate verifier reconstructs constraints, balances and outputs.
4. Multi-fidelity continuity: one scenario and asset identity across static, RMS, EMT, frequency-domain, HIL and physical evidence, without mixing claim tiers.
5. Decision-aware uncertainty: acquire the next measurement when it changes a decision; abstain when it cannot be supported.
6. Service-first resilience: optimize useful service and restoration, not only energy, cost or solver objective.
7. Reproducible lifecycle evidence: design -> procurement -> as-built -> commissioning -> operation -> repair -> requalification.

Claim superiority only after a matched model, dataset, hardware/compute budget, formulation and independent acceptance protocol show it. A feature list or synthetic test count is not SOTA evidence.

## 10. Immediate sequence

1. Add schemas and invariants to the existing Power Fabric contract; do not create a duplicate registry.
2. Freeze the small GFM weak-grid reference case and its analytic/negative controls.
3. Bind the exact model profile to existing module identity, ServiceGraph and evidence IR.
4. Add configuration dependency invalidation and build-receipt coverage.
5. Define multi-rate co-simulation semantics; defer runtime until ports and numerical checks are stable.
6. Compare existing solver stacks only on supported, matched formulations.
7. Start physical correlation planning with qualified equipment/lab partners only after the software contract and safe test requirements are reviewed.

## 11. Claim ceiling

This proposal is a contract and research plan. It does not establish that Symtropy currently contains a validated power-flow, RMS, EMT, Maxwell, protection or thermal solver; that any Luminous model is physically calibrated; or that any system is safe, certified, grid-connected or utility accepted.

A software PASS proves only the frozen software semantics and numerical fixtures. Reference differential, HIL, laboratory and field evidence must be earned separately.
