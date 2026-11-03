# Construction Robot Safety Envelope V1

A dependency-free Rust reference evaluator for the public Symtropy research
programme in [CONSTRUCTION-ROBOT-SAFETY-001, issue #3317](https://github.com/Luminous-Dynamics/symtropy/issues/3317).

## Research question

Can a portable, machine-specific safety-envelope contract turn the recurring
requirements of construction-robot safety into deterministic, inspectable
simulation evidence without pretending to be a real safety controller?

V1 binds a runtime observation to the exact policy, machine/as-built revision,
tool, work package, site snapshot, coordinate frame, jurisdiction, standards
profile, operational domain and a digest-bound site coverage grid. The evaluator then checks:

- emergency-stop state and non-resettable emergency recovery boundary;
- mode permissions, operator override and acknowledged handover;
- policy validity, observation age, clock uncertainty and communications
  heartbeat freshness;
- critical sensors, calibration, confidence, coverage, age and independent
  fault domains;
- human-presence evidence bound to the exact contributing sensor IDs, its own
  source digest, timestamp, confidence and spatial coverage;
- a site-specific required-cell map for the human hazard zone, identified by a
  SHA-256 grid reference bound to the site frame/revision; every required cell
  must be covered by the fused evidence and independently covered by the
  required number of sensor modalities and fault domains;
- required ODD conditions with explicit units, bounds, source references and
  maximum ages;
- human separation using relative approach speed, reaction time, stopping
  distance and an uncertainty margin;
- mode-specific speed and acceleration limits.

The evaluator emits a typed disposition, reasons and a requested response
class. Each decision records the exact expected policy subject and the observed
runtime subject separately, along with the policy digest reference, operating
mode, evaluation/observation timestamps and sorted source references for the
coverage grid, every sensor, human-detection receipt and ODD condition. The
coverage-grid digest is an opaque reference intended to pin the declared
site-local coordinate frame, grid revision, origin/resolution metadata and cell
identities. The code checks reference equality and cell-set logic; it does not
fetch the grid bytes, verify geometry, ray-cast sensor fields of view, model
occlusion, prove calibration or authenticate signatures. The references are
not byte-authentication receipts. The evaluator never sends a
command or changes a physical state.

## Paradigm shift

The first-generation abstraction is not "distance to nearest person". A safe
operating claim is conditional on a complete, versioned tuple:

`machine + tool + task + site + ODD + sensor evidence + operating mode + policy`

The same clearance distance can be admissible for one configuration and
inadmissible for another because stopping distance, tool envelope, movement
energy, human approach speed, uncertainty, sensor coverage and jobsite context
differ.

The key shift is from a static envelope to a **revision-bound, evidence-aware
operating-state contract**. A `NoHumanDetected` state is not a free-standing
boolean: it requires an explicit coverage receipt tied to contributing sensor
IDs, source digest, freshness and confidence. More importantly, percentage
coverage is not enough. The policy defines the exact cells in the hazard zone;
the fused receipt must cover every required cell, and the required number of
sensor modalities and independent fault domains must each cover the entire
zone. A cell cannot be counted as observed unless at least one named source
sensor reports it.
Runtime confidence is not a safety function. Missing or contradictory evidence
is not converted into a safe observation.
A learned policy cannot change thresholds, standards applicability or
emergency-stop state.

## Decision semantics

| Disposition | Meaning in this synthetic evaluator |
|---|---|
| `WithinSyntheticEnvelope` | Supplied evidence is inside the declared synthetic profile. |
| `RequireProgressiveMitigation` | Conditions remain supportable only after the declared mitigation/reassessment path. |
| `HoldForHumanReview` | A handover, override or emergency recovery boundary requires external review. |
| `StopRequiredBySyntheticPolicy` | A hard-stop condition is present in the declared test policy. |
| `EvidenceInconsistent` | Identity, digest syntax or evidence structure is inconsistent. |
| `UnsupportedOdd` | The declared ODD, standards profile or policy validity does not cover the observation. |

The response is an advisory label, not a command. Even
`RequestControlledStop` does not actuate equipment. Emergency-stop reset is
outside this crate: an observation after emergency mode remains held for
external authority and this evaluator cannot clear the latch.

## Frozen corpus

The V1 corpus contains 58 named synthetic cases and 46 authored Rust tests. It includes:

- clear and near-boundary observations;
- separation breach and fast human approach;
- stale critical localization;
- degraded peripheral tool-encoder evidence while the human-detection
  sensor modalities remain healthy;
- common-mode sensing failure and insufficient independent fault domains;
- contradictory, unknown, stale and low-confidence human tracking;
- human-detection claims with inadequate percentage coverage, uncovered zone
  cells, unsupported fused cells, duplicate/unknown cells, mismatched grid
  revisions, stale receipts, missing sensor identity, common-mode evidence and
  single-modality evidence;
- speed, acceleration and maintenance-mode violations;
- heartbeat loss, clock uncertainty and future/stale observations;
- exact machine, tool, task, site, policy and standards bindings;
- marine wave, surface dust and missing/stale ODD evidence;
- mode handover, override, emergency-stop and post-stop recovery cases;
- malformed or duplicated sensor identities and evidence digests.

Expected dispositions are pinned as test data and are not computed by the
production evaluator. The corpus runner prints one deterministic row per
case and exits non-zero if a disposition differs or a negative control becomes
`WithinSyntheticEnvelope`.

Run locally, independently of hosted CI:

```sh
cargo fmt --manifest-path research/construction-robot-safety-envelope-v0-1/Cargo.toml -- --check
cargo test --manifest-path research/construction-robot-safety-envelope-v0-1/Cargo.toml --locked
cargo run --manifest-path research/construction-robot-safety-envelope-v0-1/Cargo.toml --locked --bin safety-envelope-benchmark
cargo clippy --manifest-path research/construction-robot-safety-envelope-v0-1/Cargo.toml --locked --all-targets -- -D warnings
```

The dedicated workflow pins Rust 1.96.0 and runs on Linux, macOS and Windows.

## Standards and state of the art

This crate does not claim compliance with any standard. Applicability is
machine-, task-, jurisdiction- and integration-specific.

- The [ISO 19014:2026 series](https://www.iso.org/ics/53.100/x/) for earth-moving
  machinery was published in October 2026: [Part 1](https://www.iso.org/standard/87988.html)
  specifies the methodology for determining required machine performance levels
  (MPLr); [Part 2](https://www.iso.org/standard/87718.html) covers hardware and
  architecture requirements and evaluation of achieved machine performance
  levels (MPLa); [Part 3](https://www.iso.org/standard/88622.html) specifies
  environmental testing of safety-related electrical/electronic components;
  [Part 4](https://www.iso.org/standard/87720.html) addresses safety-related
  software development and signal transmission; and
  [Part 5](https://www.iso.org/standard/87719.html) provides normative MPLr
  tables by common function and machine type. The relevant scope is earth-moving
  machinery defined in ISO 6165. MPLr/MPLa are not interchangeable labels for
  ISO 13849 PLr/PL or IEC 61508 SIL, and this evaluator does not calculate any
  of them. The series covers hazards caused by safety-control-system failure,
  not every hazard arising from the equipment itself.
- [ISO 17757:2019](https://www.iso.org/standard/76126.html) remains current
  after 2024 confirmation and directly addresses autonomous/semi-autonomous
  earth-moving and mobile-mining machine systems and their defined environments.
- [ISO 10218-1:2025](https://www.iso.org/standard/73933.html) and
  [ISO 10218-2:2025](https://www.iso.org/standard/73934.html) are current
  industrial-robot and application/cell safety standards. Their scope should
  not be assumed to cover every construction machine or jobsite integration.
- [ISO 3691-4:2023](https://www.iso.org/standard/83545.html) is the published
  driverless-industrial-truck standard. Its third edition is under development
  as ISO/DIS 3691-4 at stage 40.60; the draft is not a published standard or a
  silently applicable legal requirement ([ISO project status](https://www.iso.org/standard/88615.html)).
- [ISO 12100:2010](https://www.iso.org/standard/51528.html) remains the
  published general machinery risk-assessment and risk-reduction framework;
  [ISO 13849-1:2023](https://www.iso.org/standard/73481.html) addresses
  safety-related parts of control systems. Neither is replaced by this evaluator.
- [ANSI/A3 R15.08-1-2020 (R2026)](https://webstore.ansi.org/standards/ria/ANSIA3R15082020R2026)
  and R15.08-2:2023 are relevant industrial-mobile-robot manufacturer and
  application/integration references, not a universal heavy-construction profile.
- The [2026 ISARC analysis of 12 standards](https://doi.org/10.22260/ISARC2026/0104)
  identifies human-machine separation, mode-aware behaviour, sensor monitoring,
  fail-safe response, progressive mitigation and context-dependent validation.
- [NIST's Mobility Performance of Robotic Systems programme](https://www.nist.gov/programs-projects/mobility-performance-robotic-systems)
  is developing metrics, test methods, artefacts and datasets for mobility,
  adaptability, task efficiency, completion quality and assurance in dynamic environments.
- A [2026 systematic review of 215 construction HRC studies](https://doi.org/10.1016/j.autcon.2026.107288)
  reports that 54% remain at controlled-laboratory validation and identifies
  weak integration of uncertainty across sensing, prediction, control and supervision.

These findings support a benchmark programme, not a claim that this small
reference evaluator is safer or more capable than deployed industrial
safeguards.

## Commercial deployment context (not a matched safety comparison)

Public vendor and programme reporting shows that construction robotics has
moved beyond laboratory demonstrations in several bounded workflows. The
figures below describe different tasks and conditions; they must not be
combined into a league table.

| System / task | Publicly reported evidence | Correct interpretation |
|---|---|---|
| [HP SitePrint](https://www.hp.com/us-en/printers/site-print/layout-robot.html) — floor layout | HP specifies up to ±2 mm layout tolerance and ±0.8 mm average floor-level accuracy under a high-accuracy setup at 5–30 m. HP also advertises up to 10× productivity; the separately footnoted up-to-30% increase compares its prior software version, not a controlled manual-labour study. | Useful survey/layout quality reference. Reproduce its exact tracker, total-station, distance, file and surface conditions before comparing residuals or productivity. |
| [Dusty FieldPrinter 2](https://support.dustyrobotics.com/hc/en-us/articles/52682349645851-FieldPrinter-Specs) — floor layout | Dusty specifies ±1/16 in (±1.6 mm) print accuracy with surveyed control, 1.0 m/s navigation with obstacle sensors enabled, three forward IR time-of-flight sensors with about 0.9 m range, and separate cliff sensors. Its specifications warn that the cliff sensors may not detect narrow openings deeper than 6 in because of their field of view. | A valuable real product operating-envelope reference. The published specification is not a complete third-party functional-safety assessment; benchmark the declared detection geometry and edge cases, not a generic "safe" label. |
| [Built Robotics / ARENA](https://arena.gov.au/knowledge-bank/built-robotics-australian-robotic-piling-deployment-for-large-scale-solar-lesson-learnt-report-1/) — solar pile installation | The Australian 5 MW pilot report records 1,723 piles installed by Gen 1. ARENA's October 2026 scale-up project targets domestic manufacturing capacity for 12 robot sets/year and describes future production / labour-reduction estimates as project outcomes to be achieved. | Strong field-deployment comparator for pile cycle, first-pass engineering acceptance, survey error, rework, intervention, crew-hours and night/site-condition limits. Separate demonstrated pilot results from future programme projections. |
| [FBR Hadrian](https://investors.fbr.com.au/announcements/xzoJwm-fbr-annual-report-2026) — robotic masonry | FBR's 2026 annual report reports an effective rate above 285 blocks/hour and a record near 36 m² of wall/hour using specified large-format Porotherm blocks. | Vendor-reported result for a particular block, wall and production arrangement. Compare quality acceptance, corners/openings, downstream readiness, setup and crew composition before throughput. |

The commercial lesson is not that one general-purpose robot wins. Current
systems are task-specialized, configured around purpose-built workflows,
survey controls, materials, site access and operator processes. A credible
comparison therefore needs a **matched task contract** and the full resource
boundary: robot + crew + survey + setup + consumables + maintenance + rework +
safeguards + interruption + downstream inspection.

For this safety-envelope package, the product figures above are context only.
The relevant head-to-head measurements are detection coverage, false-safe
classification, stopping-distance residuals against instrumented ground truth,
protective-stop latency, nuisance-stop rate, fault recovery, sensor common-mode
detection and evidence traceability. Public product brochures do not establish
these quantities, so this README does not infer that any named competitor
passes or fails them.

## Comparison plan

Compare matched observations and the same declared machine/task/site profiles
against these baselines. First select the applicable machine-family framework:
ISO 19014 for in-scope earth-moving machinery, ISO 17757 for autonomous and
semi-autonomous earth-moving/mining systems, ISO 3691-4 or ANSI/A3 R15.08 for
in-scope driverless/industrial mobile robots, and ISO 10218 for industrial robot
applications. A standards applicability matrix must be completed by competent
engineers for the actual configuration and jurisdiction; merely storing a
standard identifier is not a conformity assessment.

| Baseline | Policy |
|---|---|
| B0 | Fixed conservative separation and hard-stop envelope. |
| B1 | B0 plus operating-mode-specific motion limits. |
| B2 | B1 plus freshness, calibration, confidence, coverage-grid binding, complete required-cell coverage and per-modality/per-fault-domain zone coverage. |
| B3 | B2 plus progressive mitigation, handover and explicit external recovery boundary. |
| B4 | Independently adjudicated expected dispositions for the frozen corpus. |

Report, never collapse into one score:

- false-within-envelope count and rate (target: zero on hard-stop controls);
- required-zone cell coverage recall, uncovered-cell detection and false
  coverage claims (target: zero unsupported cells);
- complete-zone coverage for each required modality and independent fault domain;
- detection of unknown/duplicate cell identities and coverage-grid revision drift;
- hard-stop and hold detection recall (target: 100% on frozen controls);
- false-stop / unnecessary interruption rate;
- stale, missing, contradictory, misbound and out-of-domain detection;
- time from evidence timestamp to disposition (p50/p95/p99 on timed runs);
- decisions by machine family, ODD, mode, sensor modality and fault domain;
- mitigation correctness, recovery review and handover correctness;
- deterministic replay and input-order invariance.

The software acceptance targets are exact reference agreement, zero false-safe
classifications on the frozen negative controls, zero unsupported covered cells,
full required-zone coverage by each required modality and fault domain, no
autonomous emergency-stop reset, and no actuator output. These are finite
software-contract targets, not physical safety rates.

## How to go beyond current practice

1. **Uncertainty as a first-class safety input.** Carry source age, calibration,
   coverage, coordinate frame, clock error, fault-domain dependence and
   measurement uncertainty all the way to the decision.
2. **Space-time coverage, not a single confidence score.** Track exactly which
   required hazard-zone cells are covered, by which sources, in which modalities,
   under which fault domains and at what source time. Percentage confidence
   cannot repair a blind region.
3. **Temporal and causal fault injection.** Next versions should replay event
   streams with dropped, delayed, reordered, duplicated and contradictory
   observations, rather than only testing independent static snapshots.
3. **Machine-family profiles.** A surface mobile manipulator, TBM, dredger,
   crane and precast cell require different hazard variables, risk analyses
   and applicable standards. Never clone a profile by changing its label.
4. **Independent assurance.** A separate evaluator should reconstruct
   dispositions from the frozen policy and evidence, while a real safety
   controller independently owns protective functions.
5. **Sim-to-real qualification.** Only matched instrumented trials can measure
   detection performance, stopping distance, nuisance-stop rate, exposure and
   discrepancy between simulated and physical sensing.

A meaningful industry comparison must use the same machine, payload, tool,
site, operating mode, task, safeguard configuration and acceptance criteria.
For earth-moving machines, benchmark required and achieved machine performance
levels and software/data-transmission safety evidence under the applicable
ISO 19014 framework; for mobile robots and industrial cells, use the relevant
standard-specific safety functions and integration evidence. Never compare a
synthetic classification rate to an achieved MPL/PL/SIL or infer certification
from this crate. Vendor marketing figures and synthetic oracle accuracy are
not comparable with measured jobsite productivity or certified protective
performance.

## Ownership boundaries

- Symthaea owns robot/platform design, capability qualification and field
  evidence (see construction fleet issue #9870).
- Symtropy owns this portable synthetic safety-envelope evaluator and the
  physics/simulation-side test contracts. Existing per-joint
  `JointSafetyAuthority` remains a separate bridge component; V1 does not
  replace it.
- [ROBOT-QUAL-ORACLE-001 #2672](https://github.com/Luminous-Dynamics/symtropy/issues/2672)
  owns end-to-end fault-to-disposition qualification across the full mission
  timeline. This crate is only a per-observation safety-envelope sub-oracle;
  it does not declare a mission Complete, Blocked or safe to execute.
- [Shared robotics scenario pack #1595](https://github.com/Luminous-Dynamics/symtropy/issues/1595)
  owns common scenario identity/fault schedules; [ENG-SYNTH #1172](https://github.com/Luminous-Dynamics/symtropy/issues/1172)
  owns deterministic execution and replay. A temporal V2 must compose those
  owners rather than create another event runtime.
- Symtropy issue #1834 and the construction qualification evaluator own
  machine-to-work-package capability matching, not runtime safety control.
- Mycelix owns identity, authorization, custody, provenance, correction,
  revocation and disputes where its authorities are explicitly integrated.
- Sol-Atlas provides spatial/site context and projection, not safety authority.
- Qualified machine safety systems, safety PLCs, emergency stops, human
  operators, responsible engineers and competent authorities retain all
  physical safety and approval functions.

## Claim ceiling

All values and fixtures in V1 are synthetic. This crate validates its own
narrow contract and expected classifications only. It does not fetch or
authenticate evidence bytes, calculate SIL/PL, conduct a risk assessment,
interpret all clauses of any standard, validate a sensor model, prove a
stopping-distance model, certify a robot, authorize work, operate a safety
PLC, actuate a stop, clear an emergency stop, or establish physical safety.

A syntactically valid SHA-256 reference is an identifier, not proof that the
underlying bytes are authentic or true. Field deployment requires a
machine-specific risk assessment, competent safety engineering, applicable
standards/regulatory analysis, safety-rated hardware/software and independent
physical validation.
