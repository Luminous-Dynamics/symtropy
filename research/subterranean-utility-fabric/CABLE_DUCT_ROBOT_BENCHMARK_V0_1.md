# Cable-Duct Robot Benchmark v0.1

**Parent programme:** https://github.com/Luminous-Dynamics/symtropy/issues/4910  
**Dedicated workstream:** https://github.com/Luminous-Dynamics/symtropy/issues/4913  
**Status:** Research specification for a non-energized laboratory qualification only.

## 1. Scope and non-transfer rule

Qualify inspection and maintenance tooling for utility cable ducts and conduits. Sewer-crawler results are not transferable by default: cable ducts differ in diameter, bend / sweep geometry, couplings, occupancy, pulling loads, tether hazards, moisture, debris and visibility.

This benchmark separates four capabilities:
1. traverse and inspect a declared empty / occupied duct profile;
2. locate and measure a seeded obstruction or duct condition;
3. place / retrieve a draw rope;
4. pull an inert dummy cable while respecting its declared mechanical envelope.

Passing one does not imply passing the others.

The first demonstrator is a modular, non-energized laboratory rig. No live cable, energized asset, public infrastructure or autonomous electrical operation is involved.

## 2. Qualification article and test rig

Construct a modular rig from measured and revision-controlled sections. Exact dimensions are selected from real, rights-cleared duct product and utility requirements before the test campaign; this document does not invent a universal duct size.

The rig should support controlled substitution of:
- straight duct sections and known lengths;
- single and compound bends with measured centreline radii;
- joints, couplers, diameter transitions and intentionally seeded offsets / steps;
- tees, junctions, access chambers and pull boxes;
- horizontal, inclined and vertical transitions;
- empty duct, draw rope and inert dummy cable occupancy;
- dry, damp, standing-water and controlled silt / debris states;
- partial blockage, full blockage and defined obstruction inserts;
- lighting, reflectivity, sensor fouling and communication / tether conditions.

Each module receives a stable ID, measured geometry, tolerance, material / product revision, inspection record and fixture digest. Rig truth must be independent of the robot under test.

### Test-article profile

A profile binds:
- internal diameter / cross-section and tolerance;
- total length, straight lengths and bend geometry;
- joint / coupling type, spacing and internal steps;
- access-node dimensions and retrieval geometry;
- material, liner and internal surface properties;
- route slope and orientation;
- occupancy state;
- water, debris and obstruction state;
- environmental limits;
- the exact device / tool capability envelope under test.

Do not use a single generic "pipe" profile for all results.

## 3. Robot and tooling families

Compare only devices that physically fit the declared profile:
- inspection camera / sensor head on pushrod or semi-rigid rod;
- tethered crawler for compatible bore and bend geometry;
- draw-rope insertion and retrieval tool;
- instrumented capstan / winch or pulling tool for an inert dummy cable;
- conventional manual proving / mandrel / draw-rope / CCTV process as the relevant baseline.

A microduct may be better served by a draw-through inspection head or conventional proving tool than by a crawler. The benchmark must allow the simpler tool to win.

## 4. Run-level evidence record

Every run binds:
- profile ID, geometry revision and fixture digest;
- robot / tool / tether / controller / firmware / battery identities;
- sensor and calibration identities, health and synchronization;
- operator / reviewer role and procedure revision;
- ground-truth stationing and obstruction / defect locations;
- occupancy, water, debris and environmental state;
- mission plan, start / stop times, actual route and all retries;
- raw observations and coverage / visibility disposition;
- localization trace and uncertainty;
- pull-force profile, where applicable;
- stop / retrieval / recovery events and operator interventions;
- pre/post duct and dummy-cable inspection;
- disposition and explicit claim ceiling.

A missing geometry or mechanical limit remains unknown / unsupported. Do not infer the cable's minimum bend radius, allowable pulling tension or jacket resistance from a visually similar product.

## 5. Benchmark suites

### D1 — traversal and access
Measure:
- route reachability by profile;
- observed / unobserved / unusable / not reached / unsupported length;
- traversal, setup, cleaning and retrieval time;
- junction and branch confusion;
- entry, turning, reversal and exit success;
- tether management and snag frequency.

### D2 — localization and metrology
Measure:
- chainage P50 / P95 / maximum error against rig truth;
- position / heading error where ground truth supports it;
- drift per metre and loop-closure correction;
- bend, joint and junction localization;
- obstruction position and dimensional MAE;
- uncertainty-interval coverage and abstention.

Acceptance thresholds are profile- and task-specific. Freeze them before runs based on the access spacing and maintenance decision, not after observing results.

### D3 — inspection and obstruction detection
Measure:
- per-class precision / recall and critical-blockage false-negative rate;
- duct deformation / joint-offset metrology error where the sensor is capable of measuring it;
- false alarms per route length;
- image / geometry quality-gate performance;
- percentage of obscured / flooded / fouled surface;
- repeat inspection burden and human review minutes.

A duct defect is not automatically a cable insulation defect, electrical failure, remaining-life estimate or permission to energize.

### D4 — draw-rope and inert cable handling
Measure:
- insertion and retrieval success;
- peak, percentile and integrated pulling force;
- force spikes at bends, joints and transitions;
- bend-radius compliance against the exact dummy profile;
- snag / break / retrieval event rates;
- dummy-cable abrasion / jacket damage before and after;
- time, energy and operator interventions per accepted run.

Dummy cable properties are declared fixture inputs. This is not a type test or installation qualification for any commercial power cable.

### D5 — recovery and fault containment
Inject:
- communications loss;
- low battery / brownout;
- drive-wheel / track immobilization;
- tether snag or excessive pulling force;
- flooded / blocked segment;
- sensor fouling or localization loss;
- access-node obstruction;
- controller restart or stale mission;
- retrieval-tool failure.

Measure safe-stop correctness, successful recovery, time to recover, duct / dummy-cable damage, operator intervention and ability to restart with a known mission state.

## 6. Baselines and fair comparison

Use the best applicable conventional workflow for each profile:
- manual duct proving / mandrel and draw rope;
- conventional CCTV or inspection head where physically applicable;
- human operator inspection and fault localization;
- vendor-stated device envelope, kept separate from independently measured capability.

Use the same rig, geometry, obstruction truth, condition, operator skill / time accounting and acceptance rule. Include setup, calibration, cleaning, maintenance, retrieval and failed / aborted runs. Do not compare robot drive speed against manual labour cost while excluding the robot's setup, tether handling and recovery.

Report results separately for each profile, tool, occupancy state and environment. A pooled average must not hide failure in a profile.

## 7. Proposed gates — targets, not achieved

### G0: digital / finite corpus
- all route segments receive explicit observation dispositions;
- zero unknown-to-clear promotion;
- 100% detection of seeded hard robot / duct capability mismatches in finite fixtures;
- deterministic report bound to exact fixture, tool and calibration identities.

### G1: dry laboratory envelope
- zero irretrievable robots / tools inside the declared profile envelope;
- 100% route-coverage accounting;
- zero damage to duct or dummy cable in the frozen declared test set;
- all seeded safe-stop conditions produce the predeclared safe state and operator handover;
- per-profile localization / metrology thresholds are fixed before the run.

### G2: wet / debris / communications envelope
Repeat G1 with declared moisture, standing water, debris, fouling and communication faults. Preserve all aborted missions and manual retrieval in the denominator. Do not extrapolate beyond the tested envelope.

These are laboratory qualification gates only. They do not authorize a field pilot, energized-cable interaction or public-asset operation.

## 8. Luminous composition

- **Symtropy:** deterministic fixture replay, recovery scenarios and accepted-work metrics.
- **Symthaea:** bounded route / inspection planning, uncertainty-aware triage and decision-value proposals; no dispatch or electrical authority.
- **Sol-Atlas:** read-only route, asset and as-built context with spatial provenance; no duplicate registry.
- **Mycelix:** actor / authorization / material / test evidence provenance under existing owners; no ledger-issued permit.
- **Symtropy #1834:** reuse generic robot capability matching for workspace, reach, tool, energy and calibration.
- **Symtropy #4905:** reuse observation-quality and human-adjudication concepts, but not sewer labels or scores as cable-duct qualification.

## 9. First implementation order

1. Freeze one rights-cleared duct profile and one fully measured modular rig configuration.
2. Publish a versioned machine-readable profile and run manifest.
3. Implement a finite route / obstacle / fault fixture corpus with independent expected outcomes.
4. Define conventional manual proving / draw-rope / CCTV comparison protocols.
5. Instrument traversal, tether force, pull force, energy, operator time and pre/post damage.
6. Only after G0 review, build or adapt the non-energized laboratory rig.
7. Revisit other duct profiles only after the first profile has a reproducible result.

## 10. Nonclaims

No robot hardware is currently qualified. No sewer robot paper result proves cable-duct performance. A laboratory run does not prove cable integrity, electrical insulation, thermal rating, protection coordination, installation compliance, safety for energization, legal access, utility approval or authority to manipulate an electrical asset.
