# Forest Terrain Grid Benchmark v0.1

**Status:** synthetic software contract; not a field benchmark, hardware safety case, or proof of forestry machinery conformity.

This tranche changes the optimization target. Instead of measuring only shortest path or nominal mission completion, it evaluates whether a robot can distinguish traversable from unknown terrain, plan a round trip with an energy reserve, produce declared degraded/blocked outcomes under injected faults, and tie coverage claims to traversal and observation instead of plans.

## Paradigm shifts

1. **Evidence before motion:** unobserved cells are not free space. The prototype refuses an unknown target rather than assuming it is safe.
2. **Round-trip viability, not one-way reachability:** route planning prices the return route plus a reserve before simulated traversal.
3. **Faults are first-class benchmark inputs:** GNSS loss, pose jumps, perception failure, wheel slip, emerging obstacles, comms loss, battery drop, stale time, map disagreement, and E-stop are frozen scenarios with expected state and reason codes.
4. **Coverage is an evidence ledger:** planned, traversed, and verified targets are separate fields. A plan cannot earn observation credit.
5. **Receipts are reproducible artifacts:** metadata binds the source commit, exact corpus bytes, synthetic map digest, config digest, fixed seed, and scenario outputs.
6. **Safety is layered:** simulated fail-closed transitions cannot certify motor controllers, physical stopping, worker separation, or machinery hazards.

## Frozen synthetic corpus

The fixture has 15 scenarios: one completed baseline, two degradations, and twelve blocked cases; ten scenarios inject explicit faults. The runner checks expected states and reason codes exactly and rejects a miss against each declared deadline in the simulator's step domain.

## Proposed benchmark targets for later versions

These are engineering targets, not achieved metrics or standards:

- **Software safety corpus:** 100% expected negative-control state and reason-code match; zero false completions; all injected fault transitions before declared simulated deadlines.
- **Unknown-space policy:** 100% rejection or deferral when the requested target is unobserved; add partial occlusion and conflicting maps next.
- **Return reserve:** no mission begins when modeled energy-to-return plus reserve exceeds available energy. Replace the illustrative per-cell model with calibrated platform measurements and uncertainty intervals.
- **Coverage integrity:** zero credit to planned-only cells; report planned, traversed, and independently verified surface area separately.
- **Localization:** once independent ground truth exists, publish median/p95 position error, drift per 100 m, time without valid pose, relocalization rate, and confidence calibration. This simple grid provides none of those measurements.
- **Forest inventory (separate dataset track):** tree-detection precision/recall and DBH median/p90 error stratified by species, diameter, occlusion, terrain, season, and sensor package. A 2025 forest-inventory field study reports one setup reaching up to 1 ha within 30 minutes and typical DBH accuracy around 2 cm; that is a context-setting reference, not a guaranteed target for this design. Source: https://doi.org/10.1109/TFR.2025.3583972.
- **Operational/ecological cost:** joules per verified hectare, cost per qualified observation, intervention rate, ground disturbance, missed hazards, maintenance hours, and repairability; compare against a human baseline on the same plots.

## Research lessons incorporated

The 2023 SPOT forest-inventory trial found tree identification and position errors varied across sensing and path conditions, supporting stratified reporting instead of one aggregate score: https://www.mdpi.com/1999-4907/14/11/2170. GNSS navigation should not be treated as ground truth; Nav2's GPS tutorial describes generic GPS errors that can be meters and can jump: https://docs.nav2.org/rolling/tutorials/general_tutorials/navigation2_with_gps/navigation2_with_gps/. Safety scope needs qualified review against the applicable edition and machine-specific hazards of ISO 11850 and other relevant standards; this prototype does not claim compliance: https://www.iso.org/standard/51182.html.

## Deliberate limitations

No ROS adapter, motors, physical E-stop, live sensors, felling, chainsaw, planting tool, load carrying, production map fusion, stochastic-confidence claim, or field-performance claim. Fault response is instantaneous in software and reported in simulation steps. The model is a small BFS grid whose energy units are illustrative. Next increments: deterministic footprint-based raster coverage, pose-uncertainty envelopes, partial sensor footprints, dynamic obstacles, map-reconciliation events for two robots, and independent property/fuzz tests before considering supervised physical trials.
