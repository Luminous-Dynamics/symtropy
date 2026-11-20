# Marine Recovery Reachability: 2026 State of Practice and Benchmark Contract

**Date:** 20 November 2026  
**Implementation:** Symtropy #6298, branch `research/marine-recovery-reachability-20261120`

## Executive distinction

A route can be feasible and still not be recoverable. A one-step action shield can reject a bad immediate action and select a safe fallback, but that does not establish that the fallback leads to an evidenced safe terminal before intervention time expires. This work therefore evaluates bounded **post-trigger continuation graphs**.

This is not a new route optimizer, a mathematical viability-kernel proof or a navigation controller. It is an independent verifier of a proposed finite action graph under a specific vessel/crew/ODD contract, trigger state and evidence set.

## 1. 2026 comparison set

### 1.1 Mission autonomy and runtime assurance

The 17 November 2026 review *Onboard Decision-Making for AUV Autonomy: Review and Research Gaps* finds no universal architecture winner: FSM, behavior-based, deliberative and hybrid designs suit different mission profiles. It identifies mission-level risk management, runtime assurance under embedded power/compute limits, integrated mission reasoning, sim-to-real transfer and explainability as open implementation areas. It also highlights a lack of comparative benchmarks under shared mission conditions and sparse reporting of availability and maintenance effort. DOI: https://doi.org/10.3390/jmse14222179

**What this means for our comparison:** do not market a single universal autonomy architecture. Compare architectures under the same fault, resource, communications, authority and compute budgets. Record maintenance, availability and evidence-production burden alongside path success.

### 1.2 Integrated multi-energy voyage planning

The September 2026 rolling-horizon green multi-energy ship microgrid study jointly coordinates electricity, thermal energy, hydrogen and freshwater; adds diesel-generator waste-heat recovery; and optimizes route selection and flexible speed scheduling with on-board dispatch. Its case studies report an 8.44% reduction in total operating cost and 0.109 seconds per horizon for its stated setup. DOI: https://doi.org/10.1080/20464177.2026.2728778

**What this means for our comparison:** energy-aware routing is already beyond a static fuel budget. The recovery benchmark should compare against rolling-horizon multi-carrier dispatch and keep the result separate from physical performance claims. Our additional research question is whether a post-fault continuation remains feasible across resource/service floors, qualified capabilities, destination evidence and intervention deadlines. Do not claim superiority from a synthetic graph.

### 1.3 Formal graph planning and timed verification

The 2026 paper *Safety-assured decision support for ASV navigation via hybrid graph planning and timed automata verification* combines Voronoi global planning, visibility-graph refinement and Linearly Priced Timed Automata checked with UPPAAL for safety, reachability and bounded time-energy feasibility in Singapore Strait scenarios. DOI: https://doi.org/10.1016/j.eswa.2026.131367

**What this means for our comparison:** our finite graph enumeration should be benchmarked against formal timed-automata checking on matched small cases. The formal baseline may have stronger model-checking semantics; our intended differentiator is the evidence- and authority-qualified multi-resource terminal contract, not a claim that DFS enumeration replaces formal verification.

### 1.4 Field validation of temporal mission planning

The 30 June 2026 Ocean Engineering study *Field experimental validation of temporal AI-based mission planning for a maritime autonomous surface ship* integrates a temporal planning system with a ROS-based guidance/navigation/control stack and reports simulations plus sheltered-basin sea trials. DOI: https://doi.org/10.1016/j.oceaneng.2026.125858

**What this means for our comparison:** a pure simulation benchmark is insufficient. The progression must include software-in-the-loop, processor-in-the-loop, hardware-in-the-loop and an independently reviewed sheltered-water programme before any operational claim.

### 1.5 Human remote operation

The 19 August 2026 paper *Human Factors Contributing to Remote Operation and Supervision of Autonomous Vessel Operations* identifies the sensory chasm, vigilance decrement, situation-awareness breakdown, mode confusion and delayed re-entry when the remote operator is out of the loop. It emphasizes explicit re-orientation, interface transparency and the increased risk when the operator re-enters under time pressure. DOI: https://doi.org/10.3233/NICSP260006

**What this means for our comparison:** a human-handover path is not viable merely because a communications message was queued. It needs named authority, acknowledgement, qualified communications, a response-time upper bound and a positive remaining intervention margin. Human acceptance rate, re-orientation time, response-time distribution and failed handovers should be measured separately.

## 2. Paradigm shifts to test

1. **From route continuity to recoverability at every trigger.** Evaluate all bounded, admissible continuations after each registered fault or mission-degrading event; do not assume the nominal path remains the only option.
2. **From capability labels to capability evidence.** A capability must be qualified for the exact craft/ODD scope and valid at decision time; then it must also be available in the current trigger state. A registered repair qualification cannot make a missing spare or failed component available.
3. **From “contact shore” to timed human-in-the-loop viability.** Acknowledgement and upper-bounded response time must fit inside the intervention budget. If not, the remote operator is not an admissible recovery terminal.
4. **From path-found to search-complete.** Bound nodes/edges/actions/depth and disclose truncation. SEARCH_INCOMPLETE is not INFEASIBLE and must never be reported as exhaustive.
5. **From shortest route to a set of recoverable options.** Preserve all feasible paths and rejected edges, per-axis worst margins, resource/crew costs, time envelopes and terminal identity. Do not compress rescue options into one weighted score.
6. **From planner self-assurance to independent evidence qualification.** The planner proposes graph edges; the recovery verifier independently checks state, preconditions, action evidence, hard gates, inter-sample resource envelopes, service terminal and authority receipts.
7. **From one benchmark score to shared-condition evidence.** Freeze mission, initial resources, occupancy, craft revision, trigger, available components/capabilities, service contract, hard-gate roster, environment, compute/hardware and human response assumptions across all policy arms.

## 3. Matched baseline arms

Every test case should evaluate these alternatives against the same trigger-state digest and contract:

- **B0 — Precomputed diversion:** fixed predeclared safe-haven policy, no graph replanning.
- **B1 — Shortest-time graph search:** minimize worst-case arrival time among graph paths, while retaining the same hard constraints.
- **B2 — Resource-aware bounded search:** explore paths using per-axis robust margins and worst-case duration; no weighted scalar utility.
- **B3 — Formal timed-automata verification:** encode the same finite graph/time constraints for UPPAAL or an equivalent model checker on cases within tractable state size.
- **B4 — Rolling-horizon multi-energy dispatch + route replan:** use the same resource and route inputs; record its computed continuation and any unhandled failure triggers.
- **B5 — Human handover / controlled safe hold:** separate remote authority acknowledgement and communication latency from physical refuge/resupply options.

This is a comparison protocol, not an assertion that each external baseline is already implemented in this branch.

## 4. Benchmark scorecard

| Dimension | Target for the preregistered synthetic corpus | Required reporting |
|---|---:|---|
| Trigger coverage | 100% exactly one scenario per registered trigger | Missing/duplicate trigger counts |
| Capability integrity | 100% action capability IDs resolve to current qualification records | Unknown, expired, suspended, ODD-mismatch counts |
| Action preconditions | 100% exact equipment, capability, trigger and gate checks | Rejected edge + reason |
| Resource feasibility | Zero accepted path with any inter-sample floor breach | Worst margin by axis and action |
| Terminal evidence | 100% accepted terminals with verified access/service/authority/evidence | Missing/unverified/expired reason |
| Human handover | 100% accepted only after acknowledged, time-feasible handover | Ack rate, p50/p95/p99 response, remaining margin |
| Search completeness | 100% exact graph enumeration below configured bounds | Expanded edges, depth, path count and truncation |
| Known-answer agreement | 100% against independent hand-calculated and model-checker cases | False-recoverable and false-abstain separately |
| Decision latency | p50/p95/p99 and worst-case on named target hardware | Graph size, edges expanded, resources, horizon, hardware, thermal/power profile |
| Operational readiness | No claims until SIL → PIL → HIL/PHIL → independent sheltered-water trials | Fault-injection coverage and measured evidence maturity |

**Do not target zero abstention.** In an assurance system, abstention is correct when the graph is infeasible, a terminal is unverified, evidence is stale, or the search is incomplete. Optimize zero false-recoverable declarations while controlling unnecessary false abstentions.

## 5. Current synthetic demonstration

The committed graph contains five fault triggers:
- generator trip;
- watermaker trip;
- common-cause generator + watermaker trip;
- resource margin compressed / intervention deadline exhausted;
- unverified terminal service.

The generator-trip case now has two distinct admissible continuations:
1. load-shed then divert to the verified safe haven;
2. transfer to an acknowledged remote master with a 0.5-hour response upper bound.

The same scenario retains rejected alternatives. The fast diversion breaches the energy floor, and generator repair depends on unavailable equipment/capability. The human handover path reports the remaining intervention-time margin rather than merely the time to send a message.

All values, capability records, action deltas, authority receipts and timing assumptions are synthetic. They prove only that the software contract encodes these distinctions; they are not measured vessel outcomes.

## 6. Next scientific increment

The finite graph is deliberately bounded. The next step after this auditor is not a larger arbitrary DFS. It is:
1. build an independent timed-automata encoding for the same fixtures;
2. differential-test path feasibility, time bounds, terminal status and rejected action reasons;
3. add partial observability/communications loss and remote-operator workload/re-orientation latency as explicit scenarios;
4. compare a robust search to a Pareto label-setting algorithm on the same graph;
5. add measured high-rate resource/repair/communications data before calibrating rate bounds or human-response intervals.

Only after those steps should the project consider a formal reachable viability set. A finite graph audit is not proof of continuous dynamics, and a synthetic human acknowledgement is not a real remote-control protocol.

## 7. Claim ceiling

This tool audits declared bounded action graphs and evidence receipts. It does not plan or execute routes, command a helm, certify a refuge, prove collision avoidance, validate COLREG compliance, prove continuous-system viability, or guarantee a human operator will respond. Physical authority remains with qualified vessel systems and the responsible crew/operator.
