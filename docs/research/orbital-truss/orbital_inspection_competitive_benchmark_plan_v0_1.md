# Orbital robotic inspection: competitive benchmark plan v0.1

**Evidence cutoff:** 2026-11-25  
**Purpose:** define a fair, falsifiable route from synthetic exact policy semantics to matched industrial and orbital-analogue evaluation.

## Executive position

The opportunity is not to claim that a new planner already beats industrial metrology or robotic inspection. Established industrial systems are strong at calibrated acquisition, repeatable robot programs, 3D measurement, and feature/surface workflows. Recent research also demonstrates substantial path and cycle-time reductions.

The gap worth attacking is the full decision loop: **design for inspectability → plan → qualify the actual acquisition → measure with traceable uncertainty → decide whether another observation changes the engineering decision → re-plan under resource and evidence constraints → preserve independently replayable evidence**.

The differentiator must be tested as a system property. A faster route is not better when it misses a feature; more images are not better when they add no decision value; a calibrated sensor is not enough when a change event invalidates its scope; a digital twin is not an independent measurement; and a digest is not proof that source bytes or issuers are authentic.

## What current published work establishes

### Autonomous industrial inspection

The 2026 systematic review *Autonomous Industrial Inspection with Quadruped Robots* included 32 physically validated studies. Four studies let inspection data re-plan the mission; only two re-planned within the current run. Only one of those four compared mission replanning against a no-replanning baseline. The review also reports only three studies quantifying localization accuracy on hardware, 14 without repeated-run statistics, and 27 without public code or data. These are counts from this specific quadruped industrial-inspection review, not from all robotics research and not from orbital systems.

Source: https://www.mdpi.com/2218-6581/15/10/204

### Robotic surface-inspection trajectory planning

Zou et al. (2026), *Human-Inspired Dexterity-Oriented Perception and Trajectory Optimization for Robotic Surface Inspection*, reports a physical experiment with 100% clarity coverage of the defined key surface areas, a 47.0% path-length reduction and a 37.5% solution-time reduction over its baseline. The article reports ten repeated physical runs and mean ± standard deviation. These are strong matched-baseline targets to reproduce on the same asset, geometry, sensor, candidate views and evaluation rules—not universal performance guarantees.

Source: https://doi.org/10.3390/biomimetics11050296

### Traceable robot metrology

Chekh et al. (2026), *Traceable Metrological Digital Twin for Robot-Based 3D Inspection*, integrates robot kinematic calibration, optical-sensor characterization, sensor-to-robot referencing and machine-readable metrological state. It propagates calibrated-parameter uncertainty using a GUM-aligned Monte Carlo method and compares a complex part against references at 67 characteristics. Its current validation is limited to the experimental campaign's environmental conditions and short-term stability; thermal drift and long-term stability are explicitly outside its current model.

Source: https://doi.org/10.3390/metrology6040088

### Commercial metrology bar

Manufacturer-published capabilities show why we must not reduce the comparison to a toy path-planning baseline:

- ZEISS describes TALOS as a metrology robot with claimed measurement uncertainty below 100 µm at a 3.30 m reach, no external tracking or reference markers, and one sensor workflow for full-field geometry and feature measurement. These are manufacturer specifications, not an independent Symtropy-vs-ZEISS trial.
  - https://www.zeiss.com/metrology/en/about-us/newsroom/2026/automated-inspection.html
  - https://www.zeiss.com/metrology/en/systems/optical-3d/automated-solutions/talos.html
- Creaform publishes MetraSCAN-R BLACK+ Elite accuracy of 0.025 mm and volumetric accuracy values of 0.064 mm over 9.1 m³ and 0.078 mm over 16.6 m³. Those figures are tied to the vendor's stated equipment and test conditions; they are not directly comparable with ZEISS's uncertainty claim or a different robot/sensor/workpiece.
  - https://www.creaform3d.com/en/products/r-series-automated-quality-control-solutions/technical-specifications

The comparison should therefore include the actual complete measurement chain, feature-dependent errors, calibration/reference procedure, uncertainty, environment, access, repeatability and traceability—not only the nominal sensor accuracy.

### Orbital inspection and servicing

NASA's VIPIR is described as a tele-operated visual inspection system using an articulating borescope to access spacecraft components beyond fixed-camera views. NASA's Robotic Servicing Arm has seven degrees of freedom and heritage in robotic servicing. These are relevant space-robotics capabilities, but neither source establishes a general autonomous evidence-qualified inspection campaign for a complex orbital truss.

Sources:
- https://technology.nasa.gov/advancing-commercial-space/autonomous_systems_robotics
- https://www.nasa.gov/isam/robotic-servicing-arm/

### Probability of detection and conformity

NIST TN 2119 lays out confidence intervals and confidence bounds for probability of detection, probability of identification and false-alarm rates. POD is a physical, method-specific qualification question; synthetic policy fixtures cannot establish it.

Source: https://doi.org/10.6028/NIST.TN.2119

## Fair comparator tournament

Every arm must receive the same geometry, target characteristics, candidate-view pool, hardware limits, sensor model, calibration state, evidence prefix, resource budget and evaluation truth.

| Arm | Comparator | What it isolates |
|---|---|---|
| B0 | Fixed/teach-and-repeat route | Operational baseline |
| B1 | Conventional coverage / viewpoint planner | Coverage and path planning |
| B2 | Matched 2026 dexterity-oriented hierarchical planner | Published surface-inspection planning baseline |
| B3 | Fixed recapture sequence | Non-adaptive inspection |
| B4 | Greedy information gain per cost | Entropy reduction versus actual decision value |
| B5 | Greedy worst-case decision-class separation | Decision-oriented heuristic |
| B6 | Evidence-gated recourse | Proposed deployable policy |
| B7 | Exhaustive finite policy oracle | Exact optimum on small support models only |
| H | Hindsight/clairvoyant action selection | Evaluator-only upper bound; never a deployable competitor |

For the surface-inspection paper, first reproduce its own baseline and reported metrics under its geometry and sensor protocol. Then add the recourse objective on a separate, matched extension. If its raw data, CAD or implementation are unavailable, mark the comparison as not reproducible; do not infer a numerical comparison from the abstract.

## Metrics to report independently

**Planning and execution:** path length, cycle time, solver time, reachable-view ratio, collision-free completion, localization error, intervention frequency, route stability and recovery time.

**Acquisition and sensing:** capture association success, timing uncertainty, smear/blur, illumination and projector stability, occlusion, image clarity, valid coverage per required characteristic, repeat-acquisition rate and sensor dropout.

**Metrology:** characteristic-specific bias and repeatability, reference-method comparison, expanded uncertainty, uncertainty coverage, calibration invalidation detection delay, drift response and environment-specific performance.

**Decision quality:** false engineering-decision commitment, false rejection, correct resolved decisions, appropriate abstention, unresolved decision-set size, decision regret, wasted observations and actions per resolved characteristic.

**Evidence assurance:** exact input/source binding, completeness, stale/revoked reference rejection, common-cause handling, deterministic replay, independent-oracle parity, audit completeness and policy non-anticipativity.

**Resources:** worst-case/median time, robot energy, illumination/projector energy, compute, data volume and mandatory return reserve.

Do not combine these into one weighted score. Report Pareto fronts and hard-gate violations separately.

## Staged benchmark ladder

### Stage 0 — finite synthetic semantics

Targets for the current executable work:
- 100% independent exact-oracle agreement on bounded known-answer cases;
- zero UNKNOWN, FAIL, incomplete or unqualified acquisition actions selected;
- zero false singleton decisions on opposite-decision support;
- zero extra observation when every surviving world already implies the same decision;
- zero use of future outcomes or hidden world IDs;
- zero exact-optimum claims after a search cap;
- byte-deterministic Rust replay and complete independent receipt equality.

These are software contract targets only.

### Stage 1 — simulator / digital-twin matched experiments

Use a frozen CAD model, explicit surface characteristics, known truth, realistic sensor and robot models, occlusion, calibration drift, lighting, thermal change, timing faults and common-cause dependencies. Compare every baseline on the same initial candidate pool and same budgets. Use paired runs and publish mean, spread, confidence intervals and failure strata.

### Stage 2 — physical lab campaign

Use independently measured reference truth (for example, CMM or a qualified reference scanner) and a preregistered repeated-run protocol. Include multiple robot poses, sessions, calibration events, temperature states, sensor/remount cycles and deliberate capture failures. Measure feature-level error and uncertainty—not only point-cloud overlap. Only after the method-specific physical dataset exists should POD / false-alarm claims be made using an appropriate method such as the NIST guidance and applicable ASTM/NASA requirements.

### Stage 3 — orbital analogue and environmental qualification

Introduce representative truss geometry, long-baseline structure flexure, lighting/shadow, thermal gradients, vacuum-compatible hardware, communications delay, constrained contact/access, power reserve, radiation/thermal aging where relevant, and safe degraded-mode behavior. Hardware-in-the-loop and analog testing precede any flight or operational claim.

## How to outperform, rather than just be different

The stretch objective is a **non-dominated decision-assurance frontier**: at equal resource cost, obtain more correctly resolved required characteristics with no increase in false decision commitments; or reach the same decision quality with lower path length, energy, observation count or elapsed time. At equal inspection quality, improve cycle time and energy against the matched industrial baseline. At equal path and image quality, reduce unnecessary reacquisition and false confidence by tracking calibration, timing, evidence completeness and common causes.

A meaningful win must hold across repeated runs and hard scenarios—not only average cases. Report worst-case behavior and tail failures. Keep the 2026 surface-inspection paper's 47% path and 37.5% solution-time improvements as serious reproduction targets, not figures that can be compared without matching its experiment.

## Current implementation mapping

- #7384: bounded acquisition integrity (synthetic evaluator).
- #7382: calibration-chain validity (synthetic evaluator).
- #7388: exact finite motion/exposure/illumination co-design (synthetic evaluator).
- #7394: exact finite contingent inspection recourse (synthetic evaluator; qualification pending).
- #6887: statistical sequential world-survival / anytime-valid decision sets.
- #6845: parent inspectability and independent evidence programme.

The implementation sequence should connect these contracts only through versioned, exact input and output receipts. Missing owners remain missing; synthetic or modeled evidence must never be presented as measured physical truth.
