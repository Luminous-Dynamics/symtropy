# Fault-survivable magnetic architecture qualification protocol

**Programme:** MAG-FAULT-SURVIVABILITY-001  
**Issue:** [Symtropy #7934](https://github.com/Luminous-Dynamics/symtropy/issues/7934)  
**Revision:** 0.1-draft  
**Date:** 28 November 2026

## Decision question

After a preregistered electrical, thermal, mechanical or cooling fault, does the machine retain its magnets and rotor safely and deliver the required degraded useful service?

The benchmark qualifies the post-fault operating state. It does not assume zero irreversible demagnetization is always the optimal engineering requirement, nor that an average flux-loss number is sufficient.

## Literature baseline

Silva et al., *IET Electric Power Applications* 19(1), e70133 (first published 6 December 2025), DOI 10.1049/elp2.70133, designed and tested a ferrite spoke machine as a same-envelope substitute for an existing rare-earth heavy-duty traction motor. The paper:
- uses a 260 kW peak-power design for an 18-ton vehicle;
- formulates optimization to maximize torque after a symmetric short-circuit event;
- defines demagnetization safety factor as minimum temperature-dependent intrinsic coercivity divided by bulk PM field magnitude;
- uses study-specific design constraints of 1.3 demagnetization safety factor and 2.0 mechanical safety factor;
- reports a ferrite prototype tested across 115 normal operating points, with no permanent demagnetization observed over the normal efficiency-map campaign and a maximum back-EMF amplitude decrease of 0.14%.

**Crucial evidence distinction:** the paper's fault-induced short-circuit demagnetization and post-fault torque are evaluated in the FEM/design workflow; the described physical prototype campaign validates normal operation, not a physically executed short-circuit test. The 1.3 and 2.0 factors are study-specific design settings, not universal standards.

This protocol uses that paper as an architecture/optimization comparator and raises the evidence bar for physical fault survival: real fault exposure, post-fault material map, independent structural inspection and measured degraded mission are required.

## Fault envelope

Each test case freezes fault type and topology; pre-fault operating point, current, rotor speed and magnet temperature; fault current/time waveform and duration; DC bus, inverter behavior and shutdown/control sequence; cooling availability and coolant flow; magnet lot, geometry, magnetization and assembly revision; target degraded mission and safety policy.

Fault cases may include phase-to-phase or three-phase short circuit, inverter shoot-through, overcurrent, cooling loss, locked rotor, thermal overload and external demagnetizing field. They are separate test cases; no single generic short-circuit waveform represents all conditions.

## Ordered exposure trace

Record sequence number, timestamp, step duration, current, rotor speed, local/bulk magnet temperature, DC bus, cooling flow, fault state, exact fault identity, event receipt, method, calibration, uncertainty and evidence class.

A non-monotonic or duplicated trace, reused event receipt, missing sensor method, incomplete duration or out-of-envelope event cannot qualify. An exposure uncertainty interval that partially intersects the declared envelope is Indeterminate; an exposure definitely outside the declared scenario is NotComparable.

## Magnetic evidence

Collect a separate independent bulk pre/post magnetic measurement and a spatially resolved pre/post measurement for every preregistered magnet region.

For each measurement, preserve pre/post flux density, temperature, method, instrument, calibration and uncertainty. The evaluator computes:

    irreversible_flux_loss_pct = 100 * (1 - post_fault_flux / pre_fault_flux)

The reported bulk value comes only from the independent bulk measurement. Do not compute it by averaging local zones. Every local zone remains separately visible so that one severe region cannot be masked by a healthy average.

The flux-loss result is not by itself a universal pass/fail percentage. It is interpreted alongside Hcj, load-line and geometry evidence, mechanical retention, containment and the degraded mission.

## Structural and functional evidence

After the fault, inspect magnet retention, rotor containment, cracks/deformation and vibration; bind maximum stress and minimum safety-factor evidence to the stated design and inspection methods; physically measure post-fault torque-speed envelope, shaft energy, minimum torque reserve, thermal state and mission completion.

A design that preserves magnetic flux but loses rotor containment fails. A design that safely experiences bounded demagnetization but cannot meet the minimum degraded torque or energy requirement fails. An uncertainty interval crossing an application-specific safety/mission limit is Indeterminate.

## Benchmark ladder

- **F0:** exact identity, event ordering, local/bulk map completeness and negative-control semantics.
- **F1:** known-answer flux-retention arithmetic, interval propagation, duplicated/missing zones, sensor replay and uncertainty controls.
- **F2:** material/geometry-specific demagnetization curves, temperature-dependent Hcj and validated local field maps.
- **F3:** instrumented physical fault exposure at relevant current, speed, temperature and cooling envelope.
- **F4:** measured post-fault torque/energy/thermal degraded-mission replay, composed with #7890/#7887.
- **F5:** repeated magnet lots, rotor builds, multi-temperature / multi-fault testing, containment and independent replication.

## Competitive targets

Compare the same application and system envelope against optimized NdFeB IPM, ferrite spoke or PMaSynRM, wound-field synchronous, induction/SynRM and recovered-feedstock candidates. Compare pre-fault duty performance, post-fault degraded service, repair/replace decisions, thermal reserve, mechanical safety and virgin element intensity separately.

The 2025 ferrite spoke study gives a useful reference for study-specific demagnetization and mechanical safety factors, as well as a same-envelope 260 kW design. Its fault-response FEM must not be misreported as physical fault testing.

## Claim ceiling

Passing this protocol requires physical fault evidence. Synthetic tests are only semantic controls. No Luminous physical fault-test performance, safety certification, fleet reliability, lifetime model or commercial superiority is claimed.
