# ISO 18646-5:2026 — locomotion-performance benchmark crosswalk

Status: research crosswalk to public scope/preview information, not a conformance claim and not a reproduction of the normative standard.

Official standard page: https://committee.iso.org/cms/live/live/en/sites/isoorg/contents/data/standard/08/68/86850.html
Standard: ISO 18646-5:2026, *Robotics — Performance criteria and related test methods for service robots — Part 5: Locomotion for legged robots*, first edition, July 2026.

## 1. Scope and boundary

ISO 18646-5:2026 establishes methods for specifying and evaluating locomotion performance of complete legged robots. The published preview explicitly states that it is **not intended for verification or validation of safety requirements**. It is therefore an important industry comparability baseline, but it cannot replace Symtropy's independent viability, contact-law or runtime-assurance evidence.

The standard is neutral about intended use, mode and hardware configuration, and does not prescribe which performance tests every robot type must select. Record the selected clauses, conditions and locomotion modes for each evaluated robot. Do not advertise full conformance based on a partial benchmark.

## 2. Publicly identified performance test classes

The public preview's contents/index identifies these performance classes. Exact facilities, procedures, calculations, acceptance criteria and report fields must be taken from a lawful copy of the full standard; this crosswalk deliberately does not reproduce normative text.

| ISO performance class | Symtropy evidence to collect alongside the standard result |
|---|---|
| Rated speed | trajectory, command/achieved speed, repeatability, energy per distance, terrain and payload identity |
| Stopping distance | initial speed, stop command and actuator timing, distance, pitch/roll, contact transitions, deadline misses |
| Maximum climbing height | obstacle geometry, approach condition, foot-placement sequence, contact mode, success/failure witness |
| Stair-climbing capacity | stair geometry, gait/mode, traversal success, timing, retries, energy and contact/actuator faults |
| Maximum slope angle | slope and surface friction characterization, ascent/descent direction, slip, actuator utilization, recovery options |
| Dragging force | load and attachment geometry, force direction, speed, power/energy, traction and actuator margins |
| Crossing capacity | gap/obstacle geometry, support sequence, body clearance, phase-conditioned viability and failures |
| Minimum pass-through height | robot configuration, body/limb clearance, posture, contact interference and recovery margin |
| Minimum travel width | corridor geometry, turning/stride envelope, collision/clearance margin and repeatability |

These are not substitute test procedures. They are an evidence crosswalk so that each selected standard performance result also carries the state, model, uncertainty and failure context needed for engineering diagnosis.

## 3. Test-condition identity

For every test, preserve:
- exact robot configuration, payload, locomotion mode and firmware/controller version;
- assembly and operational preparation state;
- relevant sensor calibration identities;
- surface material, geometry and friction measurement/provenance;
- ambient/environmental conditions;
- battery state, supply voltage and actuator thermal state;
- initial pose, trajectory/command, test fixture and trial seed;
- number of attempts, completion definition, exclusions and failures.

The publicly available preview specifies calibration as part of preparation and identifies environmental and travel-surface conditions. The preview also describes a hard/even horizontal surface with a friction coefficient in the 0.75–1.0 range as a default test surface when a test does not specify otherwise, with the coefficient measured under ISO 7176-13. The full standard governs the exact application and any exceptions.

## 4. Beyond performance: Symtropy's separate assurance scorecard

For the same physical tests, add—but do not merge into the ISO performance result:

1. **Physical viability:** robustly admissible actions, viable-state coverage, recovery horizon and counterexample successor.
2. **Hybrid/contact assurance:** contact mode/evidence, branch completeness, friction-law admissibility and unresolved event ordering.
3. **Dynamic stability:** planar support/ZMP margin, 3D force-tilt margin and moment-tilt margin reported separately.
4. **Viability reserve:** axiswise uncertainty ladders and tested threshold brackets, without interpolation or scalar aggregation.
5. **Runtime evidence validity:** sensor freshness/health, calibration/model digest, actuator derating, command age and deadline.
6. **Statistical evidence:** denominators, independent trials, confidence intervals and failure taxonomy.
7. **Operational cost:** energy, thermal state, latency p50/p95/p99/max, memory and recovery time.

ISO locomotion performance and Symtropy assurance evidence must remain two adjacent, separately labelled result sets.

## 5. How to compare with industry without misleading claims

- Reproduce the standard's selected performance procedure and conditions as closely as the full licensed standard requires.
- Publish the exact selected clauses and deviations. Do not claim "ISO-compliant" or "ISO-certified" from this research crosswalk.
- Compare matched robots/morphologies only after normalizing payload, surface, environmental conditions and locomotion mode.
- Keep simulation-only evidence separate from complete-machine trials.
- Publish failures and aborts; do not report success rate without numerator, denominator and confidence interval.
- Treat zero observed failures as a finite-sample result, not zero risk.
- Add Symtropy's safety/assurance scorecard separately; do not imply the standard supplies those safety guarantees.

## 6. References

- ISO 18646-5:2026 official standard page: https://committee.iso.org/cms/live/live/en/sites/isoorg/contents/data/standard/08/68/86850.html
- Public preview/index of performance classes and conditions: https://standards.iteh.ai/catalog/standards/iso/9a2e0e1a-19c7-4816-a10e-6bc3707ac369/iso-18646-5-2026
- SPINDER six-leg reference platform, *Scientific Reports* 16, 26147 (2026): https://doi.org/10.1038/s41598-026-56856-0
- Contact-reliability-aware neural residual KF baseline, *Sensors* 26(19), 6041 (2026): https://doi.org/10.3390/s26196041
