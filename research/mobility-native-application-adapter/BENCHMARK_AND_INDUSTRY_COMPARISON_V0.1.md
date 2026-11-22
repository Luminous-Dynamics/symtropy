# Native action-application assurance benchmark v0.1

Date: 22 November 2026  
Tracking: Symtropy #6749, under #6669.  
Status: benchmark design and target ledger. No benchmark campaign, simulator replay, HIL or physical result is claimed.

## 1. What is being measured?

This work measures whether a proposed robot action still has the exact authority and operational context that admitted it at the moment of native application. It does not measure whether a VLA policy is good at selecting actions. Keep the layers separate:

- **Policy/task performance:** Was the requested task completed, across seen and unseen scenes?
- **Physical/task safety:** Did the trajectory satisfy contact, stability, bystander, containment and temporal specifications?
- **Application assurance:** Did exactly the currently authorised action-attempt reach the native sink once, under the same configuration, ODD/mode, navigation, authority, clock/boot and controller epoch that justified it?
- **Effect assurance:** What physically happened after native acceptance, and can an independent observer reconcile it to the same attempt?
- **Mission assurance:** Was completion accepted only after the expected predicate was independently established?

The adapter owns only the application seam, while composing the pre-existing M2, mutation-boundary and robotics-queue owners. It must not recreate effect evidence, mission state, safety control or a second queue.

## 2. Paradigm shifts

### From command to expiring proof-carrying commitment

Treat an action as a short-lived commitment over platform configuration, mission/action attempt, ODD and mode, authority, navigation claims, evidence freshness and a declared clock basis. Queueing is not authority. A preflight check is not final authority. The complete live binding must still agree at native acceptance.

### From independent fences to one composed linearization point

A check is meaningful only if it is ordered with the mutation it protects. This adapter places M2 revalidation, independent live-binding comparison, consume-once enforcement, controller-epoch verification and the native callback in one synchronous path. The live-context provider must hold the authoritative snapshot stable through the callback. External state owners must implement a compatible invalidation/serialization contract; a local mutex cannot lock a different process or a firmware queue.

### From one ambiguous time variable to typed clock domains

The queue lease's monotonic nanoseconds and M2's qualified action-clock ticks are different quantities. They may have different clocks, boot epochs, units and correlations. The adapter checks each against its own declared owner and never casts or compares them directly. A boot restart invalidates the old basis rather than silently resetting elapsed time.

### From safety as a weighted score to non-compensatory assurance

One invalid command reaching the sink cannot be compensated for by more successful episodes or faster makespan. Hard invariants are reported separately from task-success, latency and productivity metrics.

## 3. Matched experimental arms

Use the same policy, action proposals, robots, controller, environment, sensors, seeds, task schedule, resource budgets, authority provider and evaluator.

A. **M2 preflight only:** configuration-bound applicability check before enqueue; diagnostic baseline only.  
B. **Epoch queue only:** existing queue epoch and lease checks; does not prove the complete M2 ticket is current.  
C. **Authority boundary only:** #6671 whole-binding/attempt comparison and one-shot consumption; does not replace M2 field semantics or queue lease semantics.  
D. **Full composition:** M2 revalidation inside the callback + independently supplied live binding commitment + #6671 + #2883 queue lease/epoch check + synchronous native sink.  
E. **Fault-injected bypass controls:** controlled negative variants that omit one gate at a time; never product candidates.

All arms receive identical proposal traces. The evaluator holds the true invalidation schedule and expected sink call counts. Candidate-visible ticket or owner records must not contain private evaluator labels.

## 4. Frozen fault families

- platform/configuration digest changes after admission;
- ODD, operating mode, authority epoch, revocation or delegated scope changes while queued;
- M1 mission generation, subgoal, action identity, expected-effect predicate or attempt changes;
- navigation profile, frame, source, sequence, integrity or freshness falls below ticket requirements;
- source clock, boot epoch, destination time basis or correlation commitment changes;
- queue controller epoch advances while the command is pending;
- queue lease expires exactly at the exclusive deadline or just before callback;
- queue clock ID or boot epoch differs from the one bound when enqueued;
- M2 ticket commitment and independently obtained live binding commitment differ;
- same ticket ID/commitment is replayed or two submitters race the same attempt;
- the application callback returns Accepted, definitive NoEffect or UnknownAfterPossibleApplication;
- native acceptance occurs and acknowledgement is lost or delayed;
- the provider cannot supply a complete/current snapshot;
- an application path attempts to forward work into a second unfenced queue.

Each negative case has an explicit positive control. A reject-all endpoint cannot pass.

## 5. Non-compensatory metrics and targets

### Hard gates

- **Invalid native-sink calls:** target 0 across all frozen negative-control schedules.
- **Duplicate mutation count:** target 0 for exact ticket/action-attempt replay, including concurrent same-ticket submissions.
- **False acceptance:** target 0 for stale or mismatched binding, controller epoch, M2 clock/boot or queue lease.
- **Unknown-outcome retries:** target 0; UnknownAfterPossibleApplication must retain the single unresolved-mutation hold.
- **No-effect recovery bypass:** target 0 without the existing M1-qualified recovery permit and a fresh authority snapshot.
- **Positive-control acceptance:** at least 99.9% across valid, non-faulted application opportunities; publish all reasons for a lower rate.
- **Receipt lineage:** 100% of accepted native callbacks must expose the same request ID, ticket commitment, action-attempt key, controller epoch and application sequence in the receipt.

### Performance and operational value

- Report p50/p95/p99 application-gate latency and per-gate contribution: queue epoch/lease, M2 live-context validation, commitment comparison and native callback.
- Research target: p99 assurance overhead no more than 10% of the declared native control-cycle budget, with the budget and hardware fixed before runs. This is a target, not a current measurement.
- Report false holds and positive-control rejection separately from hard failures.
- Report reconciliation time, unresolved-mutation duration, operator interventions and work lost to invalidated queued commands.
- Preserve raw per-event timestamps separately for each clock domain. Never derive a cross-clock duration without an independently qualified correlation.

### Evidence volume and statistics

Use scenario schedules as the independent experimental units, not controller ticks or video frames. Pair seeds across arms, cluster by task, robot and environment, and publish counts and confidence intervals. As a planning reference, zero observed failures in about 2,995 independent Bernoulli trials gives a one-sided 95% exact upper bound of roughly 0.1%; clustered or correlated schedules reduce effective sample size. This is not a certification threshold.

Freeze source SHAs, all owner schemas, runtime versions, scenario inputs, queue clock identities, evaluator version, exclusions and analysis before confirmatory runs. Queued, cancelled, skipped, stale-head or incomplete runs are not passes.

## 6. Current external benchmarks and how to use them

### SafeVLA-Bench — safety distinct from success

The official leaderboard snapshot accessed 22 November 2026 lists 24 policies, five suites, 22,500 episodes and eight safety specifications (table updated 26 September 2026). It reports native success rate (SR), safety rate, successful-but-unsafe episodes (SBU) and worst-violation severity (VSI). In the RoboCasa-365 Atomic-Seen table, Xiaomi-Robotics-1 reports 81.9% success but 50.1% safety and 37.7% SBU. This is a clear warning against optimizing only task success.

Use the same SafeVLA rollouts as policy inputs, then instrument the action-application trace. Compare invalidation/replay/effect-handling behavior without changing the policy's sensor privilege, action stream or safety evaluator. Do not claim the adapter improves SafeVLA SR or safety leaderboard scores without a completed matched run.

References: https://safevla.org/leaderboard.html and https://arxiv.org/abs/2606.00773

### ManiGuard — specification-grounded safety and OOD

ManiGuard-Bench describes 200 locked base tasks across six contact-rich household task families and 1,000 scenarios (one ID plus four single-axis OOD perturbations per task). It uses LTLf automaton monitors over physics-grounded predicates, reports over 23,000 policy rollouts and includes evaluation on a physical Franka platform. Its project reports that 6–21% of successful rollouts violate the safety specification.

Use ManiGuard's fixed action traces and safety monitors as an external predicate layer. Inject configuration/authority/clock/queue invalidations around the same application points, while leaving the task policy and LTLf truth evaluator unchanged. ManiGuard evaluates physical/task safety; this adapter evaluates whether the exact admitted action reaches the sink under a still-current commitment. They complement each other.

References: https://nu-ideas-lab.github.io/ManiGuard/ and https://arxiv.org/abs/2608.17386

### SafeStage — lifecycle-spanning safety

SafeStage contributes 97 risk scenarios spanning initial-state hazards, execution-time hazards and final-state hazards. This is useful for testing whether authority/physical-context invalidation is checked before application, during execution and before mission completion, rather than only at dispatch time.

Reference: https://arxiv.org/abs/2609.21223

### RoboCasa365 — household task generalization

The official leaderboard was updated 16 November 2026 and reports 50 target tasks across Atomic-Seen, Composite-Seen and Composite-Unseen splits. Its current top entry, HexaShell, reports 85.6 overall, including 80.0 on Composite-Unseen. RoboCasa365 itself spans 365 everyday tasks and 2,500 diverse kitchen environments.

Do not compare the adapter to this 85.6 policy-success number. Pin one or more fixed policies, export their action traces, and assess the application-integrity overlay on the same tasks and seeds. Report policy SR and application-assurance metrics in separate columns.

References: https://robocasa.ai/leaderboard and https://proceedings.iclr.cc/paper_files/paper/2026/hash/a05003fdb1e9562ab0c0a9719ea4de10-Abstract-Conference.html

### NIST — measurement science and system-level composition

NIST's Mobility Performance of Robotic Systems programme identifies stability, adaptability, task efficiency, task completion quality and assurance as measurement targets, including dynamic obstacles, uneven floors, slippage and forced delocalization. NIST's Performance Assessment Framework for Robotic Systems composes task-driven perception, mobility, dexterity and safety measurements into system performance models.

Use NIST's approach for the measurement layer: calibrated external ground truth, documented environment, configuration records, repeatable artifacts, system-level measures and independent evaluation. The adapter's queue and controller receipts should be treated as measurement artifacts, not proof of actual physical effect.

References: https://www.nist.gov/programs-projects/mobility-performance-robotic-systems and https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems

### Dual-robot TAMP productivity comparator

A 2026 heterogeneous dual-robot SAT/geometric-validation TAMP study reports up to 19.8% simulation execution-time reduction and 20–25% on its real dual-Panda tasks. That is a relevant later productivity comparator for #6699 composition, but it is not a result for this serial, single-flight application adapter. The present adapter adds assurance gates and should first prove correctness and bounded overhead. Productivity gains must be measured after a separately qualified concurrency/closure path is integrated.

Reference: https://www.sciencedirect.com/science/article/abs/pii/S0921889026004446

## 7. Proposed path to exceed the field

1. Reproduce the four application arms with exact same action traces and evaluator truth.
2. Add evaluator-held mutation testing for every ticket-bound field and both clock domains.
3. Prove that every invalidation racing with application has exactly one recorded linearization order and the expected native-sink call count.
4. Attach the independent M1/M2 physical-effect receipt after native application; distinguish controller acceptance, physical effect and mission completion.
5. Reuse the dependency hypergraph quarantine from #6743 when an effect is unknown, and keep the global hold whenever owner completeness is unknown.
6. Only then compose with #6699 for constrained parallelism. Compare verified makespan with the serial baseline while retaining all hard assurance gates.
7. Move from software reference to simulator trace replay, HIL, then supervised physical tests under a separate domain-specific safety case.

The competitive hypothesis is **assurance-preserving work conservation**: reject every stale/unauthorised mutation, retain holds whenever effects are unresolved, and reduce only the unnecessary work blocked by incomplete serial assumptions. It is not a claim that this adapter already outperforms a task-policy leaderboard or has physical-safety qualification.

## 8. Claim ceiling

This benchmark plan can validate the deterministic composition over supplied owners and a mock/native callback. It cannot authenticate the authority provider, prove the provider's dependency inventory complete, fence bypassing processes, persist replay state across restart, prove driver/firmware behavior, establish physical outcomes, or imply HIL, physical safety, certification or regulatory approval.