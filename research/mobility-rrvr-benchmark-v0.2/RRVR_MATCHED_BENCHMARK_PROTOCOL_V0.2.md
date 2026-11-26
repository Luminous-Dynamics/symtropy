# RRVR Matched Benchmark Protocol v0.2

**Status:** research protocol and synthetic known-answer fixture. No campaign result or SOTA win is claimed by this document.

**Parent:** [#7106 — matched evidence, fault-delivery, recovery and diagnosis benchmark suite](https://github.com/Luminous-Dynamics/symtropy/issues/7106)

**Protocol principle:** preserve every host benchmark's native scores, then measure execution/recovery assurance as a separate axis. RRVR must not improve its apparent result by changing the task, policy, initial state, fault, oracle, exclusion rule or denominator.

## 1. Research objective

Evaluate whether a robotic system can recover after a distributed interruption while preserving exact physical-effect identity, preventing stale work from executing, distinguishing uncertain effects from known outcomes, and admitting the next attempt only through its existing authority owner.

RRVR is an assurance overlay, not a replacement for task-success, VLA safety, fault-injection validity or policy-recovery benchmarks.

The lifecycle under test is:

`Intent → Authority → Dispatch → Owner acceptance → Execution → Independent observation → Effect reconciliation → Downstream temporal closure → Recovery admission`

Each transition has a distinct owner, evidence source, fault model and claim ceiling.

## 2. SOTA and industry baselines to adopt

These are complementary baselines with different research questions. Their native metrics must remain intact.

| Baseline | What it establishes | RRVR adopts | RRVR adds |
|---|---|---|---|
| EAST (2026) | Validity-aware safety fault injection in digital twins; independent verification of fault realization and safety outcomes | Applicability checks, executable fault operators, independent fault-realization evidence, matched budgets | Distributed crash/restart cuts, exact effect identity, late command execution and endpoint fencing |
| RoboRecover (2026) | Policy recovery from execution-induced deviations reconstructed by replaying action prefixes | Prefix replay, deviation-state starting points, held-out recovery evaluation | Recovery continuity across coordinator/injector/device restart and evidence-qualified next-attempt admission |
| SafeVLA-Bench (2026) | Native task success and safety are distinct; exposes successful-but-unsafe episodes and violation severity | Preserve SR, Safety, SBU and violation severity exactly | False NoEffect/Applied/Closed decisions, duplicate effects, stale execution and recovery-authority validity |
| ROS 2 Control / DDS | Controller/hardware lifecycle and middleware freshness/liveliness behavior | Exact release, plugin, RMW/DDS and QoS profile; record lifecycle/QoS observations | Receiver/driver/firmware retention, endpoint acceptance, effect outcome and path closure |
| NIST PAFA | Task-driven composition of subsystem performance into robot-system assessment | Keep task/system decomposition and native subsystem measures | Recovery/evidence assurance as a separate, non-substitutable dimension |

Prior-art facts used for this protocol:
- EAST (arXiv:2610.34358, 2026-10-25) targets 74 admissible task-policy-operator combinations, obtains valid evidence for 63, and reports 82 valid executions that completed tasks despite safety violations.
- RoboRecover (arXiv:2609.28952, 2026-09-24) includes 2,000 scenarios across RoboTwin and LIBERO, with replayed action prefixes and fixed train/test splits.
- SafeVLA-Bench (arXiv:2606.00773; leaderboard updated 2026-09-26) reports 22,500 episodes, five suites, eight safety specifications and 24 policies; its metrics separate native task success (SR), episode safety, successful-but-unsafe episodes (SBU) and violation severity (VSI).
- The July 2026 ROS fuzzing survey (DOI 10.3390/app16147250) reviews 92 selected works and finds uneven evidence for repeatable execution/reset and diagnosable failure analysis compared with input modelling, feedback and oracle construction.

Sources:
- EAST: https://arxiv.org/abs/2610.34358
- RoboRecover: https://arxiv.org/abs/2609.28952
- SafeVLA-Bench: https://arxiv.org/abs/2606.00773 and https://safevla.org/leaderboard.html
- ROS fuzzing survey: https://doi.org/10.3390/app16147250
- ROS 2 Control Lyrical lifecycle: https://control.ros.org/lyrical/doc/ros2_control/hardware_interface/doc/lifecycle_of_a_hardware_component.html
- ROS 2 QoS design: https://design.ros2.org/articles/qos_deadline_liveliness_lifespan.html
- NIST PAFA: https://www.nist.gov/publications/performance-assessment-framework-robotic-systems-pafa

## 3. Benchmark dimensions

Keep the nine existing RRVR families:

- **JRN** — journal durability and recovery.
- **RST** — reset lifecycle and crash-cut execution.
- **IDN** — logical-effect and attempt identity.
- **EFX** — physical-effect outcome classification.
- **FNC** — authority and downstream execution fencing.
- **OBS** — observer validity and fault-model-relative common-cause assurance.
- **LIN** — evidence lineage and owner reconciliation.
- **TRN** — transport portability and platform-specific semantics.
- **SYS** — full-system composition and native authority handoff.

Retain the two cross-cut dimensions:
- **CUT** — interruption and crash-cut coverage across processes, storage, middleware, drivers, firmware and endpoints.
- **EPC** — endpoint effect identity, token, NVM history and authority-epoch conformance.

Add two separately reported dimensions:
- **TCL** — temporal closure: can any old accepted command still arrive, be re-enqueued or execute after a closure certificate?
- **SEM** — command semantic conformance: is the command correctly classified as desired state, one-shot, continuous lease or trajectory against the exact device/firmware capability profile?

A high score in one dimension must not conceal a failure in another. Do not calculate one weighted RRVR headline score.

## 4. Matched experimental arms

For each shared task, scene, policy, initial state, action prefix, seed, simulator build, robot asset, reset profile, fault schedule, observer profile and expected disposition, execute as many of the following arms as the baseline supports:

A. **Native baseline** — unmodified host benchmark; retain all native task/safety metrics.

B. **EAST-compatible validity campaign** — same fault budget; measure requested, accepted, executed and independently observed fault separately.

C. **RoboRecover-style deviation recovery** — reconstruct the same deviation state by replaying the same action prefix; report native policy recovery success/cost.

D. **Restart/retry baseline** — conventional process restart and retry handling without RRVR evidence composition.

E. **Journal-only** — coordinator intent journal and restart projection, without injector-owner reservation.

F. **Journal + injector owner** — durable same-key reservation, payload binding and owner lookup.

G. **Endpoint-fenced** — above plus endpoint-enforced one-shot token and monotonic authority epoch.

H. **Graph + temporal closure** — above plus complete declared command-holding graph, in-flight accounting and temporal-closure evaluation.

I. **Full RRVR** — above plus independently qualified physical-effect observation, observer common-cause analysis and existing native authority admission.

Do not invent a result for an unsupported arm. Mark it unavailable with a reason. Never weaken a baseline or change policy/task inputs to manufacture an RRVR advantage.

### 4.1 Pairing key

A matched pair must preserve exact equality of:
- task and scenario ID;
- policy/model/checkpoint digest;
- scene/world and robot asset digest;
- initial-state digest;
- action-prefix digest and event count;
- random seed and deterministic RNG profile;
- simulator/ROS/RMW/controller/driver/firmware profile;
- reset profile and fault schedule (including order and target);
- observer profile and measurement envelope;
- expected fault disposition and oracle revision.

The compared RRVR implementation/assurance arm is the only intended difference. Any additional difference makes the pair non-comparable and must be reported as an exclusion with a typed reason.

## 5. Scorecards and metric definitions

### 5.1 Native task and safety scorecard

Retain the host benchmark's metrics exactly. Where available, publish:
- native task success rate (SR);
- episode safety rate (every applicable specification satisfied);
- successful-but-unsafe (SBU) rate;
- violation count and severity (including raw force/trajectory measures);
- policy recovery success and recovery cost.

Do not rename RRVR assurance as task success or policy recovery.

### 5.2 Fault validity and execution scorecard

Publish separate counts and rates for:
- fault requested;
- fault accepted by injector;
- fault actually executed;
- fault independently observed;
- fault realization valid under the frozen applicability contract;
- invalid/partial/ambiguous injection;
- positive-control detection and negative-control rejection.

A requested fault is not an executed fault. An API success response is not independent fault-delivery evidence.

### 5.3 Physical-effect and recovery assurance scorecard

Use exact attempt-scoped denominators:
- **False NoEffect rate** = false NoEffect verdicts / independently adjudicated cases where NoEffect was not true.
- **False Applied rate** = false Applied verdicts / independently adjudicated cases where Applied was not true.
- **False Closed rate** = temporal-closure certificates invalidated by an executable old command / closure certificates issued.
- **Duplicate-effect rate** = logical-effect attempts with more than one independently observed physical effect / eligible one-shot attempts.
- **Stale-effect-after-fence rate** = old-epoch physical effects after accepted endpoint fencing / fenced endpoint trials.
- **Unsafe admission rate** = fresh attempts admitted while any required evidence/authority precondition was false / fresh-attempt admission trials.
- **Unknown/Disputed fraction** = unresolved or contradictory outcomes / all attempted effects.
- **False-hold rate** = positive-control opportunities held despite all required evidence being valid / positive-control opportunities.
- **Evidence yield** = eligible cases with a reconstructable, scope-valid, independently checkable evidence bundle / eligible cases.

Do not classify Unknown as NoEffect. A false safe verdict is a hard failure; false holds and availability are reported separately.

### 5.4 Temporal closure (TCL)

Report:
- old commands delivered after upstream queue-empty reports;
- late command arrivals after certificate;
- old-epoch effects after certificate;
- unaccounted in-flight messages;
- inventory edges declared vs independently discovered;
- path-bound availability, invalidation causes and unresolved fraction;
- sequence-barrier closure latency;
- bounded-expiry horizon and measured distribution;
- clock uncertainty, message-age refresh, forwarding cycles and unbounded-path holds.

A finite waiting period is not a temporal proof where any reachable path lacks a qualified finite transit/retention bound or endpoint fence.

### 5.5 Command semantics (SEM)

Report separately per command class and device/firmware profile:
- semantic-class confusion matrix;
- false-idempotency classification rate;
- relative-command replay misclassification;
- one-shot cardinality/token conformance;
- lease epoch/expiry/watchdog-response conformance;
- trajectory segment replay, skip and measured-resume conformance;
- frame/unit/payload/profile substitutions rejected;
- unknown capability and conservative hold rate.

Never pool the four effect classes into one headline accuracy. A false idempotency classification is a hard negative-control failure, not just one classification error.

### 5.6 Diagnosis and reproducibility

Report:
- exact failure-signature replay rate;
- deterministic replay identity for fixed plan, binary, simulator, seed and artifacts;
- minimized event count and bytes while preserving the exact property/witness signature;
- 1-minimality achieved vs budget-exhausted/incomplete minimization;
- root-cause top-1/top-3 accuracy against independently adjudicated labels;
- time-to-replay, time-to-minimize and time-to-explanation;
- inter-reviewer agreement and unresolved label fraction.

Do not count a different failure signature as successful reproduction or minimization.

### 5.7 Performance and availability

Report p50, p95, p99 and p99.9 for:
- append and durable sync;
- owner reservation and reconciliation;
- reset/reopen and journal replay;
- endpoint fence and token lookup;
- downstream queue drain and temporal closure;
- independent physical observation and outcome reconciliation;
- fresh native authority admission.

Also report CPU/memory/storage overhead, bytes per evidence record, operator interventions, false holds and mission makespan **after** all hard evidence/safety gates pass.

## 6. Statistical protocol

- Preregister scenarios, task/policy profiles, exclusions, oracles, metrics, fault schedules, stopping rules and analysis.
- Preserve raw denominators and confidence intervals, not only percentages.
- Separate independent trials from repeated/clustered episodes.
- Cluster by scene/world, policy checkpoint, seed, robot asset, fault schedule, hardware/firmware profile and observer configuration as appropriate.
- Use paired comparisons for matched episodes and cluster-aware bootstrap or hierarchical models where observations share a cluster.
- Do not pool heterogeneous hardware/firmware profiles into one probability estimate.
- Publish failure counts and exact confidence intervals even when zero failures are observed.

For a one-sided 95% binomial upper bound below (10^{-3}) after zero observed failures, approximately 3,000 genuinely independent representative trials are needed. Correlated episodes do not satisfy this assumption and cannot be counted as independent trials.

## 7. Required evidence bundle

Each compared arm must publish a machine-readable manifest and retain:
- plan/campaign and pair IDs;
- source manifest, compiled plan and exact ordered schedule commitments;
- task/policy/scene/action-prefix/robot/simulator/profile digests;
- toolchain and binary digests;
- raw requested/accepted/executed/observed fault artifacts;
- journal, injector-owner and endpoint receipts as distinct source-specific artifacts;
- endpoint token/epoch and queue-generation evidence;
- physical outcome evidence and independent observer proofs;
- graph inventory and per-node/per-edge quiescence receipts;
- temporal-closure verdict and exact proof path;
- semantic profile and device capability profile;
- exact model/oracle revisions, logs and expected-mutant counterexamples;
- minimized reproducer and replay result;
- qualification tier and claim ceiling.

A hash chain or manifest proves byte identity under its assumptions; it does not authenticate a producer or prove physical truth.

## 8. Qualification tiers

Report separate strata:
1. specification/schema;
2. bounded formal abstraction;
3. exact-head compile/unit/static analysis;
4. process crash/restart and storage fault injection;
5. simulator/digital twin;
6. HIL;
7. physical hardware with independent metrology;
8. independent safety/security assessment and any required conformity/certification.

No lower tier implies a higher one. Process termination is not power-loss qualification. Gazebo is not firmware. HIL is not an application safety certification. RRVR does not establish PL/SIL.

## 9. Research targets (not achieved results)

On frozen negative controls:
- 0 false NoEffect, false Applied and false Closed verdicts;
- 0 duplicate one-shot physical effects;
- 0 stale-epoch effects after confirmed endpoint fencing;
- 0 unsafe recovery admissions;
- 100% declared crash-cut, command-holder node/edge and semantic-class coverage;
- 100% rejection of frozen source/plan/schedule/target/epoch/clock/device-capability mutation controls;
- 100% exact-signature reproduction for the frozen known-answer corpus.

Utility goals, set per platform rather than universally:
- ≥99.9% valid positive-control admission with confidence intervals;
- ≥95% valid evidence yield;
- report false-hold cost separately;
- ≥25% lower verified mission makespan than a safe sequential baseline only after all hard evidence gates pass;
- ≥50% median counterexample reduction only when the exact failure signature and causal prerequisites are preserved.

These are research goals. They are not statements of current RRVR performance or SOTA superiority.

## 10. Claim ceiling

A benchmark pass qualifies only the exact frozen tasks, profiles, arm implementations, fault campaigns, observers, oracles and evidence tiers that actually ran. It does not establish general robot safety, all-hardware coverage, functional safety, PL/SIL, standards compliance, certification or field reliability.

A SOTA comparison is valid only where the native benchmark, task/policy/scene/action prefix, fault budget, oracle, and evidence tier are genuinely matched. RRVR's intended contribution is an additional, reproducible assurance axis—not a replacement leaderboard claiming that task-success or safety benchmarks are obsolete.
