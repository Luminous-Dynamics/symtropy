# Hand Mission Qualification and Evidence Bundle v0.1

**Tracking issue:** [HAND-MISSION-EVIDENCE-001 #6399](https://github.com/Luminous-Dynamics/symtropy/issues/6399)  
**Existing component contract:** [Synthetic Joint Qualification Contract](https://github.com/Luminous-Dynamics/symtropy/tree/main/benchmarks/synthetic-joints/qualification-contract/v0.1)  
**Status:** M0 interoperability proposal and illustrative machine-readable bundle. No physical benchmark has been run.

## Why this is not another dexterity leaderboard

The current field already has complementary benchmark layers:

| External framework | What it measures | How Symtropy should use it |
|---|---|---|
| HAND ERC Multi-Level Hand Benchmarks v1 alpha | Application, system, hand and component levels; component testbeds and metrics including strength, bandwidth, impedance, repeatability, thermal properties, fatigue and creep | Use as the layer/attribution crosswalk; do not duplicate the taxonomy |
| POMDAR | 18 physical/simulation dexterity tasks across vertical, horizontal, continuous rotation and pure grasping; throughput combines correctness (0.8) and speed against human baseline (0.2) | Preserve the benchmark profile and score; extend the run with repeated mission and thermal/lifecycle observations |
| NIST end-effector protocols (2020) | Grasp strength, grasp cycle time, finger strength and repeatability | Maintain as comparable hand/finger physical measures |
| HandsOn (2026) | Cross-embodiment simulation benchmark with 66 verified tasks, transfer split and policy evaluation | Use for policy generalization context, not actuator or hardware qualification |
| Bench2Dex (2026) | Bimanual simulation benchmark across 12 hands and 26 tasks with simulated visuotactile support | Use for bimanual policy/sensor context, not a direct physical actuator comparison |

The Symtropy joint-result schema already records candidate/configuration identity, task envelope, installed boundary, per-metric source and verification states, failure/censor denominators, lifecycle state and claim ceiling. This bundle references those component records by ID. It does not duplicate their detailed component measurements.

## The new layer: sustained mission throughput

Keep POMDAR's own score intact. Add a distinct, transparent system-level result:

\[
\text{Sustained mission throughput} =
\frac{\text{accepted task completions}}
{\text{total elapsed mission time}}
\]

The denominator includes every interval inside the preregistered mission boundary:
- task execution;
- thermal cooldown;
- derating and interrupted operation;
- fault recovery;
- service and recalibration;
- setup/standby when included in the stated mission boundary.

Always report this value alongside attempt denominators, task-family strata, latency distributions, task correctness, energy boundary, initial/final temperature channels, thermal events and component records. A system must not improve throughput by removing cooldown, failures or difficult tasks from the denominator.

This is not a replacement for POMDAR's correctness/speed score and is not an aggregate scalar for ranking every hand. Report the vector of outcomes, not a universal weighted score.

## Interoperability crosswalk

- **Application level:** deployment context, user acceptance, economics and return on investment remain separate application studies. The mission bundle links the specific application task profile and service boundary.
- **System level:** POMDAR task outcomes and sustained mission throughput; HandsOn/Bench2Dex simulation policy results remain in their own evidence class.
- **Hand level:** GRASP taxonomy, grasping volume, wrap/pinch/pullout strength, grasp cycle time, workspace, repeatability and impact/grasp resilience.
- **Component level:** actuator thermal properties, continuous/peak force and torque, transmission losses/coupling, bandwidth, impedance, tendon creep/fatigue, sensor calibration and latency.
- **Joint qualification contract:** linked_component_records points to existing source-bounded J1/J2/J3 records; it does not copy or promote their claims.

## Fair comparison protocol

1. Freeze the benchmark profile, source snapshot, task catalog, task-mix digest, object/fixture revision, acceptance criteria, ordering/randomization seed and planned attempts before testing.
2. Identify hardware, software, controller, policy/teleoperation, calibration and service state.
3. Declare whether arm, wrist, external vision, tactile sensing, human intervention and external compute are allowed.
4. Declare the installed-system and energy measurement boundary. Unknown components remain unknown.
5. Record every attempt as accepted, failed, aborted, censored or excluded. Exclusions require a reason and remain visible.
6. Record a complete timeline from mission start to mission end. Intervals must be ordered, non-overlapping and cover the entire mission clock with no unaccounted gaps. Cooldown and recovery are explicit intervals.
7. Synchronize task outcomes with bus/source energy, actuator state and separate temperature channels where instrumentation exists.
8. Link the component qualification records needed to attribute bottlenecks. Keep model predictions, physical measurements and independent reproductions separate.
9. Compare candidates on the same task mix and boundary. For stochastic controllers, preregister paired seeds and sample size and publish confidence intervals.

## Machine-readable contract

- hand_mission_bundle.schema.json — JSON Schema Draft 2020-12, closed-world mission record.
- examples/pomdar_repeated_mission_illustrative.json — illustrative subset with two accepted tasks, one failed repeat, a 40-second cooldown interval and a 120-second total mission clock.

The example calculates \(2/(120/3600)=60\) accepted tasks per elapsed hour. It is deliberately marked illustrative and contains no physical temperature trace. The schema validates structure; a semantic verifier must additionally check:
- each attempt references a task in task_catalog;
- attempt and timeline IDs are unique;
- all attempt intervals match task_execution events;
- timeline intervals cover exactly [0, mission_elapsed_s] without overlaps or gaps;
- mission_clock.elapsed_s equals run_summary.mission_elapsed_s;
- the five outcome counts sum to planned_attempts and agree with the attempt array;
- accepted_task_completions equals the count of accepted attempts;
- sustained_mission_throughput_per_h is recomputed from accepted completions and the full elapsed time;
- excluded/censored/aborted outcomes have reason strings.

A JSON Schema pass alone does not certify these cross-record arithmetic conditions.

## Benchmark targets

These are proposed research gates, not current results:

- Reproduce POMDAR's standard task score without changing its scoring method.
- Add repeated missions with preregistered workload and full cooldown/recovery accounting.
- Report per-task acceptance and latency plus sustained throughput and energy per accepted completion.
- Establish thermal channels and task-level data synchronization before attempting causal attribution.
- Demonstrate component-to-hand-to-system attribution by linking J1/J2/J3 records to hand- and task-level outcomes.
- Repeat at least one profile across an independent lab and publish raw traces, calibration, configuration digests and exclusions.

Do not set an arbitrary universal success percentage or claim to beat POMDAR from a different task mix. First reproduce its protocol and then test whether mission-aware evidence reveals capability differences hidden by short demonstrations.

## Current limitation

This is an evidence schema and protocol crosswalk, not hardware, policy or safety qualification. The example has synthetic outcomes only. No score, performance advantage or physical measurement is claimed.
