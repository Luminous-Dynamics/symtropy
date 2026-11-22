# RRVR trace replay and minimization: research alignment v0.1

**Implementation:** `research/mobility-rrvr-trace/`  
**Tracking issue:** [MOBILITY-RRVR-TRACE-001 #6902](https://github.com/Luminous-Dynamics/symtropy/issues/6902)

## 1. Current research landscape

### ROS-based fuzzing

Yang et al. reviewed 92 selected works on ROS/ROS-based and closely related robotic/CPS testing (20 July 2026). They report that studies exercise different ROS layers and define failures differently, making results hard to compare. Their review identifies repeatable execution and diagnosable failure analysis as weaker areas than input modeling, feedback and oracle construction. They recommend explicit execution/reset assumptions, replay and nondeterminism controls, event slicing, dependency-aware minimization and root-cause ranking.

RRVR's trace bundle and causal minimizer directly target this gap: fixed system/model/fault/oracle revisions, initial-state commitment, event identities, happens-before edges, exact witness identity, oracle-call budget, replay status and a minimized schedule that preserves the exact same property violation.

Reference: Mucheng Yang et al., [A Survey of Fuzzing for ROS-Based Robotic Systems](https://doi.org/10.3390/app16147250), *Applied Sciences* 16(14), 7250 (20 July 2026).

### Long-horizon embodied manipulation

Zheng et al.'s survey, published 16 November 2026, argues that binary success conceals partial completion, error propagation, recovery cost and horizon-dependent degradation. It recommends verified intermediate completion, failure localization, recovery cost, horizon sensitivity and generalization reporting.

RRVR should be treated as a complementary assurance dimension, not as a replacement for task benchmarks. A robot can finish a task while recovering incorrectly, and a recovery system can preserve safety while taking longer or requiring an operator. Both must be visible.

Reference: Yi Zheng et al., [Long-horizon embodied manipulation: a survey of methods, trends, and challenges](https://doi.org/10.1007/s44336-026-00048-x), published 16 November 2026.

### Safety-aware VLA evaluation

SafeVLA-Bench separates native task success from safety, successful-but-unsafe rollouts and worst-violation severity. Its public leaderboard lists 24 policies, five evaluation suites, 22,500 episodes and eight safety specifications (updated 26 September 2026). The reported results show that high task success can coexist with substantial unsafe execution.

RRVR should adopt the same non-compensatory discipline: task completion, physical-effect continuity, recovery correctness, observer evidence and native authority validity are separate dimensions. No weighted aggregate may allow a strong task-success score to compensate for an unsafe recovery.

Reference: [SafeVLA-Bench](https://safevla.org/), with published leaderboard and method.

## 2. Relevant standards and scope boundaries

| Reference | Official scope/status | How RRVR should use it |
|---|---|---|
| ISO 10218-1:2025 | Published industrial robot safety requirements for the robot as a partly completed machine | Map the applicable system hazards and lifecycle controls; do not claim compliance from RRVR software tests |
| ISO 10218-2:2025 | Published safety requirements for industrial robot applications and robot cells | Treat cell integration and application-level risks as distinct from the robot component |
| ISO 21423:2026 | Published in October 2026; communication/interoperability among industrial mobile robots, fleet managers and other industrial systems; explicitly not safety-related and excludes navigation requirements | Use as an interoperability context for cross-owner/fleet/facility event identity and versioned evidence exchange, not as a safety standard |
| ISO 18646-5:2026 | Published in July 2026; performance criteria and test methods for legged-robot locomotion; explicitly not intended for verification/validation of safety requirements | Align locomotion performance reporting where relevant, but maintain separate safety/recovery qualification |

Official sources: [ISO 10218-1:2025](https://www.iso.org/standard/73933.html), [ISO 10218-2:2025](https://www.iso.org/standard/73934.html), [ISO 21423:2026](https://www.iso.org/standard/86749.html), [ISO 18646-5:2026](https://www.iso.org/standard/86850.html).

The trace minimizer is not a conformity assessment and must not be presented as an alternative to risk assessment, safety-rated control, or standards-based validation.

## 3. Benchmark baselines

Use the identical frozen fault traces, target properties, replay oracle and compute budget across all arms.

| Arm | Reduction rule | Main expected limitation |
|---|---|---|
| B0: no minimization | Publish full failing trace | Diagnosis cost and irrelevant event volume |
| B1: sequence-only delta debugging | Remove event ranges without dependency closure | Can create invalid traces or remove necessary causal context |
| B2: causal dependency minimization | Preserve predecessor closure | Can preserve a different failure unless the oracle checks exact identity |
| B3: RRVR exact-signature minimization | Dependency closure plus exact property/class/witness preservation and fixed oracle revision | More replay calls; still depends on deterministic, trustworthy oracle |

### Mandatory metrics

- **Exact replay rate:** fraction of minimized bundles that reproduce the exact original failure signature.
- **False preservation rate:** fraction accepted even though property, violation class or witness differs. Target: 0.
- **Invalid-trace rate:** candidates rejected due missing/forward dependencies or missing witness.
- **Reduction ratio:** minimized/original event count and raw byte count, reported separately.
- **Causal closure:** fraction of retained events whose declared predecessors are present.
- **1-minimality certification rate:** only count reports whose single-event causal removals were all checked within budget.
- **Oracle efficiency:** oracle calls per removed event and per minimized failure.
- **Replay determinism:** repeated exact bundle/oracle/seed outcome agreement.
- **Diagnosis utility:** time for an independent engineer to identify the causal failure and responsible subsystem.
- **Cross-environment replay:** replay success across declared simulator, middleware, controller or hardware versions; never pool unlike environments.

Do not optimize only for smallest trace. A tiny trace that no longer explains the physical or distributed-system context is not a useful counterexample.

## 4. Proposed targets

Software-corpus targets:
- 100% exact failure-signature preservation on the frozen known-answer corpus.
- 0 false preservation of a different failure, pass or invalid trace.
- 100% deterministic output for fixed trace, oracle revision, seed and budget.
- 100% ancestor/descendant closure on frozen graph cases.
- 100% accurate reporting of incomplete minimization when the budget is exhausted.
- 100% agreement with an independently authored minimization oracle on the frozen corpus.

Performance targets should be reported as distributions, not invented universal thresholds:
- event and byte reduction by fault family;
- oracle calls and wall-time by initial trace size;
- replay success rate across repeated runs;
- diagnosis time and root-cause localization accuracy;
- sensitivity to causal graph width, depth, concurrency and shared events.

## 5. Planned RRVR benchmark integration

The same minimized trace format should eventually represent:
1. TLA+ counterexample transitions;
2. Rust reference-contract decisions;
3. ROS 2 middleware and lifecycle events;
4. simulator and fault-injection events;
5. HIL controller/driver/firmware receipts;
6. independently observed physical-effect events.

A format adapter must preserve source timestamps, logical sequence, clock identity/uncertainty, raw artifact digests and source revision. It must not silently convert incomparable clocks into a total order. Use explicit happens-before edges where wall-clock ordering is not justified.

## 6. Claim ceiling

The reference crate only checks and minimizes supplied typed traces against a supplied replay oracle. It does not parse TLC output or ROS bags, authenticate raw artifacts, prove the event inventory complete, establish physical causality, qualify a simulator, or establish a safety case. Cross-environment replay, physical capture and root-cause correctness require separate campaigns and independent evidence.