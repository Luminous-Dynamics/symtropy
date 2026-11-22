# CDAAB Research and Targets v0.3 — Authority Linearizability and Benchmark Validity

**Date:** 2026-11-21  
**Status:** research proposal / target ledger; not an achieved benchmark result  
**Parent:** [CDAAB v0.2](CDAAB_RESEARCH_AND_TARGETS_v0.2.md)  
**Implementation work:** [MOBILITY-MUTATION-BOUNDARY-001 #6669](https://github.com/Luminous-Dynamics/symtropy/issues/6669) / [draft PR #6671](https://github.com/Luminous-Dynamics/symtropy/pull/6671)

## 1. Research thesis

The next useful claim in autonomous systems is not simply that a planner selected a successful action. It is that the *mutation itself* was admitted under the exact authority and context that remained current at the native application boundary, and that the resulting physical effect was separately established.

CDAAB should evaluate three independent propositions:

1. **Authority-linearizable application:** every native mutation can be placed at a legal linearization point against the current M1/M2/platform authority snapshot. A stale, revoked, expired, replayed or mismatched ticket never reaches the native sink.
2. **Evidence-correct outcome:** native acceptance, actuator response, observed physical effect and mission completion remain distinct. An independent evaluator can reproduce the outcome claim and its source lineage.
3. **Epistemically safe recovery:** every native application occupies a single unresolved-mutation slot until independent outcome reconciliation. Unknown effects retain the hold; NoEffect and PartialEffect require a one-shot M1 permit bound to the failed attempt, current authority epoch and exact next action binding. A timeout is not evidence of no effect; retries require a new independently justified action/attempt.

Linearizability itself is established concurrency theory, not a new invention. Herlihy and Wing define the property as the illusion that each concurrent operation takes effect at one instant between invocation and response, while preserving real-time order for non-overlapping operations. CDAAB's research contribution is to make the *complete authority binding and one-shot physical mutation* the sequential specification being checked, then connect that application history to independent physical-outcome evidence.

Reference: Herlihy and Wing, “Linearizability: A Correctness Condition for Concurrent Objects,” ACM TOPLAS 12(3), 1990, https://doi.org/10.1145/78969.78972

## 2. Proposed measurable property: authority-linearizable execution

Define an abstract boundary state:

```text
S = (
  current_authority_binding,
  active_M1_action_attempt,
  controller_generation,
  qualified_clock_basis,
  consumed_ticket_ids,
  next_application_sequence,
  unresolved_physical_effects
)
```

The sequential specification has two distinct transactions.

### A. Native application transaction

```text
Apply(ticket, qualified_now):
  reject unless ticket.binding == current_authority_binding
  reject unless ticket.attempt == active_M1_action_attempt
  reject unless ticket.controller_generation == current_generation
  reject unless qualified_now uses the ticket clock + boot epoch
  reject unless issued_at <= now < expires_at
  reject unless ticket_id has never been consumed
  atomically consume ticket_id and invoke native application
  return a receipt bound to ticket + binding + generation + sequence
```

The check, one-shot consumption and actual native acceptance must share one linearization boundary. An API that checks and then forwards to another unfenced queue is not conformant.

### B. Physical outcome transaction

```text
Observe(ticket, independent_receipt):
  validate exact mission/subgoal/action/attempt/predicate identity
  validate source, sequence, calibration, frame and qualified time lineage
  validate declared observer dependency path against executor path
  Satisfied   -> eligible for M1 completion
  NoEffect    -> M1 recovery path only
  Partial     -> reconciliation hold
  Unknown     -> reconciliation hold
```

Application acceptance does not prove physical effect. The outcome transaction must remain independently evidenced and may occur after the application transaction.

### The key history property

For each observed concurrent history, there must exist a sequential ordering that:
- preserves real-time order of non-overlapping operations;
- obeys the abstract boundary state machine;
- never applies one ticket identity twice;
- rejects every operation whose full binding is stale at its linearization point;
- records ambiguous application results as unresolved, not as definite rejection;
- does not allow a later mission-completion transition without matching independent outcome evidence.

A history checker should return **PASS**, **FAIL** or **INCOMPLETE**. Bounded exploration that did not cover the complete declared schedule space must never report PASS.

## 3. External benchmark landscape — keep unlike scores separate

The latest public results available on 21 November 2026 provide useful but non-interchangeable comparators.

| Benchmark | Latest relevant public result | What it measures | CDAAB's additional question |
|---|---|---|---|
| LADDER | Research page reports 62.0% for GPT-Policy/GPT-6 Astra; live leaderboard lists 63.3%. Resolve exact run/evaluator definitions before selecting one headline baseline. | Long-horizon manipulation/task completion | Did each subgoal actually complete? Were action attempts authorized at application? How did the system recover from ambiguity? What was the full cost per verified episode? |
| RoboCasa365 | Official 50-task leaderboard updated 16 Nov 2026: HexaShell 85.6 overall; 91.1 Atomic-Seen; 85.0 Composite-Seen; 80.0 Composite-Unseen. | Multi-task kitchen manipulation on three splits | Report same splits, plus safety-spec compliance, success-but-unsafe, worst-violation severity, unauthorized actions and independent effect evidence. |
| SafeVLA-Bench | 24 policies, 5 suites, 22,500 episodes and 8 safety specifications. Its RoboCasa-365 table includes a 81.9% native-success policy with 50.1% safety and 37.7% success-but-unsafe; the top safety-ranked policy has 53.1% safety and 58.4% success. | Task success separately from safety specification compliance and violation severity | Use safety and SBU as independent non-compensatory axes; add action authority, time-of-check/time-of-use, outcome integrity and recovery. |
| Bench2Drive v0.0.4 | LEAD: 95.59 driving score and 87.28% success on the published 220-route validation set. | Closed-loop autonomous driving under the exact versioned route/evaluator setup | Add revocation, stale-map/navigation, clock discontinuity, delayed sensor, V2X common-cause and ambiguous actuator/vehicle outcome schedules. Recompute comfort using motion/distance-aware diagnostics and audit that navigation inputs are not overridden by the evaluation harness. |
| PX4 engineering maturity | CI, SITL, bench hardware testing and HIL are separate stages. Bench tests expose boot ordering, USB/link lifecycle, storage, loop-rate drift and concurrency defects that desktop SITL may miss. | Embedded flight-stack testing and hardware integration | Preserve evidence levels: software contract != simulator != HIL != supervised physical operation. |
| NASA runtime assurance / Simplex | A runtime monitor detects violation of a safety property and transfers control from an advanced controller to a trusted reversionary controller. | Runtime safety monitoring and safe fallback for autonomous UAS | CDAAB is complementary, not a replacement: it qualifies whether a particular action remains authorized and current at native application, and whether the subsequent physical effect is independently evidenced. Domain RTA and local safety interlocks remain authoritative. |

Primary references:
- LADDER research: https://ladderrobotics.ai/research
- LADDER live leaderboard: https://ladderrobotics.ai/
- RoboCasa365 leaderboard: https://robocasa.ai/leaderboard
- SafeVLA-Bench: https://safevla.org/leaderboard.html
- Bench2Drive v0.0.4: https://github.com/Thinklab-SJTU/Bench2Drive/blob/0.0.4/docs/v004_update.md
- PX4 bench testing: https://docs.px4.io/v1.18/en/test_and_ci/bench_testing
- NASA runtime assurance framework for autonomous UAS: https://ntrs.nasa.gov/citations/20240007986

These scores must not be averaged, ranked against one another or presented as if they shared task definitions. Runtime assurance architectures such as Simplex address controller-level safety monitoring and fallback; they are an essential complementary layer, not a baseline that action-ticket checks can replace. LADDER's 62.0/63.3 mismatch is an explicit baseline-reconciliation item, not a rounding adjustment.

## 4. Relationship to existing safety standards and safety cases

CDAAB is an engineering benchmark and evidence format, not a safety standard, certification route or complete safety case.

- **ISO 34502:2022** describes a scenario-based ADS safety-evaluation framework scoped to limited-access highways. The published scope explicitly excludes misuse, HMI and cybersecurity, and excludes non-safety topics such as comfort, energy and traffic-flow efficiency. CDAAB should add adversarial authority/revocation, queue/replay, common-cause, evaluator-integrity, operator-intervention and cost/energy evidence as complementary dimensions; it must not present these as already covered by ISO 34502.
- **ISO 21448:2022 (SOTIF)** provides an argument framework and guidance for hazards caused by functional insufficiencies and situational-awareness limitations, including complex sensors/processing and relevant operation-phase activities. CDAAB's stale/conflicted/unqualified observation and independent-effect evidence can contribute concrete verification evidence to a domain SOTIF argument, but cannot replace that argument. The official ISO catalogue currently shows a working draft for a successor edition; implementations must pin the edition they actually use.
- **UL 4600 Edition 3** is an active standard for developing and evaluating a safety argument for autonomous products, including lifecycle risk mitigation and evidence. CDAAB run receipts can support the evidence layer of such a safety case; they do not establish that the entire argument is complete, valid or accepted.

References:
- ISO 34502:2022: https://www.iso.org/standard/78951.html
- ISO 21448:2022: https://www.iso.org/standard/77490.html
- ISO working draft successor listing: https://www.iso.org/cms/%20render/live/en/sites/isoorg/contents/data/standard/09/30/93071.html
- UL 4600 Edition 3: https://www.shopulstandards.com/ProductDetail.aspx?productid=UL4600

## 4. Benchmark the benchmark

A 2026 CoRL audit of LIBERO, CALVIN, SimplerEnv, RoboCasa and RoboTwin 2.0 identified shortcut solvability, statistical significance, creeping overfitting and data-source dependence. It reports that only 19.8% of audited LIBERO SOTA claims were provably statistically significant; a 0.09B probe without a language encoder reached near reported LIBERO SOTA; and CALVIN's average tasks-completed score dropped from 4.17 to 3.14 when object poses were resampled within the training range.

CDAAB should require four corresponding diagnostics before performance claims are published:

1. **Shortcut challenge:** small probe baselines, trivial heuristics, no-language and no-planner controls; test whether the benchmark can be solved without the claimed capability.
2. **Statistical challenge:** preregister a minimum meaningful effect, sample size and analysis; publish per-stratum denominators and confidence intervals.
3. **Overfit challenge:** resample objects, poses, scene layout, lighting, timing and fault combinations within and outside training support; retain a private confirmatory set.
4. **Data-source challenge:** disclose train/test source overlap, demonstration provenance, simulator version, pretraining coverage and privileged-state access.

References:
- Jiang et al., “What Are We Actually Benchmarking in Robot Manipulation?” (CoRL 2026): https://arxiv.org/abs/2606.04233
- Audit diagnostics: https://ripl.github.io/manipulation_benchmark_audit/

### Additional closed-loop evaluator pitfall: “Comfortably Stuck”

The October 2026 Bench2Drive audit reports that the published comfort metric correlates with stationary time at approximately r = 0.97 across eight models; a corrected distance-based metric reduces that correlation to r = 0.06. It also identifies controller-side steering override and training/evaluation route-target mismatch. This is directly relevant to CDAAB: the evaluator/controller harness is part of the system under test and can silently replace the policy behavior or reward inactivity.

Add mandatory checks for:
- stationary-time / comfort correlation;
- distance-normalized comfort and progress;
- zero hidden steering override or route-command substitution;
- matched navigation-signal distribution and disclosure of train/evaluation mismatch;
- stationary-but-not-progressing episodes retained as failures or incomplete, never as task success.

Reference: Schwarzfischer et al., “Comfortably Stuck: Evaluation Pitfalls in Bench2Drive's Closed-Loop Protocol” (15 Oct 2026), https://arxiv.org/abs/2610.21741

## 5. CDAAB target ledger

All values below are **research targets**, not current results or operational safety claims.

### Hard assurance gates

| Metric | Target | Required evidence |
|---|---:|---|
| Stale/revoked/expired/mismatched native application | 0 accepted in frozen negative corpus | Exact sink-call-count oracle plus independent schedule evaluation |
| Duplicate/replayed ticket application | At most 1 native application per immutable ticket ID | Concurrent replay and restart/reorder corpus |
| Unknown/partial effect followed by blind retry | 0 | Controller timeout, lost reply, partial effect and delayed receipt schedules |
| False mission completion | 0 in safety-critical negative corpus | Independent physical/evaluator truth; outcome receipts bound to exact attempt |
| Negative-control detection | 100% of preregistered mutation families | Mutations covering authority, configuration, ODD/mode, clock, navigation, observer dependency and ticket identity |
| Valid positive-control admission | At least 99% | Frozen positive fixtures; report unnecessary-hold rate and confidence interval |
| History conformance | 100% of fully explored bounded histories linearizable | Independent sequential-specification checker; incomplete exploration is INCOMPLETE |
| Pending mutation hold | 0 new native mutations while an earlier action lacks a resolved independent outcome | Cross-history oracle and native sink-call-count assertions |
| Recovery permit scope | 0 accepted attempt/epoch/plan mismatches | Typed permit lineage and negative mutation corpus |
| Recovery permit replay | 0 duplicate permit consumptions | Global permit-ID ledger and cross-attempt replay tests |
| Reconciliation hold release | Only qualified Satisfied, NoEffect-with-permit or PartialEffect-with-replan-permit transitions may clear the hold | M1-owned outcome/permit integration; Unknown always retains the hold |
| Receipt lineage | 100% of required fields and exact digests present | Independent schema + semantic validator; incomplete records never promoted |

### Utility and recovery targets

| Metric | Target hypothesis | Guardrail |
|---|---:|---|
| Nominal task success | Non-inferior to matched base policy within 1 percentage point for the first assurance ablation | No authority/evidence violation allowed |
| Fault recovery | At least 90% successful recovery on the preregistered recoverable-fault subset | Zero unsafe recovery and zero blind retry |
| Operator intervention | At least 10% relative reduction per independently verified completion | Hold and escalation remain valid outcomes; do not optimize by hiding interventions |
| Resource cost | At least 10% lower all-in cost per verified completion at matched task success | Include sensing, compute, power, network, rework and operator time |
| Invalidation response | Report p50/p95/p99/max invalidation-to-fence latency; domain owner sets the absolute bound | Bound must be below the applicable control/application deadline, not a universal CDAAB number |

### Task-performance targets

Only after the assurance gates are stable:
- LADDER: matched full-suite run, same task suite, embodiment, policy budget and evaluator; aim for at least 5 absolute percentage points improvement in *verified* task success or a materially better verified-success/cost Pareto frontier. Keep 8–10-subgoal results separate.
- RoboCasa365: target at least 90.6% overall with a separate Composite-Unseen floor of at least 85%, on the exact 50-task protocol. Safety/SBU/worst severity remain hard independent axes; no task score compensates for an unsafe success.
- Bench2Drive v0.0.4: target to exceed LEAD's 95.59 driving score and 87.28% success on the same 220 routes only after the closed-loop adapter, exact evaluator and safety strata are available. This score is not a physical-road qualification.
- Report cost, energy, operator intervention and recovery separately; no weighted “autonomy index” should hide tradeoffs.

## 6. Matched baseline tournament

Keep the policy/controller fixed to isolate the contribution of each assurance layer:

- **A — Direct policy/controller:** existing task policy with its baseline progress evaluator.
- **B — M1 supervision:** independent goal evidence, attempt identity, reconciliation and recovery.
- **C — M1 + M2:** configuration-bound action ticket, current context checks, physical outcome contract.
- **D — C + native application gate:** full binding comparison, generation/lease check and one-shot ticket consumption at the native application linearization point.
- **E — D + active assurance:** choose the next observation using risk reduction, value of information, latency, energy and cost; keep each constraint separate rather than weighting them into a single score.

For task-performance comparison, use identical policies across A–E. Separately compare stronger policy versions; never attribute a policy-model improvement to the assurance layer.

## 7. New paradigm shifts to test

1. **Authority as a revocable temporal capability:** ticket scope, attempt, configuration, ODD/mode, clock basis, navigation evidence and expiry travel with work. A broker or queue cannot extend authority by delay or retry.
2. **Native mutation as a linearizable transaction:** compare the complete live binding, consume a unique ticket and enter the native controller under one application boundary. Fencing generations are useful only if all invalidators are mapped to them.
3. **Physical effect as a second independent transaction:** controller acceptance is not task completion. Independent observers and evaluators establish effect; missing or common-cause evidence remains unknown.
4. **Observation as control:** sensing is itself a resource-constrained action selected for expected information gain, risk reduction and causal discrimination. An observation that shares the same clock, power, bus or processor may add little independence.
5. **Recovery as state restoration:** measure whether the system returns to a known, admissible state—not whether it simply retries until a green task score appears.
6. **Benchmark integrity as a product feature:** hidden evaluator truth, frozen manifests, reproducible receipts, negative controls and audit diagnostics are release gates, not paper appendices.

## 8. Formal verification and reconciliation semantics

The current research branch includes a bounded Rust history checker and a finite TLA+ model in addition to the in-process application reference. The checker distinguishes Linearizable, NonLinearizable and Incomplete. It requires one complete ordered event recorder; missing invocations, responses or trace events cannot produce PASS.

The reference model prevents a second mutation while an earlier native application is pending. An unknown outcome retains the hold. Recovery permits must match the failed attempt, current authority epoch and exact next authority/action binding, and the permit ID is consumed once. This is a structural contract check only; M1 remains responsible for authenticating the permit and validating plan allowlisting.

The TLA+ model expresses the same finite-state concepts and checks invariants for one-shot tickets, exact ticket binding, application sequence, one pending mutation and scoped one-shot permits. The TLC workflow is configured with a pinned stable tool artifact. Until its exact-head workflow passes, no TLC verification result is claimed.

This formal-method layer complements rather than replaces independent physical outcome evaluation or domain-specific runtime assurance.

## 9. Engineering sequence

1. Finish the public contract and dependency ownership map in Symtropy #6669 / PR #6671.
2. Extend the in-process reference model into an independent history checker; exercise adversarial concurrent schedules and one-shot semantics.
3. Adapt the existing controller epoch/application queue to consume the complete M2 binding, not just a controller-local integer epoch.
4. Refresh the M1/M2 receipt adapter pins to the exact current source heads and re-run targeted and broad CI independently.
5. Integrate through one simulated domain adapter only after the native application semantics are conformance-tested.
6. Run a frozen evaluator-separated fault campaign; publish all planned, failed, timed-out and excluded cases.
7. Progress through HIL and approved supervised physical testing only when the named platform/domain owners have provided the required hardware and safety evidence.

## 10. Non-claims

No robot-policy SOTA, simulator score, physical-effect accuracy, sensor truth, cryptographic authentication, complete failure-domain inventory, real-time guarantee, HIL, physical safety, regulatory approval or operational authorization is claimed by this research note or the current reference crate.

The goal is a stronger and auditable autonomy comparison: task capability, authority integrity, outcome truth, recovery and cost on separate axes, with reproducible evidence for every reported claim.
