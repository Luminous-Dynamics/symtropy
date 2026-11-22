# Mobility Capability Commitment and Industry Benchmark v0.1

**Programme:** [MOBILITY-ENERGY-RESILIENCE-001 #6707](https://github.com/Luminous-Dynamics/symtropy/issues/6707)  
**Executable child:** [MOBILITY-COMMITMENT-LEASE-001 #6827](https://github.com/Luminous-Dynamics/symtropy/issues/6827)  
**Implementation branch:** \`research/mobility-commitment-lease-v0-1\`  
**Research cut-off:** 22 November 2026  
**Status:** source-level implementation, strict schema, synthetic fixture, CLI and 13 Rust test definitions committed; exact-head CI pending.

## 1. The architectural gap

The current assurance chain has three verified or separately scoped layers:

1. **Offer-window feasibility — #6802 / PR #6803.** A supplied per-resource dispatch allocation must fit each resource's mission-adjusted energy and slot-power envelope.
2. **Chronological recovery — #6808 / PR #6809.** The same resources must restore their reserve before their next required use and survive the admitted repeated-shock scenarios.
3. **Cross-domain promise admission — #6827 / this implementation.** Once capability is offered or accepted, the same kW/Wh cannot be promised again to another market product, aggregator, mission planner or service contract over the same time/resource window.

A feasible schedule is not a unique commitment. Two independently feasible optimizers can both promise the same battery to different counterparties. If the system checks each offer in isolation, it can double-sell capacity without any individual optimizer returning an infeasible answer.

The new primitive is a **resource-scoped capability lease**. It binds an exact resource/configuration and time window to a finite kW/Wh allocation, an idempotency key, an expiry, and evidence references to the flexibility and recovery snapshots.

## 2. Paradigm shift: treat flexibility as a single-use, time-bounded capability

The system now needs to distinguish:
- physical capability: what the resource could supply;
- protected obligation: what mission, inventory, safety and recovery already require;
- offered capability: what a counterparty has been told is available;
- accepted capability: what has become a firm promise;
- executing capability: what is being delivered;
- unresolved capability: what may have been delivered but whose outcome is not yet known;
- released capability: what has been rejected, cancelled, expired or settled with valid terminal evidence.

The crucial invariant is:

\[
\sum_{c \in C_{r,t}^{active}} P_{c,r,t}
\leq
P^{safe}_{r,t}
\]

and independently for each resource over the lease horizon:

\[
\sum_{c \in C_r^{active}} \sum_t E_{c,r,t}
\leq
E^{safe}_r
\]

No sum over resources may be used to cure one resource's deficit. A site's aggregate kW is not enough.

The lease state machine is intentionally conservative:

\`FirmOffered -> Accepted -> Executing -> Settled\`

Alternative terminal paths include \`Rejected\`, \`Expired\` and \`Cancelled\`. A timeout transitions to \`Unresolved\`; it remains reserved until an evidence-backed reconciliation changes its state. Expiry is not an automatic release action. Back-to-back windows use half-open intervals \`[start_slot, end_slot)\`.

This is a pure admission verifier. It returns a report but never writes a lease, calls a market API, commands a charger, or authorizes grid dispatch.

## 3. Executable v0.1

Files:
- \`tools/ev-passenger-cost-frontier/src/mobility_commitment_lease.rs\`
- \`tools/ev-passenger-cost-frontier/src/bin/mobility_commitment_lease_screen.rs\`
- \`tools/ev-passenger-cost-frontier/schemas/mobility_commitment_lease_v0_1.schema.json\`
- \`tools/ev-passenger-cost-frontier/fixtures/mobility_commitment_lease_v0_1.json\`

Run:

\`\`\`bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin mobility_commitment_lease_screen -- \\
  tools/ev-passenger-cost-frontier/fixtures/mobility_commitment_lease_v0_1.json
\`\`\`

The evaluator checks:
- flexibility and recovery digest syntax plus matching report evidence references;
- per-resource safe grid energy and per-slot power budgets;
- exact resource and site configuration binding;
- allocation vectors covering the complete horizon, with zero allocation outside their half-open commitment window;
- unique commitment IDs, idempotency keys and resource allocation entries;
- unique state-receipt IDs with exact evidence-reference binding, and terminal release backed by measured or independent-test evidence;
- reservation of firm-offered, accepted, executing and unresolved claims;
- protected mission, recovery, maintenance and safety-hold intervals;
- candidate offer expiry before the service window starts and before source evidence expiry;
- deterministic output ordering and checked arithmetic.

### Frozen synthetic known answer

The fixture contains two resources, three existing commitments and one candidate lease:
- two live claims reserve capability, one in \`Accepted\` state and one in \`Unresolved\` state;
- one \`Settled\` claim is released only with independent-test terminal evidence;
- ESS-02 has a future mission-protected interval;
- the new ESS-01 lease begins exactly at the end of an earlier lease, so the half-open interval rule allows it.

| Resource | Safe energy | Active + candidate energy | Remaining |
| --- | ---: | ---: | ---: |
| ESS-01 | 1,500 Wh | 1,100 Wh | 400 Wh |
| ESS-02 | 900 Wh | 300 Wh | 600 Wh |

ESS-01 per-slot committed power is [500, 500, 2,500, 1,500, 0, 0] W against 3,000 W each slot. ESS-02 is [0, 500, 500, 0, 0, 0] W against 2,000 W.

The fixture uses synthetic data. These values validate arithmetic and reservation semantics only. No market, dispatch, ledger-mutation or physical authority is granted.

The module contains 16 focused Rust test definitions, including a 12-case independent resource-capacity reference. These are source-level test definitions until an exact-head Actions run completes.

## 4. Current research and industry comparisons

### 4.1 Hierarchical swapping and grid scheduling

Chen et al. (3 June 2026) propose a BSS-cluster method with day-ahead declaration, 15-minute rolling optimization and station recommendations every 10 seconds. Their five-station Beijing case reports a 100% declared demand-response tracking rate, approximately 53% lower total cost in the comparison that excludes DR shortfall, an approximately 25% queue-cost reduction, 2.36-second average cluster-scheduling time and sub-5-ms station recommendations. They also identify battery degradation and imperfect communication as future work.

This is a useful operational baseline. It demonstrates that demand forecasting, rolling resource scheduling and real-time station guidance can be coordinated. The Luminous addition is a separate resource-level admission boundary so that two firm promises cannot both spend the same per-resource flexibility budget, including after a timeout. Reproduce the paper's source scenario and denominators before any numerical comparison.  
Source: [Chen et al., Electronics 2026, 15(11), 2444](https://doi.org/10.3390/electronics15112444).

### 4.2 Distributionally robust and learned scheduling

Du & Zhu (16 November 2026) formulate heavy-truck BSS scheduling as a KL-ambiguity-set robust MDP, using an offline VAE transition estimate and DR-SAC charge-count actions. They explicitly introduce a battery-scarcity index and report constraint violations in high-demand evaluation. Their paper-specific results show that peak shaving and reward do not by themselves guarantee reserve safety.

The comparison for Luminous should report:
- peak shaving, total cost and queue waiting;
- hard mission/inventory reserve violations;
- double-commitment rejection;
- unsafe proposals rejected by the reserve shield;
- revenue/utility lost due to the shield;
- performance when a lease is unresolved and capacity remains locked.

A fair tournament runs DR-SAC and other policies as proposers behind the same fixed verifier. Never allow a learned policy to change the verifier's thresholds or evidence requirements.  
Source: [Du & Zhu, Applied Sciences 2026, 16(22), 11346](https://doi.org/10.3390/app162211346).

### 4.3 Chance-constrained and bilevel reserve provision

Koufalexis et al. (15 November 2026) propose a stochastic chance-constrained bilevel EV charging and reserve model. The study models uncertain renewables and demand, time-varying EV fleet availability and degradation costs, and evaluates 24-hour scheduling on three test systems with 3–80 distributed generators and 260–1,100 EVs. Its reported operating-cost reductions vary from 1.7% to 7.7% across the examined confidence levels.

This is a strong economic/uncertainty baseline. Our distinction should be measurable: the lease gate is deterministic and resource-specific *after* the optimizer produces a candidate. It does not replace probabilistic planning; it stops capacity from being committed twice or released on an unknown outcome.  
Source: [Koufalexis et al., Electronics 2026, 15(22), 5312](https://doi.org/10.3390/electronics15225312).

### 4.4 Mobile replenishment networks

A September 2026 study models mobile battery-swapping vans as a closed-loop system: vans deliver charged packs, carry out swaps, return to charging stations and reload. It jointly sizes and allocates vans, racks and batteries under uncertain demand, service and charging.

This is a relevant comparator for mobile carriers and persistent service. The Luminous benchmark should add identity-bound asset leases across the mobile/central charging boundary: a pack in transit, reserved for a mission, waiting in a rack or offered as grid flexibility cannot be simultaneously promised elsewhere.  
Source: [Integrated optimization of mobile battery swapping and charging systems under uncertainty, Transportation Research Part D 158 (2026), 105458](https://doi.org/10.1016/j.trd.2026.105458).

### 4.5 Commercial autonomous-drone dock

DJI publishes the following Dock 4 specifications:
- battery swap below 25 seconds;
- landing-to-next-takeoff turnaround below 45 seconds;
- at most three stored batteries;
- two parallel charging channels;
- 27-minute charge from 15% to 95% at 25 °C.

The swap and turnaround numbers are measured in a controlled test environment and described by the manufacturer as reference figures. The charge time is a separate measure. These are not fleet-wide availability, lifecycle-cost or grid-flexibility measurements.

The correct competition is not merely to beat 25 seconds. It is to maximize successful on-time missions per total fleet/infrastructure cost, while preserving reserve and proving recovery.  
Source: [DJI Dock 4 manufacturer specifications](https://enterprise.dji.com/dock-4/specs).

### 4.6 Mid-air replenishment

The July 2026 IEEE Robotics and Automation Letters paper on autonomous mid-air battery exchange reports simulations and outdoor tests in moderate wind, with a supplier aircraft transferring a battery to a receiver aircraft. This is a useful alternative to fixed docking, mobile-carrier swap and fleet rotation.

The benchmark should compare **net additional mission-hours** after accounting for donor energy, rendezvous, transfer failure, payload penalty, reserve, weather and additional aircraft-hours—not only transfer time. This paper is a research prototype, not an operationally qualified aerial logistics service.  
Source: [IEEE RA-L, DOI 10.1109/LRA.2026.3693549](https://doi.org/10.1109/LRA.2026.3693549).

## 5. Grid and safety standards: keep the boundaries precise

- [IEC 62933-3-1:2025](https://webstore.iec.ch/en/publication/75427) covers planning and performance assessment of electrical energy storage systems, including monitoring, system information and operational performance.
- [IEC 62933-5-4:2026](https://webstore.iec.ch/en/publication/67442) describes safety test methods/procedures for grid-integrated lithium-ion EES.
- [NRS 097-2-1:2024](https://www.sseg.org.za/wp-content/uploads/2024/02/NRS-097-2-1-Published-2024.pdf) focuses on the AC utility-interface side of small-scale embedded generation. Its scope explicitly excludes specifications for the customer-side energy source or storage system; South African grid codes for BESS/RPP and utility requirements remain relevant.

These standards inform stationary ESS/site integration and test plans. They are not evidence that an aircraft propulsion pack, BMS, charger, inverter or carrier is qualified for bidirectional grid operation. Keep site protection, metering, connection approval and market eligibility separate from the mobility-resource commitment ledger.

## 6. Paradigm shifts and research programme

### A. Proof-carrying capability leases

A future firm offer should carry:
- exact resource serial/configuration and site connection;
- per-slot kW and horizon Wh allocation;
- mission/recovery reserve snapshot and report digests;
- expiry and delivery window;
- unique idempotency key and commitment ID;
- evidence-backed state transitions;
- release proof after settlement/cancellation/reconciliation.

This becomes a reconstructable receipt, not a promise that depends on an aggregator's mutable internal state.

### B. Unknown outcomes consume capacity

A charging timeout, network partition, lost settlement message or uncertain dispatch acknowledgement is not evidence that the resource is free. Unresolved claims stay reserved. This may strand capacity temporarily, so publish both the safety benefit and its economic opportunity cost. The remedy is faster independent reconciliation, not optimistic release.

### C. Separate planning, promise and actuation

Optimizers propose. The lease gate checks uniqueness and remaining capability. Independent dispatch authorities and physical protection decide whether a real action can proceed. The ledger records evidence-backed outcomes; it does not command or self-certify them.

### D. Reserve a recovery budget before bidding

A grid offer's usable budget should be the intersection of:
- per-resource offer-window flexibility from #6802;
- chronological recovery and next-mission viability from #6808;
- existing active commitments and protected intervals from #6827.

The available quantity is computed per resource and per slot. A resource cannot be simultaneously counted as mission reserve, post-event recovery, grid capacity and another market's firm offer.

### E. Compare system-level outcomes

The eventual benchmark objective should be:

\[
\frac{\text{Successful on-time mission-hours}}
{\text{Total lifecycle cost of aircraft, batteries, chargers, carriers, site and operations}}
\]

with grid-service revenue shown separately and only when the service is actually deliverable, measured and authorized.

## 7. Benchmark tournament and targets

Use identical initial states, forecasts, scenarios, resource descriptors, deadlines, compute budgets and hardware. Reproduce each paper's native benchmark first; then evaluate all methods on a new, matched UAV/carrier dataset.

| Family | Baseline methods | Primary metrics |
| --- | --- | --- |
| Scheduling | fixed charging, EDF, greedy tariff, MPC | mission service, p95/p99 queue, stockouts |
| Optimization | MILP, two-stage robust/C&CG, chance-constrained, Wasserstein DRO | cost, optimality gap, reserve shortfall |
| Learning | DR-SAC, hierarchical RL, decision-focused prediction | regret, violations, abstentions, shield loss |
| Cyber resilience | stale/replayed data, false-data injection, delays, partitions | false acceptance, unresolved lock duration, recovery |
| Commitment integrity | independent offers, overlapping counterparties, cancellation/settlement races | duplicate acceptance, overcommitment, time-to-release |
| Physical service | fixed dock, mobile carrier, fleet rotation, mid-air replenishment | successful mission-hours, donor cost, recovery latency |

Targets (not achievements):
- zero false acceptance across frozen overcommitment, stale/replay, configuration-mismatch and unresolved-timeout cases;
- 100% agreement with an independent enumerator for the bounded resource/slot family;
- 100% deterministic replay/canonical output under input permutations;
- zero mission-protected-window encroachment;
- candidate lease admission only when every resource's cumulative Wh and every slot's kW fit;
- target p99 below 10 ms for the bounded evaluator on a declared reference runner;
- publish the fraction of capacity locked by unresolved outcomes and its median/p95 reconciliation latency;
- report the performance/revenue opportunity cost of the verifier without weakening hard invariants;
- on larger optimizable cases, target mean regret <=1% and p95 <=5% against exact/MILP references where tractable.

For system performance, keep five distinct denominators: mechanical swap success, turnaround latency, charge restoration, network-level resource availability and successful on-time missions. Add a sixth for verified grid service: delivered kW/kWh, response accuracy, sustain duration and reserve restoration.

## 8. Next executable phases

- **v0.1 (this branch):** single-site bounded commitment admission and unresolved-state reservation.
- **v0.2:** persistent append-only commitment journal with compare-and-swap/version preconditions, consume-once idempotency, canonical receipts and independently verified terminal release. This should compose #6700 rather than duplicate its physical state journal.
- **v0.3:** multi-site/transport leases, common-cause grouping, transfer windows, pack custody and network partition reconciliation.
- **v0.4:** matched optimizer tournament and adversarial benchmark corpus.
- **v0.5:** cryptographically authenticated evidence/receipt chain and an independently operated verifier; no authority is inherited from signatures alone.

## 9. Current execution boundary

The v0.1 source includes the evaluator, CLI, strict schema, synthetic known-answer fixture and 13 Rust test definitions. The first CI run must establish formatting, compilation and tests at the exact PR head. No Clippy, cryptographic authentication, physical test, market authorization or field performance is claimed.
