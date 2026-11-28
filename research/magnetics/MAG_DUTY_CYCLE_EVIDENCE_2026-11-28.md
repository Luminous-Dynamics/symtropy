# Magnetic duty-cycle evidence protocol

**Programme:** MAG-DUTY-CYCLE-EVIDENCE-001  
**Issue:** [Symtropy #7887](https://github.com/Luminous-Dynamics/symtropy/issues/7887)  
**Revision:** 0.1-draft  
**Date:** 28 November 2026

## Decision question

Does an identified motor component deliver the declared application function across the complete ordered mission profile, while respecting every segment-level thermal, demagnetization, retention and performance requirement, and what are its conservative net energy flows over that same mission?

The evaluator accepts physical measurement evidence. It does not calculate electromagnetic, thermal, mechanical or inverter physics.

## Profile boundary

The profile binds the application, component, ordered segments, system boundary, measurement setup and initial thermal state. A segment declares its duration, operating quadrant and acceptance limits.

A comparison between architectures is meaningful only when the profile digest, component application, system boundary and measurement setup are matched. A segment cannot be silently re-ordered, omitted, duplicated, substituted or appended.

## Energy accounting

Motoring, regenerative and auxiliary energy are separate records. The net DC energy is computed as:

    E_net = E_DC,motoring + E_DC,auxiliary - E_DC,regenerative-return

Motoring and regenerative efficiency are separate ratios of aggregated energy totals:

    eta_motoring = sum(E_shaft,out) / sum(E_DC,in)
    eta_regenerative = sum(E_DC,out) / sum(E_shaft,in)

Do not compute the unweighted arithmetic mean of segment efficiency percentages. The segment energies carry the actual duration and profile weighting; the evaluator aggregates energy, not percentages.

The first implementation treats each interval as a deterministic bound. Interval sums and ratios remain conservative without assuming statistical independence. This does not mean that a calibration source, confidence interval or uncertainty record is valid: its issuer, method scope and coverage remain external evidence requirements.

## Hard requirements

Every ordered segment is checked independently. The initial implementation checks torque and speed tracking, maximum winding and magnet temperature, minimum demagnetization margin and minimum retention margin.

A definite physical interval outside a hard limit is FAIL. An interval overlapping a limit is INDETERMINATE. Missing/invalid evidence is INDETERMINATE. An average efficiency or lower cost cannot compensate for a local violation.

Thermal start and end state digests are chained from the frozen initial state across the mission. This is an identity/continuity control, not a thermal solver or a proof of correct state measurement.

## Standards and external comparators

- IEC 60034-2-3:2024 defines methods to determine converter-fed AC motor losses and efficiency. It provides seven standardized operating points and an interpolation procedure across the torque/speed range, including field weakening and overload. Use that standard's applicable methods as a reference dataset and test anchor; do not claim that seven points automatically represent an application's mission.
- ISO 21782-1:2023 defines general test conditions and terminology for voltage-class-B propulsion components and motor systems.
- IEC 60404-5:2015 and IEC 60404-18:2025 provide permanent-magnet magnetic-property measurement methods. Magnet-temperature and material-state scope remain attached to each applicable claim.

## Integration

The resulting duty-profile summary is an input to the architecture decision owner (#7802). The aggregator composes the existing route/specimen, cohort, maturity, field-map, covariance, thermal-cooling and recovery workstreams. It does not own route ranking, component acceptance, manufacturing yield, LCA, material recovery, or architecture superiority.

## Validation levels

- **D0:** exact schema, profile scope, order, uniqueness and complete coverage.
- **D1:** synthetic known-answer energy arithmetic and ratio-of-sums.
- **D2:** adversarial local hard-limit and uncertainty cases.
- **D3:** ordered thermal-state continuity.
- **D4:** replay of an available, licensable, measured seven-point converter-fed motor dataset.
- **D5:** independently measured component data under a frozen mission profile.
- **D6:** M4 acceptance, M5 lot/build cohort and third-party replication.

## Claim boundary

The v0.1 fixture is synthetic. Rust test success establishes software semantics only. No physical efficiency, energy saving, thermal advantage, demagnetization result, architecture advantage, standards conformity or production qualification is claimed.
