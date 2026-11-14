# Maritime Network Cascade Propagation Benchmark v0.1

**Status:** research benchmark contract; all fixtures are synthetic until explicitly identified otherwise.  
**Owner:** Symtropy scenario evaluation and deterministic reference behavior.  
**Not an operational system:** no bookings, schedule writes, dispatch, navigation, payments, or real-resource reservations.

## Objective

Measure how a bounded port-call disruption propagates through connected service obligations—and whether a coordinator can reduce total network harm without hiding who absorbs the cost.

A vessel ETA or local berth-window decision is not the full unit of success. One delayed berth release may block a pilot window, extend occupancy, displace another vessel, miss a rail or feeder connection, and move inventory or workforce demand downstream. The benchmark makes those dependencies explicit and compares the resulting service outcomes under a frozen scenario.

This work composes rather than replaces:
- [MARITIME-JIT-001 #1858](https://github.com/Luminous-Dynamics/symtropy/issues/1858) for joint voyage/arrival/berth/energy-window planning.
- [MARITIME-REC-001 #1867](https://github.com/Luminous-Dynamics/symtropy/issues/1867) for a bounded contingent recovery policy inside the declared planning boundary.
- [MARITIME-NET-001 #1648](https://github.com/Luminous-Dynamics/symtropy/issues/1648) for source-pinned macro port-connectivity inputs.
- [MARITIME-TIME-001 #1650](https://github.com/Luminous-Dynamics/symtropy/issues/1650) and [TRANSPORT-RES-005 #1671](https://github.com/Luminous-Dynamics/symtropy/issues/1671) for decision-time evidence and historical-disruption replay.

This contract owns network-level propagation and its independent tests. It must not create another canonical port-call event schema, maritime simulator, resource ledger, general optimizer, or authority for real schedule changes.

## Paradigm shift

Move from **“predict a vessel’s delay, then optimize its call”** to **“represent service dependencies, replay a shock, expose the whole impact envelope, and compare bounded responses.”**

The benchmark must keep three different structures distinct:

1. **Causal/contractual dependency:** a declared prerequisite, such as service B cannot start until service A completes. This is not inferred merely because two timestamps correlate.
2. **Shared physical capacity:** several operations consume the same canonical resource over overlapping intervals, such as the same berth or pilot resource despite different aliases.
3. **Information lineage:** the source, issue time, validity period, and provenance used to justify an edge, capacity, duration, or obligation. A valid hash proves identity, not truth or permission.

The network objective is not a single vessel's punctuality. Report total service delay, missed obligations, displaced demand, resource contention, plan churn, and recovery alongside per-participant impacts. Do not use one hidden weighted score to erase those tradeoffs.

## Bounded model

Represent the frozen scenario as a versioned graph of operation instances, service obligations, shared-resource identities, and typed dependency edges. At minimum, each operation carries:
- stable operation/call identity and profile/configuration digest;
- earliest and latest feasible start/end or explicit duration interval, with units and time basis;
- predecessor / downstream obligation identifiers;
- canonical physical resource identifiers, typed capacity demand, and half-open occupancy interval;
- service class, declared deadline/cutoff, and explicit served/unserved outcome;
- evidence source, source issue/valid/ingest times, evidence tier, and the decision information cutoff.

Each dependency edge states its semantics (precedence, downstream cutoff, shared capacity, or information dependency), source/provenance, applicability envelope, and whether it is measured, contract-defined, synthetic, inferred, or unresolved. Unknown edge semantics must not silently become a precedence fact.

Start with a finite horizon and hand-checkable instances. Reject cycles in a pure precedence graph unless the fixture explicitly declares and separately tests a bounded iterative/fixed-point interpretation. When required input is absent or contradictory, report an unresolved/cut-set result instead of treating the missing dependency as “no impact.”

### Shock vocabulary

Use synthetic, parameterized shocks such as:
- berth or handling operation duration overrun;
- pilot/tug/shore-energy resource unavailable for a declared interval;
- service start delayed past a downstream cutoff;
- capacity revision arriving after a decision's information cutoff;
- alternate service becoming infeasible before its trigger;
- multiple operations affected by one common cause.

A common cause must be represented as one correlated scenario root where that is how the fixture is defined. Do not multiply independent probabilities or fabricate real-world disruption probabilities from hand-authored cases.

## Propagation and response arms

For an identical frozen manifest, compare:

- **B0 — no propagation:** evaluate only the directly shocked operation (diagnostic baseline, not a recommended policy).
- **B1 — frozen schedule replay:** preserve assignments and show every resulting conflict, delay, and missed obligation.
- **B2 — independent local repair:** repair a call or operation without accounting for downstream calls, except for hard constraints in its local scope.
- **B3 — bounded network-aware repair:** explore only declared feasible alternatives and a fixed compute/search budget; report truncation and completeness.
- **B4 — contingent recourse adapter:** where available, compose the bounded options and trigger rules from #1867 rather than duplicating its recourse engine.
- **B5 — exact oracle:** exhaustive enumeration only for tiny cases, implemented independently from production propagation, capacity checks, and objective aggregation.

Symthaea-guided candidate ranking may be evaluated as an optional matched-budget arm; it is not assumed to outperform a conventional heuristic. The renderer and Sol Atlas projection must not decide whether a dependency or schedule is physically valid.

### Causal attribution contract

Every delayed operation should expose its *binding causal drivers*, not merely its final delay total:
- a direct shock is a driver only when the shock-adjusted scheduled start is binding and increases the operation's start relative to baseline;
- a delayed predecessor is a driver only when its finish-plus-lag constraint binds the scenario start and is later than that same constraint in the baseline;
- if two or more predecessor constraints tie at the scenario start and each is later than baseline, preserve all binding drivers;
- a shared-resource overlap is a distinct conflict fact and must not be mislabeled as a precedence cause unless a separate explicit capacity/precedence rule establishes that relationship;
- unaffected operations have no shock-attribution path merely because they share a resource or appear later in the schedule.

The receipt must allow an auditor to walk from a downstream delay backward through predecessor IDs to the originating shock. Causal attribution is conditional on the declared dependency graph and fixed-schedule replay; it is not a claim that the graph itself captures all real-world causes.

The first implementation should be an offline deterministic reference model, not a live integration. A result is a proposed counterfactual, not an instruction or reservation.

## Metrics

Report a vector and per-scenario detail, not only an aggregate:

- direct, second-order, and unresolved affected obligations;
- total downstream delay-minutes and delay-amplification ratio (only where the direct shock duration is nonzero and its denominator is declared);
- missed connections/cutoffs and unserved demand by service class;
- resource conflicts, over-capacity intervals, occupancy changes, and reserve consumed;
- affected versus displaced operations, named by participant/service class using synthetic identities;
- time to return above an explicitly declared service threshold, with recovery definition fixed before evaluation;
- changed assignments / plan churn / repeated re-plans;
- fuel, energy, and emissions only when the fixture supplies a qualified, complete activity-factor boundary; otherwise mark them not evaluated;
- completeness, candidate/branch counts, truncation, runtime, and peak memory.

For uncertainty, report the bounded outcomes per scenario and worst case unless scenario probabilities are justified by calibrated evidence. Do not call the count of synthetic scenarios a probability distribution.

## Independent correctness and negative controls

Freeze a small exact corpus before adding heuristic search. Include at least:
1. a linear chain where a single delay propagates to every dependent operation;
2. two branches where only one inherits the shock;
3. a shared resource represented by different aliases but the same physical identifier;
4. a delay absorbed by slack before the next operation;
5. a missed cutoff with an explicit unserved outcome;
6. two simultaneous shocks with one common cause;
7. two shocks competing for one bounded recovery option;
8. stale, conflicting, post-cutoff, and missing evidence;
9. dropped, duplicated, reordered, and replayed shock events;
10. a cycle, dangling predecessor, duplicate operation ID, and inconsistent time basis;
11. a plan that improves one participant's punctuality by displacing another, so distributional cost is visible;
12. a graph whose search budget is exhausted, which must not be labeled globally optimal.

The exact oracle may share data types only where needed; it must not call the production dependency traversal, resource-identity normalization, feasibility checks, or score reducer. Every negative fixture must state its expected disposition and why.

## Proposed qualification gates and performance targets

These are **targets to test, not measured results**:

- **100%** agreement with independent exhaustive enumeration on the frozen tiny corpus: affected set, feasible times, conflicts, served/unserved obligations, and metric vector.
- **100%** correct causal-driver attribution on frozen chains and fan-in ties: no missing binding cause, no non-binding shock, and no resource overlap misreported as precedence causality.
- **Zero** false-negative propagated dependencies and **zero** phantom impacts across the independent planted positive/negative graph controls.
- **Zero** admitted capacity overcommitments for every canonical physical resource, including alias/revision cases.
- **100/100** deterministic replay and input-order permutations produce byte-identical canonical receipts for an identical manifest.
- **100%** of unknown, contradictory, invalid-time, stale-at-decision, or incomplete-search cases carry a typed unresolved/abstain/truncated disposition; none silently become a clean feasible plan.
- **Scale target:** a pinned Rust runner should propagate one shock through 10,000 operation nodes and 100,000 typed edges at p95 below 500 ms over 1,000 repeats, excluding file I/O, optimizer search, and rendering. Record CPU, OS, compiler/toolchain, fixture digest, and measurement method; revise the target only with baseline evidence, never by weakening correctness gates.
- Compare B1–B4 on the same shock corpus and report p50/p95 runtime, p95/worst service-delay outcomes, plan churn, displaced demand, and completeness. Do not claim superiority unless comparisons use matched scenarios and budgets.

After synthetic qualification, a separate gate may admit historical or operational data only with explicit rights, source revisions, completeness analysis, decision-time cutoffs, and independent reviewer sign-off. A paper's aggregate result or narrative incident is not a row-level replay dataset.

## System boundaries and claim ceiling

- **Symthaea:** physical/service envelope validity, engineering applicability, and calibrated model evidence.
- **Symtropy:** deterministic graph propagation, counterfactual replay, bounded comparison, and exact tiny-instance oracle.
- **Mycelix:** participant-scoped authorization, binding commitments, reservations, custody, and any owner-authorized effect. This benchmark cannot commit on its behalf.
- **Sol Atlas:** read-only spatial/temporal projection, source coverage, uncertainty, and lineage; visualization is not evidence authority.

Research anchors:
- DCSA Port Call Standard 2.0 describes standardized exchange of estimated, requested, planned, and actual operational events and move forecasts: https://dcsa.org/newsroom/port-call-standard-update and https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-purpose-and-scope
- The IAPH–IHMA Port Call Optimization Guide was announced in March 2026 to harmonize nautical and operational port-call data exchange: https://www.iaphworldports.org/pickup/21900/ and https://greenvoyage2050.imo.org/port-call-optimization-guide-launched-at-imo/
- The Global Maritime Forum's 2025 port-call optimization brief covers JIT arrival, virtual arrival, waiting time, and value-chain coordination: https://globalmaritimeforum.org/report/port-call-optimisation-a-pathway-to-reducing-waiting-times-and-emissions/
- The Global Maritime Forum's 7 October 2026 Cargill pilot report covers 16 voyages across three ports and three coordination mechanisms. It reports context-specific findings including estimated 6–11% ballast-leg emissions reductions at Newcastle, close to 10% for a ship already sailing at very low speed in Albany, and potential ~222 tonnes CO₂ savings over ten voyages under a bespoke notice-of-readiness scheme in Santarém: https://globalmaritimeforum.org/insight/turning-waiting-time-into-savings-lessons-from-cargills-port-call-optimisation-pilot/. These are case-specific reported outcomes, not transferable model parameters or target values; the report stresses that savings depend heavily on waiting time available and vessel operating envelopes.

These sources motivate the problem and data boundary; they do not validate this implementation. Passing synthetic checks demonstrates only correctness within the declared graph model, not actual port recovery performance, safety, emissions savings, fairness, regulatory compliance, or authority to act.
