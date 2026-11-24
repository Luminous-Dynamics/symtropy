# RRVR Benchmark Protocol v0.2 — Cross-Restart Physical-Effect Assurance

**Status:** proposed benchmark protocol; targets below are qualification goals, not measured results.

## 1. Research thesis

Robotics fault-tolerance testing usually asks whether a robot or policy completed a task, whether a selected fault was injected, and whether the resulting trajectory violated a safety rule. Those questions are necessary but do not answer a critical distributed-systems question:

**After a coordinator, simulator, middleware node, injector or robot controller restarts during a physical operation, can the system determine what was authorized, what may have happened, what actually happened, who observed it, and whether another attempt is safe—without accidentally repeating the physical effect?**

RRVR should be evaluated as an evidence-qualified recovery protocol for physical effects, not as a generic fault injector, VLA policy, simulator or digital twin.

The central unit of analysis is a *logical effect* with multiple possible attempts:

`LogicalEffect -> Attempt -> Intent -> Dispatch -> PhysicalOutcome -> IndependentObservation -> Reconciliation -> RecoveryAdmission`

Every transition has a distinct identity and evidence scope. An unknown result is not silently converted into success, failure or permission to retry.

## 2. Paradigm shifts

### 2.1 From task execution to physical-effect lifecycle

Represent the robot's effect as a cross-process, cross-restart state machine. A mission/task result is only one downstream measure. The protocol must distinguish:
- request not sent;
- request durably intended but not yet known to be sent;
- accepted by the injector;
- applied / no effect / partial / unknown;
- observed by independent evidence owners;
- reconciled against an authoritative ledger;
- eligible or ineligible for a new attempt.

This prevents a restarted orchestrator from mistaking an RPC response for a physical fact.

### 2.2 From retries to reconciliation

Traditional idempotency keys and payload-mismatch rejection are essential, established techniques. RRVR's proposed extension is their use across the reset/world epoch, simulation clock, physical-effect identity, recovery attempt and independent evidence lineage.

The safety rule is asymmetric:
- same key + same payload: retrieve the prior result; do not repeat the effect;
- same key + different payload: conflict; stop;
- unknown or rejected result: read-only reconciliation against the effect owner;
- no-dispatch proof: terminal for the old identity, not permission to replay it;
- a new attempt: a new identity, fresh authority and newly qualified evidence.

The injector itself must implement a durable atomic mapping from key to payload to outcome. A coordinator-side journal cannot manufacture remote idempotency.

### 2.3 From logging to proof-carrying operations

An operation is not qualified merely because logs exist. Its evidence bundle should bind:
- the committed plan and fault-campaign revision;
- the exact mutator and target identity;
- run, boot, controller, reset and attempt epochs;
- payload and parameter digests;
- source clock and uncertainty;
- independent raw observation artifacts;
- observer revisions and common-cause declarations;
- verifier and model revisions;
- the final three-valued/qualified verdict.

The journal proves only ordering and integrity relative to its supplied bytes. It does not authenticate its writer, establish owner truth, or prove that a physical effect occurred. Signatures, protected keys, hardware-backed identities and external artifact custody remain separate requirements.

### 2.4 From “two observers” to fault-model independence

Observer count is not independence. The benchmark must vary shared clocks, power, network paths, hosts, firmware, calibration, models, data lineage and operator/control authorities. Deliberately correlated observers should be a mandatory negative control.

### 2.5 From pass/fail testing to evidence-quality measurement

Score whether an execution produced *valid, attributable, independently verifiable evidence*, not just whether the injected scenario ran or the task completed. Include challenge exposure, fault-realization validity, observer coverage, unresolved residuals and false assurance.

### 2.6 From component qualification to compositional robot assurance

Compose evidence across perception, planning, controller queues, downstream execution, actuator/injector, physical sensors and coordinator recovery. A component-level pass cannot compensate for an unqualified system boundary. This aligns with NIST's long-standing system-performance framing across perception, mobility, dexterity and safety.

## 3. External comparison: scope and non-claims

The following are complementary baselines, not interchangeable competitors.

| Baseline | What it evaluates well | RRVR relationship | Fair comparison |
|---|---|---|---|
| EAST (25 Oct 2026 preprint) | Literature-grounded hazard/fault taxonomy, digital-twin injection, validity checks, simulator-state safety assessment, replayable traces | Closest current prior art for systematic embodied-AI fault testing. RRVR must not claim novelty for generic digital-twin injection or replayable campaigns. | Reuse or map EAST's admissible fault operators and matched task-policy budgets. Add restart cuts, owner ledger ambiguity, cross-epoch physical-effect lineage and recovery admission. Compare valid-evidence yield and safety residuals under the same scenarios. |
| SafeVLA-Bench | Task-aware temporal safety specifications and the success/safety gap in VLA rollouts | Downstream policy safety benchmark; not a reset/dispatch protocol benchmark. | Run RRVR campaign/recovery around the same policies and trajectories. Report SR, Safety, SBU and VSI alongside RRVR recovery metrics; do not compare percentages as if they had the same denominator. |
| ROS `ros2_fault_injection` | Configurable faults on topics, transforms and services; scenarios, assertions, seeded random faults | Practical ROS fault-injection baseline. | Use its injector types and seeds as compatible fault sources. Measure whether RRVR preserves intent/effect identity, result reconciliation and safe restart across the same injections. |
| ROS-RVFT guidelines | Runtime verification and field-testing process: lifecycle/query APIs, logs, fault injection, isolation, record/replay, headless simulation and postmortem analysis | Process guidance that RRVR can operationalize with strict state/evidence contracts. | Crosswalk each guideline to a test, artifact and gate. Avoid claiming to replace ROS-RVFT. |
| Gazebo ROS 2 Simulation Interfaces | Standard reset, step, simulation-state and world interaction services/actions | Concrete simulator control surface, not a recovery protocol. | Pin exact simulator/API revision. Test every supported mutator, reset/step races and world-generation observation. A single coordinator must own all world mutators or mark the world uncontrolled. |
| AWS idempotent API practice | Caller-provided request IDs, same-token semantic equivalence and parameter mismatch rejection | Established distributed-systems foundation for RRVR's dispatch contract. | Treat as a baseline contract. RRVR's added evaluation dimension is cross-restart physical evidence and world/attempt scope, not invention of idempotency. |
| SQLite WAL / transactional stores | Mature crash recovery, transaction ordering and concurrency trade-offs | Essential storage/performance baseline for the custom append-only reference journal. | Compare the exact workload and durability settings. RRVR's custom log must justify itself with simpler typed domain transitions or better evidence semantics—not assumed durability/performance superiority. |
| NIST robot performance assessment | Compositional system capability assessment for perception, mobility, dexterity and safety | Supports system-level rather than isolated policy/component claims. | Report capability and recovery assurance at both component and integrated task-cell level. |

### Current quantitative reference points

SafeVLA-Bench's live leaderboard page reports 24 policies, 5 evaluation suites, 22,500 episodes and 8 safety specifications; the page says it was last updated on 26 September 2026. Its LIBERO leader, VLA-Adapter, shows 82.0% safety alongside 97.6% native task success and 17.0% successful-but-unsafe episodes. On RoboCasa-365 Atomic-Seen, the board's RLDX-1-FT-RC365 entry shows 53.1% safety, 58.4% task success and 24.0% SBU. These results show why task success alone is not an adequate safety metric. They are policy-rollout statistics, not RRVR protocol performance.

EAST's 25 October 2026 preprint reports 74 admissible task-policy-operator combinations targeted, 63 valid executions, and 82 valid executions that completed their task while violating safety requirements. This makes its validity-aware fault-generation and independent simulator-state assessment a required comparison point. RRVR's hypothesis is narrower: recoverable, owner-scoped physical-effect semantics across process restarts and ambiguous mutation outcomes.

ROS fault-injection and ROS-RVFT already cover practical injection, assertion, seeded replay, lifecycle instrumentation and verification guidance. RRVR should integrate with those capabilities rather than rebuild them.

## 4. Benchmark suite

Use the following suite as a versioned set of campaigns, with one immutable manifest and artifact schema per version.

| Suite | Main question | Core crash/fault cuts | Required negative controls |
|---|---|---|---|
| RRVR-JRN — Journal durability | Does the coordinator recover exactly the persisted intent/outcome history after a crash? | before write; partial write; after write before sync; after sync; after external mutation before response; response before outcome append; torn tail; middle-record mutation; disk full; sync error; process kill; power cut on target media | silent tail truncation, hash-chain acceptance after mutation, stale lock theft, second writer admission |
| RRVR-RST — Reset lifecycle | Can reset be issued once and qualified independently? | before request; server applies then response lost; response accepted then state read lost; concurrent reset; late response; restart during reconciliation | repeat reset on unknown, same epoch accepted, wrong clock, wrong initial-state digest, unpaused/nonzero-step state |
| RRVR-IDN — Effect identity | Does one logical effect have unambiguous attempts and request bindings? | duplicate key, payload change, key collision, delayed/late request, new world epoch, controller/boot rollover | key reuse after no-dispatch, different payload accepted, prior epoch/clock accepted |
| RRVR-EFX — Physical effect semantics | Can applied/no-effect/partial/unknown be distinguished? | injector acceptance, actuator boundary, physical effect, observation gap, downstream queue and execution | infer no-effect from missing log; equate accepted RPC with applied effect; partial mapped to no-effect |
| RRVR-FNC — Fencing and authority | Can stale commands, queues, permits and downstream executors be fenced? | boot/controller restart, delayed commands, queued command replay, stale ticket, permit reuse, graph partition | stale owner accepted; permit double-use; queue-only fence while downstream execution remains active |
| RRVR-OBS — Observer assurance | Is the evidence independent and correctly scoped? | sensor delay, clock skew, observer reboot, calibration drift, host/power/network common cause, stale cache | same raw artifact relabelled as two observers; shared common-cause domains; observer sees expected signature; wrong epoch/step |
| RRVR-LIN — Attempt lineage | Can every retry be traced to a distinct attempt and prior logical effect? | multiple retries, coordinator restart, handover, campaign replacement, partial prior effect | orphan attempt, duplicated attempt ID, dropped prior outcome, new attempt without admission |
| RRVR-TRN — Transport portability | Does the protocol survive different ROS/Gazebo/injector implementations? | ROS service/action timeout, QoS loss, bridge restart, DDS discovery delay, simulator reset race | adapter-specific bypass, hidden uncoordinated mutator, clock assumptions not declared |
| RRVR-SYS — End-to-end composition | Do all boundaries preserve the invariants together? | cross-product of representative crash, effect, observer, fencing and authority cuts | any dangerous transition mutant that still passes |

EAST and SafeVLA scenario families can supply task/fault content to RRVR-TRN and RRVR-SYS. RRVR should publish adapters and crosswalks instead of requiring a new simulator benchmark ecosystem.

## 5. Metrics and qualification targets

All targets below are **proposed gates**. They must be adjusted only through documented hazard analysis and matched-budget evidence, never to make a failing system pass.

### Hard safety invariants

| Metric | Definition | Qualification target |
|---|---|---:|
| Mutation-before-intent | Mutating transport calls without a successfully synced, matching intent | 0 |
| Duplicate physical-effect admission | Same logical attempt or idempotency key can cause more than one effect | 0 |
| Payload alias acceptance | Same key with a different canonical request payload is accepted | 0 |
| Reset-proof transfer | A dispatch uses a plan/profile/world/epoch/clock other than its verified reset binding | 0 |
| False no-effect proof | System concludes no effect without independent, current, owner-qualified evidence | 0 |
| Unsafe recovery admission | New attempt admitted while any prior effect/queue/authority/observer state remains unresolved | 0 |
| Stale evidence acceptance | Wrong epoch, wrong attempt, stale observation window or mismatched artifact is accepted | 0 |
| Fail-open corruption handling | Torn, reordered, mutated or unknown-version journal is accepted or silently repaired | 0 |
| Common-cause false independence | Observers sharing a declared fault domain are treated as independent | 0 |
| Dangerous-mutant detection | Mutants violating any safety invariant survive the qualification suite | 0 survivors for the defined mutation catalogue |

### Effectiveness and operational performance

| Metric | Definition | Initial target |
|---|---|---:|
| Valid campaign yield | Valid evidence executions / admissible planned executions | >=95% after toolchain stabilization; report every invalid reason |
| Recovery projection completeness | All intents/outcomes in a valid journal reconstructed with exact identities | 100% |
| Reconciliation completeness | Ambiguous outcomes reaching a qualified terminal disposition under the specified owner-ledger assumptions | >=99.9%; unresolved residuals remain visible |
| Deterministic replay | Same manifest, pinned inputs and source observations produce byte-identical projections | 100% |
| Restart identity continuity | Recovered plan, logical effect, attempt, payload, world and clock match pre-crash identities | 100% |
| Recovery latency | Time from process restart to safe read-only recovery state | Report p50/p95/p99/p99.9; target from hazard budget |
| Journal append latency | Intent append + sync latency on target media | Report p50/p95/p99/p99.9; p99 must fit the documented mutation-deadline budget |
| Runtime overhead | CPU, memory, I/O and control-loop jitter from instrumentation | Under predeclared system budget; report worst-case, not only average |
| Evidence provenance | Required artifacts with hash, owner, revision, scope and timestamp/clock bound | 100% |
| Unresolved effect ratio | Logical effects whose physical outcome cannot be qualified | Minimize, never hide or coerce into success/failure |

Do not set an absolute sub-millisecond journaling goal before measuring the target filesystem and storage device. A durable sync may cost more than a controller period; in that case the design must move the barrier out of the hard real-time loop or provide an independently qualified durable owner-side intent mechanism. Never remove the barrier to improve latency.

### Statistical power and confidence

A zero-failure run is not proof of a zero failure rate. For a binomial independent-trial approximation, the one-sided 95% upper bound after zero observed failures is approximately `3/n` (the rule of three). Thus:
- 3,000 zero-failure trials only support an upper bound near 1e-3;
- 30,000 support near 1e-4;
- 300,000 support near 1e-5.

These are only approximations and require independent, representative trials. Fault campaigns are often correlated; use stratified crash-cut, fault-family, simulator, filesystem and device results, and do not combine unlike strata into a misleading single denominator.

## 6. Mandatory baselines and ablations

For each matched campaign, compare:
1. no fault injection / native baseline;
2. ROS `ros2_fault_injection` with its standard assertions;
3. EAST-compatible validity-aware fault generation and its published assessment method;
4. the current RRVR adapter without durable journal;
5. RRVR with durable journal but without the remote injector's durable ledger (expected limitation/control);
6. RRVR with a truly durable owner-side key/payload/outcome ledger;
7. naïve retry mutant;
8. dispatch without independent observation;
9. observer quorum with declared common-cause analysis removed;
10. plan/world/clock binding removed.

Report matched fault budget, seeds, task/policy, simulator version, ROS distribution, hardware, timing, crash cut and all exclusions. Never compare unlike fault budgets or task distributions.

## 7. Storage benchmark and architectural decision

The current file-backed journal is a reference implementation, not presumed superior to databases. Compare it with SQLite WAL and at least one embedded Rust transactional store under the same event schema and durability policy.

Measure:
- append+sync latency distributions at 1, 10, 100, 1,000 and 10,000 events;
- replay time and peak memory for 1e3, 1e5 and 1e6 entries;
- writer contention and read-only projection latency;
- crash at every write/sync boundary;
- torn tail, middle corruption, stale lock, full disk and delayed/failed sync;
- disk amplification and bytes per event;
- determinism of recovered projections.

SQLite WAL is a relevant mature baseline, but its documented same-host/shared-memory constraint and sync-mode trade-offs must be included. Test a current fixed SQLite release, not an obsolete build. The custom log earns its place only if the narrow typed state machine, fail-closed replay and integration simplicity justify the operational cost.

## 8. Evidence artifact format

Every benchmark result should carry:
- benchmark ID/version and immutable campaign-manifest digest;
- source revision and build/toolchain lock digests;
- simulator, ROS, DDS, injector and hardware versions;
- seed, fault operator, target, run/attempt identity and crash cut;
- expected and observed transition traces;
- raw transport response and independent owner artifact digests;
- clock identity, units, resolution and uncertainty;
- common-cause fault model and observer revisions;
- model/config/TLC digests and counterexample artifacts;
- per-trial verdict, unresolved residual and exclusion reason;
- raw metrics plus aggregate confidence intervals;
- provenance chain and machine-readable report schema.

An invalid or missing artifact must reduce valid-evidence yield; it must not be silently excluded from the denominator.

## 9. Research hypotheses that can be falsified

H1. A plan/world/clock-bound durable intent journal plus a durable injector ledger can prevent duplicate physical-effect admission under all modeled coordinator restarts and selected injector failure cuts.

H2. Request-bound read-only reconciliation reduces unsafe retry admissions compared with naive retry, without materially reducing valid campaign yield after the injector ledger is available.

H3. Independent physical and downstream observer evidence reduces false no-effect conclusions under common-cause injection compared with simple observer majority.

H4. The recovery projection is transport-independent: different ROS 2/Gazebo/injector adapters emit the same typed journal state and RRVR verdict for equivalent traces.

H5. Evidence-quality scoring exposes failures missed by task success, fault-activation assertions or single-sensor observation alone.

A hypothesis is not supported by TLA checking alone. Require model counterexamples, Rust property tests, simulator fault campaigns, HIL, and independent physical evaluation as distinct evidence levels.

## 10. Prior-art links

- EAST, *Embodied AI Safety Testing via Systematic Fault Injection in Digital Twins* (25 Oct 2026): https://arxiv.org/abs/2610.34358
- SafeVLA-Bench method and live leaderboard: https://safevla.org/ and https://safevla.org/leaderboard.html
- ROS 2 fault injection: https://github.com/reeceholland/ros2_fault_injection
- ROS-RVFT verification/testing guidelines: https://ros-rvft.github.io/
- Gazebo ROS 2 Simulation Interfaces: https://gazebosim.org/docs/latest/ros2_sim_interfaces/
- AWS Builders' Library, *Making retries safe with idempotent APIs*: https://aws.amazon.com/builders-library/making-retries-safe-with-idempotent-apis/
- ROS 2 tracing: https://github.com/ros2/ros2_tracing
- NIST Performance Assessment Framework for Robotic Systems: https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems
- SQLite Write-Ahead Logging and sync/durability notes: https://www.sqlite.org/wal.html and https://www.sqlite.org/howtocorrupt.html

## 11. Current implementation status

Implemented on the stacked draft branches:
- plan-bound Gazebo reset/dispatch composition and in-memory reset lifecycle;
- versioned dispatch key and payload binding to plan/run/world epoch/clock;
- independent observer blinding and exact-step quorum assessment;
- bounded reset TLA model with unsafe retry and plan-substitution mutants;
- file-backed typed append-only journal, hash-chain replay, stale writer lease handling and deterministic recovery projection;
- read-only `rrvr_journal_verify` CLI, schemas and cross-language known-answer fixture.

Not yet demonstrated:
- live adapter integration of the journal before each real simulator/injector mutator;
- remote injector durable/linearizable idempotency ledger;
- TLA/Rust transition-by-transition refinement;
- ROS/Gazebo head-to-head campaign;
- filesystem/power-cut qualification on deployment hardware;
- HIL or independent physical robot results.

Do not promote a proposed target to a result until exact-head evidence exists for the corresponding test layer.
