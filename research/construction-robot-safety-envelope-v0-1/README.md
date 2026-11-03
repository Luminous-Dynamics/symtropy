# Construction Robot Safety Envelope V1

A dependency-free Rust reference evaluator for the public Symtropy research
programme in [CONSTRUCTION-ROBOT-SAFETY-001, issue #3317](https://github.com/Luminous-Dynamics/symtropy/issues/3317).

## Research question

Can a portable, machine-specific safety-envelope contract turn the recurring
requirements of construction-robot safety into deterministic, inspectable
simulation evidence without pretending to be a real safety controller?

V1 binds a runtime observation to the exact policy, machine/as-built revision,
tool, work package, site snapshot, coordinate frame, jurisdiction, standards
profile and operational domain. The evaluator then checks:

- emergency-stop state and non-resettable emergency recovery boundary;
- mode permissions, operator override and acknowledged handover;
- policy validity, observation age, clock uncertainty and communications
  heartbeat freshness;
- critical sensors, calibration, confidence, coverage, age and independent
  fault domains;
- required ODD conditions with explicit units, bounds, source references and
  maximum ages;
- human separation using relative approach speed, reaction time, stopping
  distance and an uncertainty margin;
- mode-specific speed and acceleration limits.

The evaluator emits a typed disposition, reasons and a requested response
class. It never sends a command or changes a physical state.

## Paradigm shift

The first-generation abstraction is not "distance to nearest person". A safe
operating claim is conditional on a complete, versioned tuple:

`machine + tool + task + site + ODD + sensor evidence + operating mode + policy`

The same clearance distance can be admissible for one configuration and
inadmissible for another because stopping distance, tool envelope, movement
energy, human approach speed, uncertainty, sensor coverage and jobsite context
differ.

The key shift is from a static envelope to a **revision-bound, evidence-aware
operating-state contract**. Runtime confidence is not a safety function.
Missing or contradictory evidence is not converted into a safe observation.
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

The V1 corpus contains 42 named synthetic cases. It includes:

- clear and near-boundary observations;
- separation breach and fast human approach;
- stale critical localization;
- degraded camera with independent lidar;
- common-mode sensing failure and insufficient independent fault domains;
- contradictory, unknown, stale and low-confidence human tracking;
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

- ISO 10218-1:2025 and ISO 10218-2:2025 are the current published industrial
  robot safety editions. Their scope should not be assumed to cover every
  construction machine or every jobsite integration.
- ISO 3691-4:2023 is the published driverless-industrial-truck standard. The
  third edition was still a Draft International Standard at enquiry stage
  40.60 on 3 November 2026; a draft must not be described as a published
  standard or silently used as a legal requirement.
- The 2026 ISARC analysis of 12 standards identifies human-machine separation,
  mode-aware behaviour, sensor monitoring, fail-safe response, progressive
  mitigation and context-dependent validation as recurring safety principles.
- NIST's Mobility Performance of Robotic Systems programme is developing
  metrics, test methods, artefacts and datasets for mobility, adaptability,
  task efficiency, completion quality and assurance in dynamic environments.
- A 2026 systematic review of 215 construction HRC studies reports that 54%
  remain at controlled-laboratory validation and identifies weak integration
  of uncertainty across sensing, prediction, control and supervision.

These findings support a benchmark programme, not a claim that this small
reference evaluator is safer or more capable than deployed industrial
safeguards.

## Comparison plan

Compare matched observations and the same declared machine/task/site profiles
against these baselines:

| Baseline | Policy |
|---|---|
| B0 | Fixed conservative separation and hard-stop envelope. |
| B1 | B0 plus operating-mode-specific motion limits. |
| B2 | B1 plus freshness, calibration, sensor coverage, confidence and independent fault-domain checks. |
| B3 | B2 plus progressive mitigation, handover and explicit external recovery boundary. |
| B4 | Independently adjudicated expected dispositions for the frozen corpus. |

Report, never collapse into one score:

- false-within-envelope count and rate (target: zero on hard-stop controls);
- hard-stop and hold detection recall (target: 100% on frozen controls);
- false-stop / unnecessary interruption rate;
- stale, missing, contradictory, misbound and out-of-domain detection;
- time from evidence timestamp to disposition (p50/p95/p99 on timed runs);
- decisions by machine family, ODD, mode, sensor modality and fault domain;
- mitigation correctness, recovery review and handover correctness;
- deterministic replay and input-order invariance.

The software acceptance targets are exact reference agreement, zero false-safe
classifications on the frozen negative controls, no autonomous emergency-stop
reset and no actuator output. These are finite software-contract targets, not
physical safety rates.

## How to go beyond current practice

1. **Uncertainty as a first-class safety input.** Carry source age, calibration,
   coverage, coordinate frame, clock error, fault-domain dependence and
   measurement uncertainty all the way to the decision.
2. **Temporal and causal fault injection.** Next versions should replay event
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
Vendor marketing figures and synthetic oracle accuracy are not comparable
with measured jobsite productivity or certified protective performance.

## Ownership boundaries

- Symthaea owns robot/platform design, capability qualification and field
  evidence (see construction fleet issue #9870).
- Symtropy owns this portable synthetic safety-envelope evaluator and the
  physics/simulation-side test contracts. Existing per-joint
  `JointSafetyAuthority` remains a separate bridge component; V1 does not
  replace it.
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
