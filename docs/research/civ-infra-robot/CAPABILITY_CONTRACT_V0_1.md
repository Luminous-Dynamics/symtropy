# CIV-INFRA Robot Capability Matching Contract v0.1

**Status:** Draft reference contract; documentation and synthetic fixture only.  
**Parent work:** [CIV-INFRA-ROBOT-001](https://github.com/Luminous-Dynamics/symtropy/issues/1834).  
**Fixture:** [robot-capability-v0.1.json](fixtures/robot-capability-v0.1.json).

## 1. Claim boundary

This contract specifies how a construction work package can be compared with a robot/tool capability profile. It does not implement a dispatcher or authorize physical motion.

`design intent != work package != capability match != work authorization != physical execution != accepted as-built work`

A capability match is a proposal-level result. It cannot grant motor authority, certify a machine, waive site controls, establish code compliance, approve structural/hydraulic/thermal engineering, or replace the existing Symtropy Fabrication F0–F12 / Construction C1–C5 path.

All numerical data in the companion fixture are invented for deterministic software tests. They are **not** safe operating limits, vendor specifications, field measurements, or productivity claims.

## 2. Ownership and composition

Reuse existing owners instead of introducing a second source of truth:

- Symthaea owns design intent, robot-design lineage, evidence interpretation and claim ceilings: [robotics design bootstrap #4795](https://github.com/Luminous-Dynamics/symthaea/issues/4795), [built-environment profile #6032](https://github.com/Luminous-Dynamics/symthaea/issues/6032), [civil infrastructure #7697](https://github.com/Luminous-Dynamics/symthaea/issues/7697).
- Symtropy owns supported physical/resource semantics and synthetic verification: [corridor verifier #1732](https://github.com/Luminous-Dynamics/symtropy/issues/1732), [solver-backed drainage follow-up #1764](https://github.com/Luminous-Dynamics/symtropy/issues/1764), and the existing construction lifecycle/staging owners.
- Sol Atlas projects source-qualified spatial information; its projection must not become geometry or engineering authority: [civil corridor projection #119](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/119).
- Mycelix coordinates appropriately authorized organizational commitments and provenance: [construction approvals #5149](https://github.com/Luminous-Dynamics/mycelix/issues/5149).
- The timber-specific portfolio remains at [WOOD-ROBOT-PLATFORM-001 #1818](https://github.com/Luminous-Dynamics/symtropy/issues/1818).

The corridor fixture is the first host. Home/factory tasks and timber robotics are later profiles; they must not be forced into a road-specific physical model.

## 3. Input contract

### 3.1 Capability profile

A capability profile is immutable within a revision and binds at least:

- durable platform and configuration IDs/revisions;
- tool/end-effector identity, tool state, firmware/controller identity;
- supported task classes and material/tool compatibility;
- coordinate frame and bounded workspace/reach;
- capability limits with explicit quantity, unit, comparison direction and validity domain (for example payload maximum or maximum allowed pose error);
- terrain/environment limits and resource assumptions where relevant;
- exact calibration subject, calibration revision/epoch and currentness;
- supporting-evidence kind, provenance, scope, currentness and known limitations;
- required operator role, human approval, safety-interlock evidence source, exclusion zones and recovery prerequisites;
- profile version and canonical digest.

The record must distinguish a synthetic test assertion, vendor statement, calibrated measurement, independent qualification and unknown/conflicted evidence. A profile's own `qualified: true` field cannot independently establish qualification.

### 3.2 Work-package requirements

A work package binds:

- stable task ID/revision and exact site, geometry, frame, materials and input snapshots;
- task class, preconditions, dependency graph, resource requirements and half-open schedule interval `[start, end)`;
- required tool/material compatibility, workspace/reach, payload/load bounds, precision, operating environment and energy reserve;
- required pre-task observations and post-task inspection/test evidence;
- human-approval gates and recovery requirements;
- allowed evidence maturity for every capability on which admission depends.

Requirements may state a lower bound, upper bound, interval, categorical compatibility or explicit supported set. They must not rely on dimensionless comparisons between incompatible units. Missing units, frame, revision, calibration identity or comparison semantics are schema errors, not favorable defaults.

### 3.3 Site/observation state

Known, bounded-unknown, unbounded-unknown, stale and conflicting observations are different. In particular:

- bounded unknowns can support only claims that remain valid across their declared uncertainty envelope;
- an unbounded unknown that intersects a required work/safety envelope blocks a positive compatibility conclusion;
- absence of utility geometry does not prove the trench is clear;
- a plan, render mesh or completion marker does not prove current as-built geometry;
- an old calibration or site snapshot cannot be relabeled current by the consumer.

## 4. Result vocabulary

The verifier returns exactly one primary disposition per assignment:

| Disposition | Meaning |
|---|---|
| `CompatibleForSyntheticCase` | Every modeled hard requirement is supported by the admitted fixture evidence; synthetic tests only. |
| `Incompatible` | Admitted evidence positively establishes a hard mismatch, such as insufficient payload, wrong tool or inadequate reach. |
| `UnknownReviewRequired` | A necessary fact is missing, stale, uncalibrated, below the required evidence maturity or unbounded-unknown. |
| `EvidenceInconsistent` | Required IDs, revisions, digests, units, frames or records contradict one another. |
| `Unsupported` | The task/capability semantics or physical model class is outside this version's declared coverage. |

The evaluator must preserve per-requirement findings and source references. Do not collapse unknown, unsupported or inconsistent into infeasible, and never collapse them into compatible.

Precedence for the primary disposition is deterministic: invalid/conflicting bindings produce `EvidenceInconsistent`; unsupported required semantics produce `Unsupported`; a proven hard mismatch produces `Incompatible`; any remaining required unknown/stale/insufficient evidence produces `UnknownReviewRequired`; only the fully supported synthetic case may produce `CompatibleForSyntheticCase`. Implementations must retain all findings even when one primary disposition is selected.

## 5. Matching and scheduling semantics

A compatibility check is an intersection of requirements with the **exact profile revision**. It must verify:
1. identities/revisions and coordinate-frame compatibility;
2. task/tool/material support;
3. all required bounds and validity domains;
4. calibration/currentness and evidence-maturity requirements;
5. operator, approval, interlock-evidence and exclusion/recovery prerequisites;
6. site unknowns intersecting the requested operation envelope;
7. resource and half-open time-interval conflicts for the proposed schedule;
8. the required inspection/acceptance evidence path.

Task order is not permission to skip prerequisites. A model may return a candidate schedule, but execution admission remains in the existing construction/authority paths. A completed task without its required inspection evidence is not an accepted deliverable.

## 6. Independent verifier requirements

A future executable harness must:
- parse a versioned fixture with strict finite/unit/frame/domain checks;
- derive compatibility findings from canonical input fields, not planner-supplied booleans or digests;
- recompute resource occupancy and reject overlapping use of a capacity-one resource;
- reject duplicate/dangling IDs and stale cross-references;
- preserve unknown/unsupported dispositions and explicit claim ceilings;
- canonicalize findings deterministically while binding report provenance to the exact input bytes;
- test permutation invariance of semantic results separately from exact input-byte identity;
- compare any bounded finite scheduling case with exhaustive enumeration where feasible.

The candidate generator and reference verifier must be separate code paths. Separate functions in one codebase are useful checking but are not independent implementation or third-party audit.

## 7. Frozen synthetic benchmark ladder

### C0 — contract correctness

Targets (not achieved results):
- 100% rejection of seeded hard mismatches in the frozen corpus;
- zero schedule acceptance when a capacity-one robot/tool/resource is double-booked;
- zero accepted-complete dispositions without required inspection evidence;
- 100% visibility of unknown, stale, conflicting and unsupported cases;
- byte-stable normalized report for identical admitted bytes and profile revisions;
- equivalent semantic findings under input-order permutations;
- agreement with exhaustive enumeration for the explicitly bounded toy assignment/schedule domain.

Report denominator, each case outcome, exact reasons, fixture digest, profile revisions and implementation commit. These targets say nothing about real machines.

### C1 — simulation comparisons

On equivalent frozen tasks, compare an explicitly defined human-operated baseline, one-platform scheduling and heterogeneous fleet scheduling where supported. Report independent metrics:

- accepted/qualified work packages per robot-hour and total elapsed hour;
- setup, idle, handoff, charging/refuelling and recovery time;
- first-pass yield, dimensional-error distribution, rework and damage;
- energy/material per accepted functional unit;
- human interventions, blocked tasks, resource conflicts and worst-case outcomes;
- availability, maintenance and local repair time when measured.

Keep safety/quality gates separate from cost, speed, energy, accessibility, serviceability and environmental trade-offs. Preserve a Pareto set, not an opaque universal robot score. Clearly mark synthetic assumptions versus measured observations.

### C2 — supervised physical pilot

A separate future stage requiring exact hardware/configuration identity, risk assessment, calibrated instruments, machine/task-specific safeguarding, qualified operator procedures, independent physical-stop/interlock verification and professional review. Start with low-consequence, reversible work such as stand-off survey or non-load-bearing material logistics. Record sim-to-real residuals and held-out scenarios; no synthetic or simulation pass promotes physical authority.

## 8. Required hostile cases

The first corpus should cover compatible baseline; wrong tool/material; insufficient reach; load/payload overflow; tighter-than-supported tolerance; stale/wrong-subject calibration; unbounded unknown utility/ground; site revision changed after assignment; overlapping schedules; energy reserve failure; missing inspection; localization/communications loss without declared recovery; declined/unavailable human approval; unsupported task class; and attempts to turn a simulated completion/PASS into dispatch, safety, code-compliance or permit authority.

The companion fixture proposes these cases; it is test data, not a completed executable benchmark.

## 9. Extension order

1. Qualify this contract and synthetic capability-match corpus over the existing corridor fixture.
2. Add resource scheduling and independent finite-instance oracle checks.
3. Consume existing timber/wood-robot task profiles after a field-by-field owner audit.
4. Add home/facility and factory service-node profiles through their existing design/construction owners.
5. Add water, energy, data and thermal installation only with their domain-specific physical models and evidence requirements.
6. Treat dams, bridges, pressure systems and other high-consequence infrastructure as separate engineering profiles with specialist analysis, inspections and legal/professional review.

A reusable interface does not imply interchangeable physics or standards. No real-world suitability claim follows from passing a synthetic corpus.
