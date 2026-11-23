# RRVR paired fault campaign manifest v0.1

**Issue:** [#7025 — frozen paired fault campaigns and reproducible evidence packages](https://github.com/Luminous-Dynamics/symtropy/issues/7025)  
**Validator:** src/fault_campaign.rs  
**CLI:** rrvr_campaign_validate  
**Schema:** schemas/fault_campaign_v0_1.schema.json  
**Fixture:** fixtures/fault_campaign_paired_v0_1.json

## Why this work exists

A recovery benchmark must control more than the policy and scene. A result is not comparable when the controller build, fault schedule, reset state, observer configuration, or initial action prefix changes between arms. Conversely, a benchmark that reports only final task success can hide a duplicate physical effect, a stale command, or an unjustified retry.

The v0.1 manifest records the experimental conditions needed to replay a paired campaign. It is an experiment-plan contract, not a fault injector.

## Paired evaluation design

Each pair shares:
- policy ID and exact checkpoint digest;
- task, scene digest and action-prefix digest;
- platform profile, simulator/environment artifact and reset profile;
- random seed and stable logical-effect ID;
- exact ordered fault schedule, injection parameters and fault-domain identities.

Only the assurance arm changes:

| Arm | Evidence retained | Purpose |
|---|---|---|
| last_attempt_only | most recent attempt only | Quantify loss of prior physical-effect history |
| flat_history | attempt list without strict parent/scope composition | Isolate the value of typed ancestry and scope binding |
| owner_receipts | ordinary owner receipts | Isolate the additional value of observer and graph-fence composition |
| full_rrvr | complete attempt-lineage, observer and fence contract | Candidate assurance gate |

The fixture contains a positive pair (full RRVR must admit; the comparison arm is measured) and a negative pair (full RRVR must block; the comparison arm is measured). The negative schedule includes process crash, observer common-cause failure, controller queue retention and firmware power loss across four declared cut points and four layers.

The manifest validator requires each pair to have identical task/policy/scene/action-prefix/platform/reset/seed/logical-effect/fault-schedule fields, plus one full_rrvr arm and at least one comparator. The policy and physical experiment are therefore held constant while the assurance design changes.

## Required fault campaign fields

The strict manifest contains:
- campaign version, ID, fault-model revision and digest;
- platform ID, profile digest, environment artifact digest, simulator, ROS, middleware, controller, driver, firmware, calibration and observer-stack revisions;
- reset mechanism/revision, reset-profile digest, expected initial-state digest and post-reset observation ID;
- explicitly declared required fault layers and lifecycle cut points;
- per-scenario pair ID, arm, positive/negative/mutation class, task/policy/scene/action-prefix identities, seed and logical-effect ID;
- ordered per-scenario injections: unique fault ID, cut point, target layer/component, fault-domain ID, fault kind and parameter digest;
- expected disposition, reason code, invariant ID and failure-signature digest.

Unknown JSON fields are rejected. The validator rejects digest errors, missing pins, duplicate scenario/fault IDs, unordered fault sequences, missing layer/cut coverage, unpaired arms, incomparable pairs, and positive/negative controls with contradictory full_rrvr expectations.

## What a PASS means

The CLI emits:
- exact manifest SHA-256;
- scenario/pair/control counts;
- fault injection count;
- declared versus covered layers and cut points;
- full_rrvr positive and negative control counts;
- explicit claim-ceiling fields.

PASS means only:
1. the input is syntactically valid under the strict schema and Rust types;
2. the platform, environment, reset, fault model, policy and scene identities are pinned;
3. every pair is internally comparable;
4. the required declared layers and cut points are covered by the manifest;
5. positive and negative full_rrvr expectations are internally consistent.

PASS does **not** mean that the fault injector ran, that a fault was delivered, that the physical platform inventory is complete, that owner evidence is authentic, that the robot behaved safely, or that the TLA+ refinement is established. The report deliberately sets all of those claims to false/not checked.

## Metrics: do not collapse dimensions

Publish these measures separately:

| Dimension | Measures |
|---|---|
| Assurance correctness | false-safe admissions, false holds, duplicate effects, stale command after accepted fence, replay/scope substitution |
| Observer resilience | false-quorum rate by fault model, common-cause cut-set exposure, observer evidence freshness |
| Recovery utility | policy RSR, original task success, safe-success, successful-but-unsafe, additional actions, wall time, operator interventions |
| Evidence debt | unresolved-effect count, oldest unresolved age, unresolved-effect seconds per 100 actions, p50/p95/p99 evidence-clearance time |
| Reproducibility | replay-complete rate, minimized-failure signature preservation, deterministic replay, trace reduction ratio, oracle calls |
| Coverage | declared cut coverage, per-layer coverage, required owner-domain presence, mutation-control kill rate |
| Performance | p50/p95/p99 validation and recovery latency, throughput, instrumentation overhead, CPU/memory/storage per attempt/event |

Evidence debt is the time a potentially consequential operation remains unresolved. Report it as an operational availability/safety trade-off, not as a score to be minimized by accepting a retry. A correct hold may increase unresolved time while preventing a duplicate effect; therefore always report false-safe and false-hold rates beside it.

For the first campaign, target:
- zero false-safe admissions and zero duplicate effects in the frozen negative-control corpus;
- 100% admission of frozen, valid full_rrvr positive controls;
- 100% rejection/blocking of frozen unsafe full_rrvr controls;
- 100% required-layer and required-cut coverage for the declared platform profile;
- 100% deterministic report output for identical input bytes;
- paired comparisons with identical seeds, policy/scene/action-prefix digests and fault schedules.

These are research targets, not measured achievements. Never aggregate safety, recovery utility and performance into a single weighted leaderboard score.

## Current research and industry positioning (23 November 2026)

The relevant public benchmarks answer complementary questions:

- [RoboRecover (24 September 2026)](https://arxiv.org/abs/2609.28952) evaluates recovery policies from execution-induced intermediate states across 2,000 RoboTwin/LIBERO scenarios. RRVR should use the same reconstructed recovery points where feasible, then add the evidence/authority layer. It should not claim to beat RoboRecover's RSR with a manifest validator.
- [SafeVLA-Bench](https://safevla.org/leaderboard.html) ranks by safety separately from task success, and reports successful-but-unsafe episodes and violation severity. Its leaderboard (updated 26 September 2026) covers 24 policies, five suites and 22,500 episodes. On RoboCasa-365 Atomic-Seen, Xiaomi-Robotics-1 reports 81.9% SR but 50.1% Safety and 37.7% successful-but-unsafe. On LIBERO, VLA-Adapter reports 97.6% SR and 82.0% Safety. These are not RRVR metrics; they show why completion cannot substitute for a safety verdict.
- [RoboCasa365 leaderboard](https://robocasa.ai/leaderboard) was updated 22 November 2026 and reports 50 tasks, 25 models and three splits. The leading listed Overall result is 85.6 for HexaShell. This is a task-performance leaderboard, not a recovery-assurance comparison; RRVR should report it only as a policy substrate with matched tasks and splits.
- [Yang et al., A Survey of Fuzzing for ROS-Based Robotic Systems](https://doi.org/10.3390/app16147250), published 20 July 2026, reviews 92 selected works. It identifies repeatable execution/reset and diagnosable failure analysis as comparatively underdeveloped areas, and asks studies to report layer, reset/replay policy, instrumentation, artifact and diagnosis output together. This motivates frozen manifests and replayable fault bundles.
- [Zheng et al., Long-horizon embodied manipulation survey](https://doi.org/10.1007/s44336-026-00048-x), published 16 November 2026, argues that binary task success hides partial completion, error propagation, recovery cost and horizon-dependent degradation. The attempt ledger and evidence-debt measures address this gap.
- [ROS 2 REP 2014](https://reps.openrobotics.org/rep-2014/) recommends reproducible grey-box performance measurement of both nodes and complete computational graphs using LTTng/ros2_tracing. RRVR adopts this measurement discipline and adds semantic owner/effect/attempt/epoch boundaries.
- [NIST Performance Assessment Framework for Robotic Systems](https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems) composes perception, mobility, dexterity and safety component measures into task-level system performance. RRVR should similarly compose evidence but must keep assurance correctness orthogonal to the task-performance result.

Prior art must remain visible: the [October 2026 IETF individual Internet-Drafts on receipt-gated effectuation and finality](https://datatracker.ietf.org/doc/draft-das-safety-first-execution-finality/) discuss staged physical actions, single-use authority, receipts, anti-replay, independent observation and recovery from unknown/partial/conflicting outcomes. RRVR should claim neither invention of those ideas nor standards status. Its hypothesis is the tested integration of owner-specific robot persistence, mission, controller, downstream graph, observer and existing native authority components into a replayable cross-owner campaign.

## Next integration path

1. Validate the manifest and synthetic paired controls in CI.
2. Add a deterministic adapter that compiles each manifest scenario into the existing trace/fault schedule format without changing its identity or order.
3. Replay the same campaign through last-attempt-only, flat-history, owner-receipt-only and full-RRVR arms.
4. Attach exact runtime/TLA witness traces and causal-minimization bundles to every invariant failure.
5. Run a pinned simulator campaign, then ROS 2/controller/driver fault injection, then HIL.
6. Attempt physical trials only under a separately reviewed safety case, calibrated independent observation, declared abort conditions and applicable protective measures.

A simulator result remains a simulator result; it is not HIL or physical evidence.


## Deterministic plan compilation

The validated manifest is compiled into one immutable plan by [CAMPAIGN_PLAN_V0.1.md](CAMPAIGN_PLAN_V0.1.md). The plan binds the exact source manifest bytes, full platform/reset/fault-model profile, pair-level experimental equivalence and per-scenario ordered fault schedule. The compiler emits pair commitments shared by all arms in a pair, scenario schedule digests, and a top-level plan hash.

The companion verifier checks the compiled plan against the exact source manifest and an externally pinned expected plan digest. An embedded self-hash is not a signature; the consumer must receive the expected digest through a trusted release record or authenticated channel.

The compiled plan is intended to become the stable input to simulator/ROS/controller/driver/HIL adapters. These adapters must preserve fault order, cut point, target identity, fault-domain identity and parameter digest. They return separate execution receipts and may not rewrite the plan. A compiled plan is not proof that a fault was injected or delivered.
