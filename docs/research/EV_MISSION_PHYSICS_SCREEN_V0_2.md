# EV Mission Physics Screen v0.2

## Purpose

The v0.1 design screen assumes battery-side energy demand has already been supplied. This v0.2 reference model adds a deliberately simple longitudinal calculation so selected mission profiles can estimate traction energy from loaded mass, distance, speed, road grade, rolling resistance, drag area, drivetrain efficiency and acceleration. It also counts auxiliary and work-tool energy, applies power-limited regeneration, and tracks usable battery energy through the declared sequence of segments.

The result is a falsifiable first-order screen, not a calibrated digital twin. All current mission profiles are synthetic. The tool has no authority over vehicle safety, public-road use, mine qualification or production design.

## Run

From the repository root:

    cargo test --manifest-path tools/rural-industrial-electrification-evaluator/Cargo.toml --all-targets
    cargo run --quiet --manifest-path tools/rural-industrial-electrification-evaluator/Cargo.toml --bin vehicle_mission_physics -- tools/rural-industrial-electrification-evaluator/fixtures/vehicle_designs_v0_1.json tools/rural-industrial-electrification-evaluator/fixtures/vehicle_missions_v0_2.json

JSON output is sorted by scenario ID. Segment order remains chronological and is never sorted.

## Model

Loaded mass:

    m = base_curb_mass + installed_module_mass + target_payload

The grade input g_r is rise/run, with its sign indicating uphill/downhill; this model converts it to a slope angle through:

    cos(theta) = 1 / sqrt(1 + g_r^2)
    F_roll = Crr * m * g0 * cos(theta)
    F_grade = m * g0 * g_r * cos(theta)
    F_aero = 0.5 * air_density * drag_area * average_speed^2
    average_speed = distance / segment_duration
    delta_Ekinetic = 0.5 * m * rotating_mass_factor * (v_end^2 - v_start^2)
    wheel_work = (F_roll + F_grade + F_aero) * distance + delta_Ekinetic

Positive wheel work is converted to battery traction energy by dividing by declared drivetrain efficiency. Negative net wheel work creates a potential regenerative-energy amount, which is capped by declared regenerative efficiency, receiver power limit over the segment duration, and available usable-pack headroom. Auxiliary loads use kW × hours. Work-tool energy is an explicit battery-side input per segment; v0.2 does not infer implement, hydraulic or excavation work from first principles.

Usable capacity is computed from pack capacity × state of health × temperature capacity fraction × (1 − reserve fraction). The mission begins at the declared fraction of this post-reserve usable capacity. Energy-consuming loads are applied before regeneration within each segment; the model records shortfall when SOC reaches zero. This convention intentionally exposes cases where later downhill regeneration cannot rescue an earlier service shortfall.

## Output meaning

- **Total traction battery energy:** battery-side energy for positive calculated wheel work.
- **Total auxiliary energy:** travel and declared non-travel auxiliary energy.
- **Total declared work energy:** external work-tool demand already expressed at the battery boundary.
- **Regeneration available:** theoretical post-efficiency regeneration before power and battery-headroom caps.
- **Regeneration captured:** energy the simplified segment model allows into the usable pack.
- **Regeneration curtailed:** potential regenerated energy not captured under this simplified cap.
- **Net battery energy demand:** traction + auxiliaries + work minus actually captured regeneration.
- **Unserved energy:** energy demanded after the usable battery state is depleted along the ordered mission.
- **Maximum segment average traction power:** segment-average positive traction power at the battery boundary, not instantaneous peak motor/inverter power.

The separately declared peak-power demand is checked against the design's battery peak-power rating. Segment-average power is another lower-bound diagnostic. This version does not reconstruct transient power traces or certify continuous motor/inverter ratings. A profile with acceptable net energy can still be blocked when its initial state of charge or segment ordering creates a shortfall.

Statuses:
- passes_synthetic_screen: the limited arithmetic checks pass for synthetic inputs only.
- blocked: a declared mass, usable energy/SOC, or peak-power gate fails.
- indeterminate: evidence is explicitly unknown and no already-known hard failure blocks it.
- passes_evidence_backed_screen: non-synthetic evidence has been declared, but specialist validation remains required.

## Current eight-profile panel

- Compact passenger EV: urban + arterial trip and reserve sensitivity.
- Family passenger EV: regional travel with climb/descent.
- Rural all-terrain utility: track, grade and rough-ground stress route.
- City bus: passenger route plus high HVAC/auxiliary energy.
- Compact excavator: very low-speed travel plus explicit stationary work energy.
- Long-haul truck: loaded flat, climb and descent segments.
- Open-pit haul: intentionally severe loaded uphill negative control.
- Autonomous warehouse carrier: repeatable low-speed cycles.

Current fixture values are synthetic arithmetic examples—not data measured from vehicles, manufacturers, South African roads, farms, mines or depots. The compact EV and rural 4×4 profiles are allowed to fail: that is useful evidence that the current assumed capacity/reserve does not cover the chosen scenario. Do not interpret the output as a universal feasibility judgment for those vehicle classes.

## Boundaries and failure modes

The model does not represent drive-cycle interpolation, speed limits, tire slip, yaw/handling, changing mass through cargo use, tire pressure, air conditioning transients, weather/wind, road-surface changes inside a segment, gear selection, motor/inverter efficiency maps, torque limits, battery voltage/current/temperature dynamics, SOC-dependent charge acceptance, regen friction-brake blending, tire/axle limits, thermal steady-state, PTO/hydraulics, suspension/rotating-inertia calibration, charging queues, battery aging over life, grid load flow, lifecycle cost/emissions or safety. A stationary work segment counts its explicit battery-side work energy but does not predict productivity per cycle.

A negative slope/kinetic work result is not equivalent to friction-braking safety. Regenerative power is not credited as a safe-stop capability. Each design still requires a calibrated mission model, instrumented validation, independent safety review and applicable homologation/industry qualification.

## Validation targets

1. Hand-calculated flat/grade/aero/kinetic cases agree within an explicitly chosen numerical tolerance.
2. Regeneration never exceeds the energy potential, declared battery-side power limit × duration, or remaining usable capacity.
3. SOC never leaves the usable interval; every unserved-energy event remains visible and blocks the candidate.
4. Mass plus payload/upfit is compared to the declared gross limit.
5. Inputs reject NaN/infinity, duplicate identities, invalid grade/efficiency domains and unknown design references.
6. Output is deterministic under scenario-order permutation; segments retain mission order.
7. Measured data include route identity, speed/grade samples, payload, ambient conditions, pack health/SOC, auxiliary loads, calibration revision and uncertainty.

## Next model layer

Add speed-time traces and segment substepping; motor/inverter torque and efficiency maps; battery voltage/current/temperature and SOC-dependent charge acceptance; continuous thermal limits; road/soil/rolling-resistance evidence and wind; explicit axle load, tire and brake envelopes; PTO/hydraulics work-cycle models; and a paired validation corpus from dynamometer/vehicle instrumentation. Keep each extension separately testable rather than hiding it behind a single efficiency factor.