# SUBTERRANEAN-HUMAN-MOBILITY-001 — Human mobility and rescue transport benchmark V0.1

Status: research contract; synthetic/bench evidence only. No device is certified or declared safe for field use by this specification.

Tracking issue: https://github.com/Luminous-Dynamics/symtropy/issues/2037

## 1. Purpose

Develop evidence for tools that help people move, self-egress, carry gear and evacuate casualties through caves, mines, tunnels and other constrained subterranean spaces. The paradigm shift is to treat **human mobility as a coupled person–device–route–hazard system**, not as a contest for the fastest robot or strongest exoskeleton.

A mobility aid is useful only if its whole task envelope is matched to the passage geometry, surface, user, load, environmental hazards, available assistance, maintenance capacity and failure-recovery plan.

## 2. Context: four different environments

Do not transfer one environment's results to another without evidence.

- **Engineered mine/tunnel:** surveyed dimensions, fixed or semi-fixed infrastructure, known procedures and equipment may be available, but geometry and hazards can change after incidents.
- **Natural cave:** narrow irregular passages, fragile formations, vertical sections, mud/water and conservation constraints; permanent installation may be unacceptable.
- **Collapsed/unknown void:** unstable geometry, dust/smoke, poor visibility and uncertain hazards; remote reconnaissance and controlled access take priority.
- **Urban/subterranean utility space:** stairs, shafts, doors, utility clutter, variable construction and legal/site access constraints.

Every result must identify the environment class and test-course properties. Never pool them into a single headline score.

## 3. Evidence-informed starting point

The DARPA Subterranean Challenge explicitly treated mobility, perception, autonomy and networking as coupled technical gaps. Its mobility stressors included constrained passages, sharp turns, drops and climbs, inclines, falling debris, mud, sand and water. It used cave, tunnel and urban-underground settings and included survivor-location tasks: https://www.darpa.mil/research/programs/darpa-subterranean-challenge

NIOSH mining ergonomics resources identify awkward postures, forceful exertion, heavy/awkward objects, repetitive handling and slips/trips as important task-specific concerns: https://www.cdc.gov/niosh/mining/topics/strains-sprains.html and https://www.cdc.gov/niosh/mining/topics/ground-falls.html

This evidence motivates a benchmark and test programme. It does not prove a new device is safe or effective.

## 4. Design tracks

### 4.1 Human movement and load handling

Investigate passive, low-profile, modular supports before assuming a powered exoskeleton is the answer: improved knee/hand protection, load-transfer harnesses, configurable carry handles, low-snag tool carriers, stable rest/transition points and aids for awkward lifting. Measure whether aids lower exertion without increasing restriction, imbalance, contact injury or entrapment.

### 4.2 Casualty extraction

Evaluate a modular low-profile litter/sled with articulated joints, configurable drag/haul interfaces, controlled braking, anti-runaway behavior, load restraint, quick release and recovery/reversal procedure. Test bent passages, uneven floors, transitions, limited headroom and team counterflow.

Any load-bearing anchor, rope path or rigging point must be independently selected, rated, inspected and operated by qualified rescue personnel. A software route model cannot certify an anchor.

### 4.3 Constrained passage movement

Investigate carts, powered assists and compact tracked/legged platforms only within explicitly declared width, height, turning radius, grade, edge/step and braking limits. Test the combination of person + PPE + breathing apparatus + aid + cargo, not the bare device.

### 4.4 Remote-first reconnaissance

Use small robot platforms to map unfamiliar segments, observe candidate hazards and deploy communications relays where appropriate. Robots can reduce initial exposure and produce evidence; they cannot certify ground stability, atmosphere safety or an escape route. A failed/stuck robot must not block the only accessible passage.

### 4.5 Human factors and accessibility

Include variation in stature, reach, strength, mobility, gloves, helmets, breathing apparatus, fatigue, low light, noise, wet/cold conditions and ability to operate controls under stress. Controls must have a safe manual mode, visible state and clear release/stop behavior. Avoid requiring a cloud connection or phone for a critical action.

### 4.6 Why powered exoskeletons are not the default

Treat exoskeletons as a separate research track. Added bulk, battery mass, altered balance, heat, joint misalignment, power/controller failure, snagging and inability to exit a constriction can negate reduced exertion. Begin with task analysis and passive design; compare powered assistance only against a measured baseline and include fail-safe doffing/release tests.

## 5. Spatial and hazard model contract

Use Sol Atlas as the authoritative topology/geometry evidence layer and Symtropy as the scenario evaluator. Do not create a competing authoritative cave/mine map.

Represent each passage or transition as evidence-bearing data:

- Geometry: cross-section profile, clear width/height, bend radius, floor/ceiling envelope, vertical transition, steps/ledges, grade and uncertainty.
- Surface: material/state, wetness, mud/loose debris, measured or unknown traction, discontinuities and evidence age.
- Hazards: water/inrush, air quality, temperature, smoke/dust, falling-rock/support condition, pinch points and known restricted areas.
- Rescue system: measured aid/device envelope, carried load and dimensions, braking capability, validated turning/vertical transfer limits, load-rated anchors only when evidence is supplied by the qualified authority.
- Services: communications availability, localization confidence/age, power, refuge/exit/service dependencies and closure status.
- Provenance: source, survey/run ID, timestamp, calibration, uncertainty, exclusions, version digest and authoring/verification authority.

Do not encode a single boolean called safe. Keep separate facts such as measured, inferred, unknown, stale, blocked, prohibited and conditionally traversable. Missing geometry or hazard evidence must never be converted into a positive safety result.

## 6. Mobility modes and route semantics

Evaluate separate missions:

1. Person travels without an aid.
2. Person self-evacuates with ordinary carried equipment.
3. Conscious casualty is assisted by responders.
4. Unconscious/injured casualty is carried on a specified surrogate litter.
5. Aid/device travels empty, then loaded.
6. Robot scouts an unoccupied segment.
7. Responder counterflow occurs during egress.
8. Route becomes blocked, flooded, unsupported, unlocalized or disconnected.

A path that is traversable by a small robot is not necessarily traversable by a person or litter. A route suitable for entry is not necessarily suitable for extraction. A shortest path is not an acceptable answer when a mandatory geometry/hazard condition is unknown.

## 7. Benchmark corpus and comparisons

Start with synthetic geometries and guarded physical mock courses. Freeze course dimensions, materials, loads, user/PPE envelopes, fault schedule, environmental states and evaluation policy.

Comparators:

- C0: unassisted person or current site-approved equipment baseline.
- C1: passive purpose-built aid.
- C2: modular litter/sled/haul setup with qualified rigging.
- C3: powered mobility assist or cart within its specified envelope.
- C4: robot recon/mapping plus human-approved route.
- C5: combined mobility + map/hazard + communications support.
- Oracle: known course geometry only; diagnostic upper bound, never a deployable policy.

All candidates receive the same course and hazard evidence. Hold out complete course families and surface/material combinations rather than randomly splitting nearby samples.

## 8. Metrics and initial acceptance targets

Targets below are **laboratory hypotheses**, not measured results or universal product specifications.

### Hard gates

- **Geometric validity:** 100% correct rejection of cases outside the declared dimensions/turning/grade/load envelope in the frozen synthetic corpus.
- **Route truthfulness:** zero routes marked safe when required geometry, hazard, support or egress evidence is missing, stale or outside the validated envelope.
- **Load control:** zero unintended load release, rollover, uncontrolled run-away or critical pinch/entrapment events in the approved surrogate and guarded test set before any human trial.
- **Fail-safe:** every injected battery, motor, controller, brake, tether, navigation or communications failure reaches an explicit controlled-stop, manual recovery, reverse/abandon or no-safe-route outcome.
- **Evidence:** 100% of results reference a versioned course, device configuration, load, test run, calibration/source and digest. Missing measurements remain missing, not zero.

### Performance metrics

- Course completion and failure rate by environment/mobility mode; do not average unlike environments.
- Minimum passage envelope, turning radius, maximum tested slope/step, traction and braking distance with uncertainty.
- Hauling/handle forces, user posture, repeated lifting, contact pressure, surrogate peak acceleration, pitch/roll, vibration, stopping distance, setup and recovery time.
- Team size, exertion proxy, rest frequency, task duration, workload rating and near-miss count.
- Route-planning false-safe rate, conservative rejection rate, uncertainty calibration and change-detection time after route closure.
- Aid mass, volume, carryability, setup, inspectability, repair time, battery autonomy, energy per metre/load and recoverability after immobilization.
- Human–robot interaction, lost/stuck device frequency, obstacle blockage, communications outage and whether the robot itself obstructed egress.

### Candidate performance goals

A 95% course-completion goal may be explored only for a narrowly specified device/mode/course family and only after every hard gate is met; it must not compensate for catastrophic failure. Set permissible casualty acceleration, harness loads, human exertion, braking distances and vertical-transfer criteria with clinical, ergonomic and qualified rescue engineers before testing. Do not invent a universal acceptable number.

## 9. Coupling mobility to communications

A mobility kit and communications plan should be benchmarked together:

- A route relay may improve message delivery but must not create a trip/snare or block the passage.
- A stale location remains stale even when a message is delivered.
- A communication outage must not make the route planner assume the person is stationary or safe.
- Local offline maps, explicit uncertainty, locally stored instructions and manual distress methods must remain available.
- If a route closes or floods, report the closure and show only independently verified alternatives; never autonomously actuate machinery or direct a person into an unverified void.

Compose with Symthaea MINE-COMMS-001 (#8010 / PR #8011), MINE-TTE-CHAR-001 (#8047), MINE-COMMS-TRACE-001 (#8066) and FLOOD-EGRESS-001 (#1963). Acoustic/seismic signalling is a candidate distress/locator channel pending characterization; it is not assumed to replace routine speech/data radios.

## 10. Research sequence

A. Publish the geometry, environment and casualty/luggage surrogate schema.  
B. Generate deterministic synthetic course families and hostile invalid-evidence cases.  
C. Benchmark offline route admissibility and explicit no-safe-route semantics.  
D. Test passive aids and unloaded/loaded surrogate transport in a guarded mock course.  
E. Add controlled equipment-fault/recovery scenarios.  
F. Couple route closure and communications degradation to the same frozen scenario IDs.  
G. Conduct supervised human factors trials only after independent review, safe surrogates and appropriate institutional/site approval.  
H. Validate on held-out physical courses before making transfer claims.

## 11. Non-goals

No live-cave/mining deployment approval, no automatic evacuation or rescue actuation, no autonomous judgement that a roof/anchor/atmosphere is safe, no powered-device assumption, no certification by simulation, and no real-cave performance claim from a synthetic corpus.

## 12. Reference links

- DARPA Subterranean Challenge — https://www.darpa.mil/research/programs/darpa-subterranean-challenge
- NIOSH Mining: Strains and sprains — https://www.cdc.gov/niosh/mining/topics/strains-sprains.html
- NIOSH Mining: Ground falls — https://www.cdc.gov/niosh/mining/topics/ground-falls.html
- Symtropy MINE-SYSTEM-001 — https://github.com/Luminous-Dynamics/symtropy/issues/1933
- Symtropy FLOOD-EGRESS-001 — https://github.com/Luminous-Dynamics/symtropy/issues/1963
- Symthaea MINE-COMMS-001 — https://github.com/Luminous-Dynamics/symthaea/issues/8010
- Symthaea MINE-TTE-CHAR-001 — https://github.com/Luminous-Dynamics/symthaea/issues/8047
- Symthaea MINE-COMMS-TRACE-001 — https://github.com/Luminous-Dynamics/symthaea/issues/8066
