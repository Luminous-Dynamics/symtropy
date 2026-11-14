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

## Complete nineteen-family profile panel

The mission fixture now has exactly one scenario for each of the 19 vehicle design IDs in the v0.1 catalogue:

1. Cargo micro-mobility: last-mile delivery.
2. Compact passenger EV: urban + arterial trip and reserve sensitivity.
3. Family passenger EV: regional travel with climb/descent.
4. Accessible shuttle: urban/suburban route plus boarding/HVAC auxiliary budget.
5. Light commercial van: cargo route plus auxiliary demand.
6. Rural all-terrain utility: track, grade and rough-ground stress route.
7. Emergency/resilience vehicle: response travel plus declared stationary export energy.
8. Municipal work truck: route plus work-tool energy.
9. Smallholder tractor: field passes plus declared implement energy.
10. Medium row-crop tractor: seasonal task plus continuous implement work.
11. Compact excavator: very low-speed travel plus explicit stationary excavation energy.
12. Urban distribution truck: stop-start delivery route.
13. City bus: passenger route plus high HVAC/auxiliary energy.
14. Long-haul truck: loaded flat, climb and descent segments.
15. Port/yard terminal tractor: yard movement plus hitch-work energy.
16. Underground mine unit: loaded grade, return drift and stationary work demand.
17. Open-pit battery-only haul control: intentionally severe loaded uphill negative control.
18. Open-pit trolley+battery design: **off-wire battery-reserve route only**; trolley-supplied traction is outside the mission's battery-energy boundary and must not be counted twice.
19. Autonomous warehouse carrier: repeatable low-speed cycles.

Current fixture values are synthetic arithmetic examples—not data measured from vehicles, manufacturers, South African roads, farms, mines or depots. Four profiles are deliberately/assumption-bound blocked by the current arithmetic model: compact-EV city/arterial reserve sensitivity, light-van delivery, rural 4×4 rough/uphill support, and open-pit loaded uphill stress. This makes energy assumptions visible and gives the design review something to change or validate. These results are not real-vehicle feasibility judgments, and passing scenarios are not production-qualified.

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