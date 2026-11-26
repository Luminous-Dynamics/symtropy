# RRVR Related Work, Differentiation, and Evaluation Plan v0.1

**Status:** research synthesis / design proposal  
**Snapshot date:** 2026-11-26  
**Claim ceiling:** literature comparison and proposed evaluation only. This document reports no new physical robot campaign, no SOTA superiority result, and no safety certification.

## Executive position

RRVR should not claim to have invented execution finality, current-actuation authorization, evidence provenance, late-evidence preservation, replay protection, independent physical-effect observation, or uncertainty-aware outcome reconstruction. These themes already appear in close 2026 individual IETF Internet-Drafts and in mature distributed-systems, cyber-physical-systems, and functional-safety practice.

The strongest defensible RRVR thesis is narrower and more empirical:

> RRVR is an executable, policy-independent evaluation and assurance overlay for determining whether a robot's recovery process preserved physical execution integrity under specified faults—and whether the evidence is sufficient to justify the next act.

That contribution must be demonstrated through matched experiments, endpoint/command-graph tests, independent adjudication, replayable raw records, and cluster-aware statistics. A design description or synthetic fixture is not that demonstration.

## 1. Closely related 2026 work

### 1.1 Actuation-bound execution finality

**Source:** [draft-das-actuation-bound-execution-finality-00](https://datatracker.ietf.org/doc/draft-das-actuation-bound-execution-finality/) — *Command Accepted Is Not Actuation Authorized: Execution Finality for Cyber-Physical and Industrial Control Systems*. Individual Informational Internet-Draft dated 2026-09-16; latest revision shown as 2026-09-15; Datatracker last-updated field 2026-09-29.

The draft describes an Actuation Finality Boundary (AFB) that checks the exact pending command, current execution authority, target actuator identity, freshness/replay state, required machine/process/safety predicates, and the effectuation path immediately before or atomically with physical output. It explicitly recognizes that existing safety PLCs, protected drives, robot controllers, safety I/O or device controllers may already provide the required boundary. It distinguishes prevention from upstream mitigation or post-effect audit.

**RRVR relationship:** substantial conceptual overlap with endpoint-enforced effect fencing, fresh authority, replay/duplicate prevention, and the distinction between a recorded command and an actual physical effect.

**Comparison question:** can RRVR's endpoint/driver contracts and benchmark demonstrate the claimed finality properties under crashes, delayed messages, restarts, loss of token history, endpoint substitution and alternate command paths? Can a third party reproduce those results against different robot stacks?

**Not established by this source alone:** an adopted IETF standard, formal IETF consensus, a public matched robot-recovery dataset, or independently reproduced physical campaign. The document is an individual work-in-progress Internet-Draft, not an IETF standard.

### 1.2 Execution-finality enforcement

**Source:** [draft-das-execution-finality-enforcement-00](https://datatracker.ietf.org/doc/html/draft-das-execution-finality-enforcement-00) — *Execution-Finality Enforcement, Protected State, and Delegated Operations*, published 2026-10-23 as an Informational Internet-Draft.

This work covers protected operation descriptors and state transitions, non-expanding delegation, replay fencing, fail-closed revalidation, multi-sink reconciliation, and implementation/deployment considerations.

**RRVR relationship:** overlaps with durable command identity, authority epochs, attempt reconciliation, endpoint enforcement, and unknown external outcomes.

**Potential empirical distinction:** RRVR can quantify what recovery systems actually do after injected faults—false effect verdicts, stale post-fence effects, duplicate one-shot effects, unsafe fresh-attempt admission, evidence yield, false holds, recovery utility and assurance overhead—using the same precommitted input state across treatment arms.

### 1.3 Shared evidence semantics and robotics profiles

**Source:** [draft-das-ai-safety-evidence-consistency-profiles-00](https://datatracker.ietf.org/doc/html/draft-das-ai-safety-evidence-consistency-profiles-00) — *AI Safety and Operational Consistency: Shared Evidence Semantics and Domain-Specific Profiles*, published 2026-10-30 as an Informational Internet-Draft.

The draft's shared evidence model addresses operation identity, source/resource/execution generations, clock uncertainty, provenance and dependency independence, three-valued consistency, actual-effect reconstruction, unknown state, and separate authorization. Its robotics profile explicitly describes:
- task and command lineage;
- physical-effect observations such as encoder, position, force, contact and load-transfer measurements;
- separation of command acceptance/controller completion from physical effect;
- preservation of uncertain consequences and late evidence;
- requalification of resumed work against current robot capability and safety-controller state.

Its hardware-observation profile also discusses queue/device generations, recycled queue positions, independent observation dependencies, coverage scope, and unresolved device-consumption outcomes.

**RRVR relationship:** strong overlap with the proposed evidence capsules and the intended physical-outcome claim model. This draft is direct related work and must be cited in RRVR publications and repository documentation.

**Potential empirical distinction:** convert a subset of those ideas into a reproducible, cross-policy test corpus and a reference evaluator; test exact endpoint graph closure and measured fault validity; publish raw matched episode evidence and independently adjudicated outcomes. Do not assume the draft's stated profiles are deployed or measured merely because they are specified.

### Status caveat

The three documents above are individual Internet-Drafts / work in progress. They are not adopted IETF standards, IETF consensus positions, product certifications, or proof of industry deployment. Their content is a reason to refine RRVR's novelty statement, not evidence that RRVR should copy their text or implementation.

## 2. Existing robotics benchmarks and what they leave to an overlay

| Work | Published scope / metrics | RRVR use | What RRVR must not claim |
|---|---|---|---|
| [SafeVLA-Bench](https://safevla.org/leaderboard.html) | 24 policies, five suites, 22,500 episodes and eight safety specifications on the referenced 2026-09 leaderboard snapshot; separates task success, safety, successful-but-unsafe (SBU) and violation severity | Preserve native success/safety/SBU/violation measures, then add command/effect/authority integrity during recovery | Do not compare RRVR percentages directly with its policy leaderboard unless using the same tasks, policies and protocol |
| [RoboRecover](https://arxiv.org/abs/2609.28952) | 2,000 recovery scenarios across RoboTwin and LIBERO; reconstructs off-nominal execution states by replaying action prefixes; uses train/test splits | Reuse the same post-deviation states to compare recovery with and without RRVR execution assurance | Do not replace its native recovery task or mislabel an RRVR overlay as a new recovery policy |
| [LIBERO-RECOVER](https://arxiv.org/abs/2609.05178) | 1,000+ scenarios across action retry, action adaptation, object-state recovery and environmental recovery | Stratify RRVR faults by the recovery level and preserve the host benchmark's outputs | Do not merge the four recovery levels into one uninterpretable scalar |
| [VLA-FixBench](https://bowen-yan-research.github.io/papers/vla-fault-recovery/) | 6,034 episodes across 40 simulated settings and two real-robot tasks; fault type, severity, timing and spatial deviations are annotated | Use its diagnosis/recovery cases to see whether fault reasoning is paired with actual command/effect lineage | Do not claim diagnosis correctness alone proves physical recovery or execution finality |
| [EAST](https://arxiv.org/abs/2610.34358) | Systematic fault injection with applicability/validity checks; reports targeted and valid fault executions | Adopt validity discipline: request, injector acceptance, actual execution and independent delivery observation are separate | An injector API acknowledgement alone is not evidence that the fault was delivered |
| [NIST Performance Assessment Framework for Robotic Systems](https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems) | Composable perception, mobility, dexterity and safety performance test methods for manufacturing-oriented robotics | Organize robot/task context and integration measures so execution-integrity evidence composes with subsystem performance | Do not treat a single assurance score as a substitute for the host system's capability model |

The official [RoboCasa leaderboard](https://robocasa.ai/leaderboard.html), updated November 25, 2026, currently lists 20 models on a 50-task evaluation across Atomic-Seen, Composite-Seen and Composite-Unseen. Its leading agentic entry scores 85.6 overall (91.1 / 85.0 / 80.0 across those splits). This is the host benchmark's average task-success measure, not a safety or execution-finality score. RRVR must preserve that native measure and never present its own assurance rates as directly comparable to the 85.6 figure.

[SafeVLA-Bench](https://safevla.org/leaderboard.html), last updated September 26, 2026, evaluates 24 policies over 22,500 episodes, five suites and eight safety specifications. The official board explicitly ranks by safety rather than success. On its RoboCasa-365 Atomic-Seen split, the top listed policy has 53.1% safety against 58.4% native success and 24.0% successful-but-unsafe episodes; on LIBERO, the top listed policy has 82.0% safety against 97.6% success and 17.0% successful-but-unsafe. These are not RRVR comparisons; they demonstrate why task success, safety conformance and execution-integrity claims must remain distinct.

**Replay validity matters:** RoboRecover's replay protocol requires end-effector and object position deviations below 2 mm and mean SSIM above 0.99 over the final five pre-recovery RGB frames. It retains intermediate-difficulty scenarios with 2–18 successes across 20 reference trials (10–90%); this deliberately excludes near-trivial and near-unsolvable cases. The paper's appendix reports 97.32% replay-validation pass rate after its audit, 183 trajectory clusters for 200 RoboTwin test scenarios, and 10,000 cluster-bootstrap draws for RoboTwin / scenario-bootstrap draws for LIBERO. RRVR should preserve these replay-validity and sampling rules, document the screening-induced target population, and report cluster-aware uncertainty. It must not generalize results on these curated recovery states to arbitrary field failures. See the [RoboRecover paper](https://arxiv.org/abs/2609.28952), particularly its replay-validation and statistical-protocol sections.

**Recovery diagnosis is not itself recovery:** [VLA-FixBench / FaultEval](https://bowen-yan-research.github.io/papers/vla-fault-recovery/) is an ICML 2026 benchmark with 6,034 execution episodes across 40 simulated settings and two real-robot tasks (tea preparation and charging-plug insertion), annotated for stage, fault type, severity, timing and spatial deviation. It evaluates static diagnosis, dynamic rollback/correction and real-robot intervention. The project page reports that the best automated VLM-assisted real-robot result (25%) remained below the VLA-only baseline (30%), while human intervention reached 65% as an upper bound. The paper's abstract reports human-intervention upper-bound gains of 13 percentage points on LIBERO and 35 on real-world robots. RRVR should distinguish diagnosis quality, intervention selection, actual recovery success and execution-integrity evidence; human upper bounds are not automated system results.

[LIBERO-RECOVER](https://arxiv.org/abs/2609.05178) contains 1,000+ failure scenarios across four recovery levels (action retry, action adaptation, object-state recovery and environmental recovery). RRVR's strata should preserve those four levels rather than collapse them into one aggregate.

## 3. RRVR's actual repository foundation

The following reflects repository issue/PR descriptions and currently visible artifacts. It is not a declaration that every exact-head workflow is green.

| Artifact | Concrete repository work | Evidence status / ceiling |
|---|---|---|
| [#7405 endpoint effect fencing](https://github.com/Luminous-Dynamics/symtropy/issues/7405) | Bounded endpoint formal model; one-shot reservation bound to payload and accepted epoch; stale-epoch and NVM-loss replay mutants | Earlier head reported 2,693 generated / 692 distinct states, depth 13 and expected mutant counterexamples. A corrected head was explicitly awaiting new exact-head runs; preserve that qualification boundary |
| [#7435 temporal closure](https://github.com/Luminous-Dynamics/symtropy/issues/7435) | Rust reference checker and TLA+ model; fenced-and-drained and bounded-expiry proof paths; per-holder/edge reasoning | Described as 15 Rust unit tests and six fixtures. Its issue says exact-head CI was still pending at that snapshot. Not physical transport-bound evidence |
| [#7471 physical command semantics](https://github.com/Luminous-Dynamics/symtropy/issues/7471) | Separate device-capability and command profiles; class-specific semantics; 16 known-answer cases and 26 authored tests | Structural reference evaluator only; does not inspect firmware or prove physical behavior. Exact-head qualification was not claimed in the issue |
| [#7478 matched benchmark v0.2](https://github.com/Luminous-Dynamics/symtropy/pull/7478) | Matched experimental arms, pairing keys, separate native/RRVR scores and synthetic protocol fixtures | Matched-protocol workflow passed after a jq scope correction in the 2026-11-26 pass. This qualifies protocol checks, not a robot campaign |
| [#7500 executable scorecard](https://github.com/Luminous-Dynamics/symtropy/pull/7500) | Rust CLI evaluates raw episode records; per-arm rates, fault stages, false verdicts, duplicate/stale effects, unsafe admissions, false holds, unresolved outcomes, latency and clusters | Synthetic four-record / two-pair fixture only. Recent CI attempts encountered rustfmt failures; test and Clippy steps were skipped. No scorecard Rust qualification or physical campaign should be claimed until a later exact-head pass |
| [#7515 evidence capsules](https://github.com/Luminous-Dynamics/symtropy/pull/7515) | Isolated Rust validator for evidence identity, provenance DAG, digests, clock-order constraints, queue-generation-bound endpoint receipts and claim assessments | 11 authored tests and synthetic fixture. Recent workflow attempts were still stopped at rustfmt; no test/build/Clippy pass is claimed yet |
| [#7559 paired contrasts](https://github.com/Luminous-Dynamics/symtropy/issues/7559) | Proposed campaign treatment declarations, physical configuration matching and paired-effect inference | Issue / proposed work, not implemented |
| [#7560 prior-art comparison](https://github.com/Luminous-Dynamics/symtropy/issues/7560) | Clause-level comparison with 2026 execution-finality and evidence-consistency work | Issue / proposed work, not completed |

### Current implementation distinction

RRVR's repository has more than a paper architecture: it contains separate endpoint, command-semantics, temporal-closure and benchmark reference implementations. The current programme's weak point is integration and experimental qualification: demonstrate that the separate contracts compose at a common command/effect identity and hold under the same preregistered fault campaign.

## 4. The most defensible differentiator

Avoid this claim:

> RRVR invents execution finality / exactly-once physical actuation / evidence-carrying recovery.

The literature above is enough to make that broad statement unsafe.

Test this narrower claim instead:

> RRVR provides an executable, matched, policy-independent evaluation method for measuring whether recovery preserves the integrity of physical execution—under explicitly delivered faults and independently adjudicated physical outcomes—and whether renewed actuation is justified by endpoint and command-graph evidence.

Potential differentiators that require proof:

1. **Matched causal comparison:** native recovery and assurance-enabled arms use identical frozen ex-ante inputs; realised physical outcomes remain arm-specific.
2. **Fault validity:** distinguish requested, injector-accepted, executed and independently observed faults, with invalid/unobservable trials retained.
3. **Effect-lineage integrity:** bind task, attempt, command, endpoint, physical effect and observation to stable identities.
4. **Temporal closure:** assess every command holder and forwarding edge, not just coordinator quietness or process restart.
5. **Evidence sufficiency:** type and qualify the evidence behind Applied / NoEffect / Partial / Unknown / Disputed instead of accepting caller assertions.
6. **Utility under assurance:** measure task recovery, safety, false holds, downtime, latency and evidence cost without allowing utility gains to offset hard safety failures.
7. **Reproducibility:** release raw records, frozen input digests, evaluator revision, environment/profile digests, fault schedule and independent adjudication artifacts.

## 5. Benchmark design: the comparison RRVR should publish

### 5.1 Matched arms

At minimum, use:
- native recovery;
- restart/retry;
- journal-only;
- journal + injector validity;
- endpoint fencing;
- graph temporal closure;
- full RRVR.

Every arm must execute the same precommitted task, policy, scene, initial state, action prefix, seed and fault schedule. Physical hardware, firmware, controller, driver, endpoint, queue, observer and calibration profiles should also be identical within pairs unless the design explicitly varies them and preregisters the stratification/factorial analysis.

The treatment declaration must list the implementation digests and exactly which RRVR components are enabled. A moving `full_rrvr` label is not an adequate treatment definition.

### 5.2 Compound fault campaign

Prioritize crash-after-effect-before-acknowledgement, then vary:
- coordinator crash before/after durable reservation;
- lost injector response and independently undelivered fault;
- endpoint restart, NVM loss and queue-generation change;
- delayed, duplicated, reordered and replayed old-epoch commands;
- cancellation racing with in-flight dispatch;
- firmware / controller / driver reconfiguration;
- partition and asymmetric communication loss;
- clock drift, uncertainty overlap and incomparable time domains;
- hidden or newly discovered command holders and forwarding paths;
- partial physical effect, sensor outage and contradictory observers.

Fault schedules and expected dispositions must be precommitted. Invalid injection, unobservable delivery, incomplete telemetry and unresolved physical outcomes must remain visible in the campaign ledger.

### 5.3 Hard-gate metrics

Report separately:
- false NoEffect and false Applied;
- false closure;
- duplicate one-shot effects;
- stale old-epoch effect after accepted fence;
- unsafe fresh-attempt admission;
- fault request/acceptance/execution/observation validity;
- unresolved / disputed outcome rate;
- positive-control false-hold rate.

The frozen negative-control target is zero false safe/closure claims. This is a research acceptance target, not a measured result or a statistical guarantee.

### 5.4 Utility and cost metrics

Also report:
- native task success, safety, SBU and recovery success;
- recovery time, downtime and mission makespan;
- p50/p95/p99/p99.9 for each named critical-path phase, with sample count;
- evidence acquisition and adjudication cost;
- throughput impact and resource overhead;
- positive-control admission and false-hold rates.

Do not produce a single weighted scalar that permits a task-success improvement to compensate for a hard false-safe event.

### 5.5 Statistical analysis

Pre-register the estimands before running:
- paired risk difference for each binary assurance outcome;
- discordant pair counts and a suitable paired test (e.g. exact McNemar where assumptions apply);
- paired differences for recovery latency and downtime;
- cluster-aware bootstrap / hierarchical inference over the actual independent sampling units;
- multiplicity handling for multiple endpoints;
- sample-size/power plan based on cluster counts, not just raw episode count.

Keep failed, invalid, unresolved and unobservable scheduled trials in an explicit accounting path. Do not assume episode independence merely because records have different IDs.

## 6. Industry and standards interface

- [ISO 10218-1:2025](https://www.iso.org/standard/73933.html) addresses industrial robot safety requirements; [ISO 10218-2:2025](https://www.iso.org/standard/73934.html) addresses industrial robot applications and cells, including integration and lifecycle activities.
- [ISO 13849-1:2023](https://www.iso.org/standard/73481.html) concerns the design and integration of safety-related parts of control systems. RRVR evidence is not a PL claim and cannot replace required risk assessment or validation.
- [ISO 21423:2026](https://www.iso.org/standard/86749.html) specifies communications and interoperability among industrial mobile robots, fleet managers and related industrial systems. The standard explicitly states that it is not safety-related and contains no safety requirements.
- [NIST's robotic performance assessment work](https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems) supports composable robot performance measurement across perception, mobility, dexterity and safety.

RRVR should map its evidence to an integrator's risk-assessment and safety-case process. It should not claim ISO conformity, PL, SIL, certification or safe deployment from a passing software benchmark.

## 7. Execution sequence

1. **Stabilize reference artifacts:** exact-head fmt, locked build, tests, strict Clippy and formal-model checks for #7405, #7435, #7471, #7500 and #7515. Record each result against its exact SHA.
2. **Define common schema composition:** one operation/attempt/command/effect identity model linking command semantics, endpoint fence receipts, graph closure, evidence capsules and benchmark episode records.
3. **Build deterministic process-fault harness:** simulate crash, lost response, duplicate, delayed message, restart, storage loss and queue generation changes without hardware.
4. **Replay existing recovery tasks:** use RoboRecover and LIBERO-RECOVER post-deviation states; do not build a disconnected toy benchmark as the primary comparison.
5. **Add Gazebo / ROS integration:** verify that the actual driver and endpoint command holders are within the graph inventory. Separate simulator evidence from physical evidence.
6. **HIL, then physical robot:** independently instrument command/effect boundaries; begin with bounded reversible tasks and a validated stop envelope.
7. **Independent reproduction:** publish fixed campaign manifests and raw evidence; have a second implementation run the scorecard/evidence validator.

## 8. Claim discipline

Use these status labels in README, PRs and reports:
- **Implemented:** source artifact exists.
- **Structurally tested:** synthetic known-answer / mutation tests exercised.
- **Formal bounded result:** specific model, bounds, state counts and counterexamples are reported.
- **CI qualified:** exact head passed the declared workflow.
- **Simulator campaign:** actual simulator episodes executed and preserved.
- **HIL:** hardware-in-the-loop evidence with declared instrumentation.
- **Physical campaign:** physical robot, raw data, independent observation and safety envelope are identified.
- **Independently reproduced:** an external party reran the declared procedure.

Never promote one status into the next without new evidence.

## Sources and review notes

The primary literature is linked inline above. Relevant primary sources:
- [SafeVLA-Bench official leaderboard](https://safevla.org/leaderboard.html)
- [RoboRecover (arXiv:2609.28952)](https://arxiv.org/abs/2609.28952)
- [LIBERO-RECOVER (arXiv:2609.05178)](https://arxiv.org/abs/2609.05178)
- [VLA-FixBench (ICML 2026)](https://bowen-yan-research.github.io/papers/vla-fault-recovery/)
- [EAST (arXiv:2610.34358)](https://arxiv.org/abs/2610.34358)
- [NIST Performance Assessment Framework for Robotic Systems](https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems)
- [ISO 21423:2026](https://www.iso.org/standard/86749.html)
- [ISO 10218-1:2025](https://www.iso.org/standard/73933.html)
- [ISO 10218-2:2025](https://www.iso.org/standard/73934.html)
