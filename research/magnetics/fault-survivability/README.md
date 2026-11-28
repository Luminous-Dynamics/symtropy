# Fault-survivable magnetic architecture evaluator

**Programme:** MAG-FAULT-SURVIVABILITY-001  
**Issue:** #7934  
**Revision:** 0.1-draft

## Purpose

Qualify whether an electric machine remains mechanically safe and delivers a preregistered degraded mission after a realistic fault-induced magnetic degradation event.

This is deliberately not a universal demagnetization-percentage gate. It separates:
1. bulk irreversible magnetic flux change;
2. spatially localized magnet-region degradation;
3. structural retention and containment;
4. post-fault machine function and thermal state.

A machine can have measurable irreversible demagnetization yet still deliver a safe degraded mission. Conversely, a small bulk average loss can hide a severely degraded local zone. Both material evidence and system-level post-fault function matter.

## Literature benchmark and maturity

Silva et al. (2025), DOI 10.1049/elp2.70133, experimentally validated a ferrite spoke machine prototype intended to replace a rare-earth motor in an 18-ton heavy-duty vehicle. The paper reports a 260 kW peak-power design, a 1.3 demagnetization safety factor and 2.0 mechanical safety factor as design constraints, and optimization that maximizes torque after a symmetric short-circuit event. The prototype was physically tested across 115 normal operating points; its back-EMF amplitude changed by at most 0.14% across the normal efficiency-map test.

**Evidence boundary:** the short-circuit demagnetization response and post-short-circuit torque are evaluated in the paper's FEM/design workflow; the described physical prototype campaign validates normal operation, not an experimentally executed physical short-circuit campaign. The study is therefore a strong architecture/design comparator, not proof that a physical fault test was performed. The 1.3 and 2.0 factors are study-specific settings, not universal standards.

## Required evidence

The test contract binds component / rotor / magnet lot / assembly revision; rotor geometry and magnetization; preregistered fault waveform, current, speed, magnet temperature, duration and cooling; monotonically ordered exposure trace and event receipts; independent bulk pre/post flux; spatial pre/post flux map for every required magnet zone; retention, containment, crack/deformation and vibration inspection; mechanical stress and safety-factor evidence; and physical post-fault shaft energy, mission completion, torque margin and maximum temperature.

Simulation can explore fault cases and local field mechanisms, but simulation alone cannot claim physical fault survivability.

## Bulk and local maps

The evaluator does not compute a bulk value by averaging local zones. It requires a separate bulk pre/post measurement. The local-zone list remains visible and every preregistered zone must be measured. A missing local region cannot be hidden by a satisfactory bulk measurement.

Flux-loss intervals are reported. They do not independently decide pass/fail percentages. Application-specific structural safety and degraded mission limits are the decision authority.

## Decision outcomes

- **NotComparable:** fault type, current/speed/temperature envelope, geometry, magnet lot, mission, or measurement scope differs from the preregistered contract.
- **Fail:** physical evidence shows containment/retention failure, cracking/deformation, definite mechanical-limit violation, or post-fault mission/torque/thermal hard-limit failure.
- **Indeterminate:** required fault trace, bulk or local map, inspection, method lineage or physical mission evidence is missing or crosses a limit under uncertainty.
- **FaultSurvivable:** every preregistered fault case passes the structural envelope and the post-fault degraded mission.

## Benchmark targets

- 100% of preregistered fault cases represented by physical trace receipts.
- 100% of required magnet zones mapped; no unregistered or duplicated zone IDs.
- Separate bulk and local flux evidence; no unweighted local-zone mean reported as bulk flux.
- 100% trace sequence, fault identity and sensor calibration joins.
- Zero false-safe outcomes in sealed negative controls for containment, mechanical margin, torque margin, thermal margin, exposure scope and missing local maps.
- Post-fault useful energy, torque reserve and temperature all satisfy the predeclared application-specific degraded mission.
- Replicate across magnet lots, builds and operating temperatures before M5 or population-level claims.

These are programme assurance goals, not achieved physical results or universal material limits.

## Integration

- #7890 / #7887: ordered mission profile and degraded duty replay.
- #7911 / #7913: material lot, source and route-specific magnetic assays.
- #7912 / #7916: element-specific virgin intensity, including any Tb/Gd alloying or diffusion input.
- #7918: matched virgin/recovered source-lot cohorts.
- #7802: matched architecture and system boundary.
- #7793: evidence-derived maturity.
- #7870: teardown, recovery and second-life disposition.

## Claim ceiling

The v0.1 fixture uses SyntheticControl evidence throughout and expects Indeterminate. Tests, once executed, validate evaluator semantics only. No Luminous physical fault test, motor safety certification, demagnetization performance, fleet reliability, lifetime prediction or commercial advantage is claimed.
