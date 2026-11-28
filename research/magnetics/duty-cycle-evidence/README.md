# Magnetic duty-cycle evidence aggregator

Programme: MAG-DUTY-CYCLE-EVIDENCE-001  
Issue: #7887  
Revision: 0.1-draft

## Purpose

Produce a digest-bound evidence summary for one ordered, application-specific motor mission profile. The result is an input adapter for architecture comparison (#7802), not a motor solver and not a weighted architecture score.

The v0.1 evaluator accepts physical measurement intervals for each required segment. It checks exact sequence and scope, preserves thermal-state continuity, applies non-compensatory local requirements, and aggregates energy flows without averaging percentages.

## Why this layer exists

The v0.1 architecture evaluator accepts one interval observation per metric. That is useful for testing architecture decision semantics, but a single efficiency observation cannot prove performance across a drive cycle. This crate produces an aggregate duty-profile receipt that can later be consumed by the architecture evaluator.

## Energy semantics

Energy flows are kept distinct:

- Motoring: DC input energy and shaft output energy.
- Regeneration: shaft input energy and returned DC output energy.
- Auxiliary loads: cooling, field excitation and declared control loads.
- Net DC energy: motoring DC input + auxiliary DC input - regenerative DC output.

Motoring and regenerative efficiencies use ratios of aggregate energy totals. They are not the arithmetic mean of segment efficiency percentages. The auxiliary load boundary is explicit and is not silently folded into or removed from traction energy.

All energy values are bounded intervals in joules. Interval addition, subtraction and positive-denominator division are conservative; no statistical independence or probability distribution is assumed. Inputs must be bounded measurement intervals, not unqualified confidence intervals.

## Segment requirements

Every profile segment carries its own limits for torque tracking, speed tracking, winding temperature, magnet temperature, demagnetization margin and retention margin. A good cycle-level energy result cannot compensate for a failed local requirement.

A segment is:
- Qualified when each required interval is entirely inside its acceptance region and the evidence is admissible.
- Fail when a valid physical interval definitely violates a hard requirement.
- Indeterminate when an interval crosses a limit or evidence is missing, invalid or non-physical.
- NotComparable when profile order, identity or frozen system scope does not match.

The profile is ordered. Each segment's thermal start-state digest must match the frozen initial state (first segment) or the previous segment's end state. Independent steady-state points do not establish transient thermal continuity.

## Evidence and limitations

Each record binds the exact profile, component, system boundary, measurement setup, method, calibration, uncertainty record, physical evidence, acceptance receipt and thermal-state transition. Digests are syntactic identity checks only; this evaluator does not authenticate issuers, verify accreditation registries or independently validate instrument calibration.

v0.1 admits only physical measurement records for qualification. Simulation, literature and synthetic records cannot promote a physical duty profile. Interpolation and solver-derived map filling are deferred until their model-domain, discrepancy, calibration and holdout evidence are independently qualified.

## Standards alignment

- IEC 60034-2-3:2024 provides converter-fed AC motor loss/efficiency methods, seven standardized load points and interpolation across torque/speed regions. It is a standardized comparison anchor where applicable, not a replacement for the application's frozen mission profile.
- ISO 21782-1:2023 provides general test conditions and definitions for voltage-class-B electric propulsion components and motor systems.
- IEC 60404-5:2015 and IEC 60404-18:2025 are relevant permanent-magnet property measurement methods; temperature and specimen-state scope must remain attached to the measured evidence.

This research protocol does not claim conformity with any standard.

## Composition

This workstream consumes rather than duplicates:
- specimen and route qualification (#7773 / #7770);
- build/lot denominator and manufacturing cohort outcomes (#7776 / #7774);
- evidence-derived M1-M5 maturity (#7792);
- experiment-selection policy traces (#7798 / #7794);
- system architecture comparison (#7802);
- thermo-electrical winding/cooling comparisons (#7824);
- circular and second-life rotor evidence (#7870);
- field-map and uncertainty foundations (#7332, #7370, #7739).

## Validation ladder

D0 schema, exact identities and complete ordered coverage.  
D1 known-answer energy arithmetic and ratios of sums.  
D2 local hard-limit and uncertainty negative controls.  
D3 thermal continuity and profile order.  
D4 replay of a published or independently measured seven-point data set where licensing permits.  
D5 matched physical component runs under a frozen application mission profile.  
D6 composition with M4 component acceptance and M5 replicated build/lot evidence.

## Claim ceiling

The software fixture is synthetic. Passing Rust tests proves evaluator behavior only. It does not establish energy savings, thermal superiority, a demagnetization margin, magnetic material performance, motor architecture superiority, certification or production qualification.
