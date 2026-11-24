# Orbital Acquisition Co-Design Benchmark v0.1

**Status:** research / implementation protocol; bounded synthetic exact-search target  
**Parent:** [Symtropy #6845](https://github.com/Luminous-Dynamics/symtropy/issues/6845)  
**Work item:** [ORBITAL-ACQUISITION-CODESIGN-001 / #7385](https://github.com/Luminous-Dynamics/symtropy/issues/7385)  
**Related:** [#7350](https://github.com/Luminous-Dynamics/symtropy/issues/7350), [#7339](https://github.com/Luminous-Dynamics/symtropy/issues/7339), [#7222](https://github.com/Luminous-Dynamics/symtropy/issues/7222), [#7164](https://github.com/Luminous-Dynamics/symtropy/issues/7164), [#7043](https://github.com/Luminous-Dynamics/symtropy/issues/7043), [#6846](https://github.com/Luminous-Dynamics/symtropy/issues/6846), [#5920](https://github.com/Luminous-Dynamics/symtropy/issues/5920), [#5892](https://github.com/Luminous-Dynamics/symtropy/issues/5892)

## 1. Objective

Given a supplied, already-admitted robot path and a finite set of acquisition events, choose segment speed profiles, exposure durations, illumination/strobe settings and (where applicable) structured-light pattern dwell so that the complete inspection schedule:

- satisfies every required per-characteristic evidence gate;
- respects supplied robot motion, energy, collision and return-reserve constraints;
- minimizes elapsed time and resource use without weakening a measurement threshold;
- retains the Pareto frontier instead of hiding trade-offs in one weighted score;
- returns an explicit infeasible / indeterminate result when no admissible schedule exists.

This issue owns only **bounded co-design along a supplied path**. It does not generate viewpoints, globally route robots, calibrate cameras, compute BRDF responses, infer feature observability, propagate metrology covariance or control physical hardware.

## 2. SOTA positioning: what already exists

Do not claim that quality-aware speed optimization or integrated robotic visual inspection is new.

1. *Specifying and optimizing robotic motion for visual quality inspection*, Robotics and Computer-Integrated Manufacturing (2021), DOI [10.1016/j.rcim.2021.102200](https://doi.org/10.1016/j.rcim.2021.102200), optimizes robot speed along a preselected CAD inspection path using image quality feedback. It reports up to 53% cycle-time reduction from its initial manually specified trajectory without image-quality degradation in its experimental setup. That is a source-specific result, not a universal target.
2. Staderini, *Integrated Planning and Optimization Framework for Robotic Visual Inspection* (TU Wien, 2026), already composes viewpoint generation, non-uniform mesh sampling density, BRDF-based photometric visibility, kinematic pose refinement and SCGTSP/ILP route planning. [DOI 10.34726/hss.2026.138640](https://doi.org/10.34726/hss.2026.138640).

The testable differentiation hypothesis is narrower: make the temporal parameterization and acquisition settings **evidence-gated per engineering characteristic**, with acquisition-time calibration, shutter/clock integrity, photometry, sampling, feature conditioning, uncertainty, resource feasibility and explicit abstention composed through versioned receipts. This is a hypothesis, not a claim that commercial or academic systems lack integrated planning.

## 3. Reuse owners; do not duplicate their models

| Input / constraint | Existing owner | Co-design role |
|---|---|---|
| Supplied candidate path, collision/route/charging/energy/return reserve | #5920; source comparator #5892 | Hard mission feasibility and path segment IDs |
| Candidate views, support and GSD context | #6925 / #7002 | Frozen acquisition-event geometry and demand |
| Whole-support and conservative occlusion | #7136 | Hard admission of the support to be measured |
| Local sampling density | #7164 | Per-critical-element sampling threshold |
| Material / illumination / clipping / usable region | #7222 | Admitted exposure/illumination option response |
| Feature identifiability and conditioning | #7043 / #7049 | Per-characteristic minimum conditioning |
| Calibration scope and acquisition-time validity | #7339 | Exact calibration-state precondition |
| Exposure, rolling-shutter, trigger and projector timing bounds | #7350 | Temporal acquisition hard constraint |
| Measurand covariance and conformity guard band | #6846 | Characteristic-level decision constraint |
| Dependency-qualified evidence diversity | #7341, consuming #4258 / Mycelix #5762 | Optional downstream evidence-path requirement; INDETERMINATE until complete owner evidence is available |

Each candidate acquisition option must carry exact source receipt identities, revisions and evidence classes. A planner must not reconstruct a missing upstream PASS from raw IDs or substitute a default value for missing model output.

## 4. Bounded v0.1 finite model

Start with a fixed path and a finite option universe. This is a reference problem for exactness, not a general robot dynamics optimizer.

### 4.1 Path and motion choices

The caller supplies an ordered list of path segments. Each segment has:
- stable path and segment IDs / revision;
- length in an explicit unit;
- a finite set of admitted speed choices;
- supplied duration and motion-energy values for each speed choice, or a declared exact fixed-point cost formula;
- segment transition constraints (acceleration / speed change) from the robot owner;
- exact collision / reachability / robot-profile receipt references.

The co-design solver does not invent robot kinematics or infer collision freedom. An option absent from the supplied motion envelope is not considered.

### 4.2 Acquisition events and options

Each required acquisition event binds:
- target characteristic, support, path segment and candidate view;
- required calibration, clock-map, shutter, sensor and projector configuration;
- one or more finite options with a segment-speed ID, exposure duration, illumination/strobe mode, optional projector pattern and dwell;
- separate motion, photometric, sampling and feature-conditioning receipts;
- separate robot dwell / acquisition overhead, illumination energy and projector energy;
- exact source receipt digests and profile revisions.

Each option is admissible only when every required upstream owner reports an admissible result for the exact combination of path/speed, exposure, shutter/readout, timestamp, illumination and pattern. A receipt for one exposure or one robot speed must not be reused for another.

### 4.3 Non-compensatory hard constraints

A schedule is feasible only when, for every required characteristic and every required acquisition:
- calibration is valid for the exact instrument/configuration at acquisition time (#7339);
- motion blur, rolling-shutter and timing bounds are inside the declared error budget (#7350);
- projector pattern timing is unambiguous if structured light is used;
- photometric usable region / exposure / saturation / underexposure constraints pass (#7222);
- spatial sampling density and minimum distinct admitted observations pass per critical element (#7164 / #7002);
- feature conditioning / identifiability is adequate (#7043);
- characteristic-level metrology and conformity constraints pass where required (#6846);
- robot path, speed transition, collision, energy, charging and return-reserve constraints pass (#5920).

Unknown, stale, contradictory or unsupported inputs make that option inadmissible or indeterminate. There is no hidden default to “valid”.

**No compensation rule:** more rays cannot compensate for saturated pixels; shorter exposure cannot compensate for an invalid calibration; geometric coverage cannot compensate for an unidentifiable feature; a high average quality cannot compensate for one under-measured critical characteristic; reduced cycle time cannot compensate for a failed return-reserve constraint.

### 4.4 Pareto objectives

Keep objective dimensions separate:
- elapsed inspection time / makespan;
- robot motion energy;
- illumination / strobe energy;
- projector energy;
- number of valid frames and distinct admitted views;
- retake / rescan and requalification count;
- measured uncertainty / conformity outcome from the metrology owner;
- human review burden and lifecycle cost when supplied by an admitted owner.

Do not assign generic weights to make incomparable objectives disappear. The evaluator returns the nondominated feasible schedules. A later policy may choose among that frontier under an explicitly owned decision policy.

## 5. Exact v0.1 algorithm

For bounded instances:
1. Validate all identities, schemas, profile revisions, evidence classes and receipt references.
2. Build the finite set of path-speed combinations and event acquisition options.
3. Reject all options that fail any required precondition before optimization.
4. Enumerate the remaining joint speed/acquisition schedules (bounded product limit declared in the input).
5. Enforce path-segment transition constraints, per-characteristic demand and mission resource constraints.
6. Compute the exact multidimensional objective vector and retain the Pareto frontier.
7. Return one of: OPTIMAL_WITHIN_DECLARED_FINITE_UNIVERSE, INFEASIBLE, INDETERMINATE or SEARCH_LIMIT_NOT_PROVEN.
8. Emit a deterministic receipt with every considered option, every rejection reason, every surviving schedule, every dominated schedule and the exact frontier.

If the enumeration limit is reached, do not label the current best schedule optimal. Preserve the incumbent and return SEARCH_LIMIT_NOT_PROVEN.

A separate standard-library Python oracle must independently enumerate small challenge cases and reconstruct feasibility and Pareto dominance. Production remains Rust-first.

## 6. Frozen challenge families

1. **Speed/exposure trade-off:** fast path + short exposure versus slow path + long exposure; both may be feasible, with different time/energy Pareto positions.
2. **Motion smear:** high-speed option violates #7350; a slower option or shorter exposure remains admissible.
3. **Underexposure trade-off:** short exposure passes motion but fails #7222; a strobe/illumination option may restore photometric usability at higher energy.
4. **Rolling shutter:** exposure smear passes but row-time distortion fails.
5. **Projector transition:** a camera exposure overlaps a projector pattern transition; that option must be rejected.
6. **Calibration invalidation between plan and capture:** stale schedule rejected or explicitly requalified; no plan-time PASS reuse.
7. **Non-uniform sampling:** a fast cadence leaves one critical large element below required density; global average cannot hide it.
8. **Feature conditioning:** area and ray density pass but required hole/slot/edge parameter remains ill-conditioned.
9. **Common cause:** additional repeated frames share a dependency and do not manufacture dependency-qualified diversity (#7341).
10. **Energy reserve:** a faster/shorter schedule violates supplied return-to-safe-state reserve.
11. **Pareto non-domination:** two schedules trade motion energy against illumination energy and elapsed time; preserve both if neither dominates.
12. **No feasible schedule:** all options violate at least one hard requirement; report the binding reasons and abstain.
13. **Scalarization trap:** a deliberately misleading weighted score prefers an invalid option; the hard-constraint evaluator rejects it.
14. **Exact boundary:** equality at a declared motion, energy, sampling or feature threshold is tested with explicit inclusive/exclusive semantics.
15. **Input permutation:** candidate order does not alter the canonical frontier or receipt.
16. **Search cap:** resource limit reached; status is SEARCH_LIMIT_NOT_PROVEN, never OPTIMAL.

## 7. Comparator tournament

Use identical path, sensor, calibration, candidate options, quality thresholds, resources and truth receipts:
- fixed-speed / fixed-exposure baseline;
- fastest nominal schedule ignoring image-quality constraints (negative-control only);
- quality-feedback speed optimization following the 2021 paper's method class;
- sequential path/speed selection followed by exposure tuning;
- exact bounded joint enumeration (v0.1 oracle);
- conservative interval-aware schedule with explicit abstention;
- future receding-horizon policy only after exact bounded semantics are stable.

Report separately:
- elapsed time / makespan;
- robot motion, illumination and projector energy;
- accepted characteristics, valid frame count and false-valid/false-hold;
- exposure / rolling-shutter / timing margin;
- underexposure, saturation and photometric usable fraction;
- sampling density and feature conditioning per critical element;
- measurement uncertainty / conformity disposition;
- requalification, retakes, human review and lifecycle cost;
- exactness status, Pareto frontier size, explored states, runtime and memory.

The published 53% improvement from the 2021 paper belongs to that paper's experiment; do not use it as a Symtropy performance claim or assume its parameters are a matched baseline.

## 8. Prospective targets

- 100% Rust / independent-oracle agreement on the frozen exact corpus.
- Exact Pareto frontier on every bounded fixture.
- Zero schedules accepted when any required hard gate fails.
- Zero hidden critical-element under-density, feature-conditioning shortfall or invalid acquisition.
- Zero robot collision, energy or return-reserve hard violations.
- Explicit retention of all rejected, infeasible, held and indeterminate options.
- Strictly lower elapsed time or lifecycle cost than a sequential baseline on at least one preregistered matched case, without degrading any mandatory evidence or quality constraint.

These are proposed research targets, not achieved results.

## 9. Physical pilot and claim ceiling

A physical pilot requires a supplied inspection path, calibrated robot and camera, exact shutter/readout model, hardware trigger logging, photometric coupons, power measurement for motion and illumination, and independent reference metrology for selected characteristics. Pre-register the path, speed/exposure/strobe options, calibration policy, quality limits, truth sources, held-out conditions and statistical analysis.

Synthetic v0.1 PASS means only that the exact finite schedule satisfies its declared synthetic hard constraints and Pareto semantics. It does not establish physical robot safety, image quality, calibration, metrological accuracy, defect POD, conformity, structural acceptance, certification or flight readiness.
