# Symthaea–Symtropy Controller Outcome Lifecycle Ledger v0.1

**Status:** research/reference implementation with authored unit tests. Exact-head compilation and execution remain unverified until the matching CI run passes. The ledger is in-memory and is not a physical execution attestation.

## Problem

The control contract can represent controller feedback, including timestamps, ticks, constraint causes, and explicit terminal statuses. A local successor validator alone does not maintain per-request history. Without a stateful owner, a caller can forget to invoke it, accept an outcome for an unsubmitted request, associate an event with the wrong controller ticket, or lose terminal-state knowledge.

The new ControlOutcomeLedger binds every accepted DispatchReceipt::Submitted to its request ID, authenticated session/sequence, and nonempty controller ticket. Request IDs and controller tickets must each be unique within the ledger. An event is accepted only when its request ID and ticket match, its payload validates, and it advances the prior lifecycle. Ticks must strictly increase. Once Vetoed, Completed, Failed, Cancelled, or Expired is accepted, no further event can reopen the lifecycle.

## Paradigm shift: measure closed-loop execution, not just dispatch

The architecture now has three separately testable contracts:

1. **Admission:** should this semantic request be submitted at all?
2. **Execution lifecycle:** what did the native controller report for the submitted ticket?
3. **Cognitive feedback:** what does the accepted outcome imply about the world model, task feasibility, confidence, and next action?

This module implements the stateful part of step 2. It does not implement the cognitive update or certify the authenticity of the controller event source. The connector must receive the actual ticket from the trusted native-controller event source; fabricating a ticket from request ID defeats the binding.

## Failure behavior

| Event | Expected behavior |
|---|---|
| Dispatch receipt is rejected or controller ticket absent | Cannot register |
| Zero session/request/sequence or empty ticket | Cannot register |
| Request ID is already registered | Reject duplicate ID |
| Controller ticket is already bound to another request | Reject duplicate ticket binding |
| Controller outcome references unknown request | Reject |
| Ticket does not match registered submission | Reject without state change |
| Malformed outcome (for example, Vetoed without a cause) | Reject without state change |
| Duplicate or non-advancing tick | Reject without state change |
| Observed time moves backwards | Reject without state change |
| Status transition is illegal | Reject without state change |
| Safety authority vetoes a currently executing request | Accept as terminal Vetoed with machine-readable cause |
| Outcome arrives after terminal state | Reject, even with a larger tick |
| Capacity is exhausted | Refuse new registrations; preserve existing terminal tombstones |

No records are automatically evicted. This deliberately trades availability for fail-closed behavior at the configured capacity. The prototype does not solve durable replay prevention across process restarts; production integration needs a persistent ledger or equivalent durable authority, controller identity/rekey rules, and authenticated event provenance.

## Unit-test coverage authored

The tests cover positive-capacity construction, registration preconditions, Accepted → Executing → Completed, Executing → Vetoed terminal transition, late post-terminal updates, unknown request IDs, ticket mismatch, duplicate ticks, backward timestamps, malformed constrained/vetoed outcomes, capacity exhaustion with retained terminal state, and immediate terminal failure. A fixed-seed 256-request adversarial corpus additionally interleaves six invalid-event classes, asserts each rejection preserves prior state, then checks valid completion and post-terminal immutability for every request.

The primary assertion is not simply that an error was returned. Rejected events must leave the last accepted outcome unchanged.

## Proposed benchmark targets (not results)

- 100% of deterministic lifecycle fixtures meet expected acceptance/rejection outcomes, including all six invalid-event classes in a fixed-seed 256-request corpus.
- 0 mismatched-ticket, unknown-request, duplicate-tick, backward-time, illegal-transition, or post-terminal events are accepted in the registered conformance corpus.
- 0 ledger mutations from rejected events.
- 100% of registered submitted requests retain their request/session/sequence/ticket provenance until external durable retention policy permits retirement.
- Report ingest latency at p50/p95/p99/max under a registered event-rate/load profile; initial target <100 μs for in-memory validation on a named host, not including transport or durable persistence.
- Report memory per tracked request and capacity exhaustion behavior. Do not silently evict tombstones to keep accepting commands.

Zero observed rejects accepted in a finite test corpus is not proof of zero lifetime risk. Property-based event sequences, fuzzed serialized inputs, restart/recovery tests, and real-controller integration are follow-on requirements.

## Industry comparison and route to stronger evidence

### Where current benchmarks sit

- **[Isaac Lab-Arena](https://developer.nvidia.com/isaac/lab-arena)** supports large-scale policy evaluation across thousands of GPU-accelerated simulation environments. Symtropy should aim to make its conformance suite cheap enough to run on every change, then separately measure throughput and reproducibility at scale.
- **[RoboDojo](https://github.com/RoboDojo-Benchmark/RoboDojo)** combines 42 simulation tasks and 18 real-world tasks across three embodiments, testing generalization, memory, precision, long-horizon behavior, and open-ended instructions. Its [simulation leaderboard](https://robodojo-benchmark.com/leaderboard?bench=sim) was updated on October 26, 2026 and reports both a process/quality score and success rate across policy submissions. This makes a good model for reporting multiple dimensions, not one average alone.
- The **[Stanford AI Index 2026 robotics review](https://hai.stanford.edu/assets/files/ai_index_report_2026.pdf)** reports 89.4% average success for EquAct on the 18-task RLBench subset as of January 2026, while warning that these are relatively short-horizon controlled simulation tasks. This is a manipulation result, not an appropriate threshold for a controller-outcome ledger or mobile-base adapter.
- **[ROS 2 Control](https://control.ros.org/master/doc/ros2_control/controller_manager/doc/userdoc.html)** documents controller-manager lifecycle and real-time control-loop concerns. The outcome ledger should remain supervisory/control-plane infrastructure; it must not introduce unpredictable work into the high-frequency actuator loop.

These are different workload families. Do not compare the lifecycle ledger's invalid-event rejection rate to manipulation success-rate leaderboards, or claim that a generic robot architecture beats SOTA because its interface spans multiple embodiments.

### A defensible comparative program

1. **Protocol correctness:** register only valid submissions; reject unknown/mismatched tickets, replayed ticks, illegal transitions, and post-terminal events. Keep deterministic fixtures plus the fixed-seed adversarial corpus as a continuously run regression suite.
2. **Closed-loop task performance:** use one platform-native controller and task suite with four matched variants: native controller only; Symthaea intent selection; intent plus runtime assurance; intent plus assurance and outcome-feedback replanning. Use identical starts, goals, disturbances and seeds.
3. **Fault recovery:** predeclare sensor dropout, stale observation, controller rejection, communication loss, obstruction and unexpected contact scenarios. Report recovery success, time to safe state, false-veto rate, task success and hard constraint violations. Never remove failed/missing episodes from the primary denominator.
4. **Systems cost:** report ledger/admission overhead p50/p95/p99/max, missed deadlines, memory per outstanding request, saturation behavior and restart recovery on a named host and load profile.
5. **Transfer evidence:** report simulation, hardware-in-loop (HIL), and physical-robot data as separate evidence tiers. On physical systems, preserve the hardware-native emergency stop and watchdogs.

The proposed <100 μs p99 target applies only to the in-memory ledger operation on a named and reproducible host profile; it excludes transport, persistence, perception and controller time. It is an engineering target, not an industry standard. First establish the baseline distribution, then determine whether a target can be met without changing correctness.

Record exact SHAs, fixture digests, test commands, registered task manifests, and raw event traces. The ledger is only a credible step toward an auditable cognitive loop if authenticated real-controller outcomes reach Symthaea and demonstrably improve replanning decisions without increasing hard-boundary violations.

## Claim ceiling

This PR adds a bounded in-memory lifecycle tracker and conformance tests. It does not authenticate the controller event stream, persist state across crashes, detect physical motion independently, implement a policy-learning loop, guarantee real-time behavior, or establish robot safety or SOTA gains.
