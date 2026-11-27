# Industry and standards comparator register — Maritime Cargo Lineage

## Existing standards to reuse

| Standard / ecosystem | What it contributes | What this benchmark adds without replacing it |
|---|---|---|
| DCSA Track & Trace | Shared data definitions and event domains for shipment, transport, equipment, IoT and reefer visibility | Exact source-local identity crosswalk qualification, alias validity/revision handling, conflict-preserving historical replay and conservative unique-box counting |
| GS1 EPCIS 2.0 | Common visibility-event vocabulary and What / When / Where / Why / How dimensions | Cross-source alias mapping and independently checkable rules for whether a receiver-receipt assertion can contribute one unique physical-box ID under the declared v0.1 rules |
| DCSA Port Call / JIT and IAPH-IHMA PCO | Port-call coordination and typed service-time milestones | Keep vessel visit/call identity distinct from container identity; never use a berth/voyage match as proof of box identity |
| Terminal operating systems and carrier platforms | Operational identifiers and transaction/event records within their authorized workflows | Read-only, independent reconciliation across system boundaries; no replacement TOS or mutation authority |

## Benchmarking the industry fairly

Do not claim that existing TOS, carrier platforms or visibility providers lack container tracking, event correlation or cargo milestone capabilities. Compare actual interfaces, mapping semantics, revision handling, false-match behavior and evidence quality on the same fixtures and source cutoffs.

A fair P3 comparison should include:
1. exact source-local lookup using authoritative identifiers;
2. source-specific deterministic mapping rules;
3. a deliberately naive string/time nearest-match baseline as a negative control only;
4. exhaustive finite resolution as the small-instance oracle;
5. the proposed revision- and time-bounded crosswalk verifier.

All methods receive identical information. The naive matcher must not be presented as a serious production competitor; it exists to quantify how many false joins arise from unsafe shortcuts.

## Scoreboard

- identity precision and recall against independently adjudicated gold mappings;
- false-merge and false-split rates;
- ambiguous/unmatched/revoked/out-of-window shares;
- source-event deduplication and revision correctness;
- unique receipt-backed physical-box IDs versus raw event-row/alias counts;
- planned/provider-asserted delivery promotions incorrectly counted as delivered;
- historical publication-cutoff leakage;
- runtime, peak memory and exact-oracle gap as the corpus scales.

A zero false-merge target is appropriate for the finite admitted corpus. On real feeds, publish a measured error rate and uncertainty interval; do not claim zero risk from a synthetic test.


## Versioned standards and access boundaries (verified 21 November 2026)

### DCSA Track & Trace

- Public DCSA overview: https://dcsa.org/standards/track-and-trace
- DCSA states that Track & Trace 2.2.0 established the common foundation for shipment, transport and equipment visibility.
- DCSA published Track & Trace 3.0.0 on 30 September 2026. The release adds IoT and reefer event types, improves shipment/document correction and withdrawal communication, and supports forwarding/intermediary exchange.
- DCSA says 3.0.0 will initially be available to DCSA members and DCSA+ partners, with public release planned for the second half of 2027: https://dcsa.org/newsroom/track-and-trace-3-0-0-iot-reefer-visibility

**Implementation rule:** use publicly accessible 2.2.0 material for current open conformance work. Do not copy, infer or claim conformance to restricted 3.0.0 schemas. Keep the adapter/profile version explicit and add 3.0.0 support only from a lawfully accessible, version-pinned specification and test suite.

### GS1 EPCIS 2.0

- Official standard: https://www.gs1.org/standards/epcis
- EPCIS 2.0 is an open visibility-event data model with standard capture and query interfaces for sharing events across applications and enterprises.
- EPCIS supplies interoperable event semantics; it does not automatically establish that a source-local identifier is the same physical container as another system's identifier. That still requires a qualified identifier and mapping assertion.

### Terminal operating systems

- Kaleris N4 product information: https://kaleris.com/terminal-operating-system/kaleris-n4-tos/
- Kaleris publicly describes N4 as coordinating vessel, yard, rail, gate and control-room operations, with RTG, terminal-truck and yard-intelligence optimization capabilities.
- These are vendor capability statements, not independent comparative benchmark results. The project must not claim that existing TOS platforms lack integrated terminal coordination or container tracking.

**Positioning:** integrate with the authorized TOS, carrier, port, rail and EPCIS/DCSA interfaces. Add cross-organization identity reconciliation, source conflict and revision accounting, historical-cutoff replay, evidence qualification and independent delivered-service measurement beside those systems.

## Research baselines: integrated optimization is already advancing

The P3 optimizer comparison must include more than berth-only FCFS:

| Research comparator | Published method / scope | Benchmark implication |
|---|---|---|
| Robust BQCY assignment (Computers & Industrial Engineering 218, August 2026) | Two-stage robust berth allocation, quay-crane assignment and yard assignment; column-and-constraint generation for exact small instances plus a heuristic for larger instances | Reproduce the exact small-instance oracle and a scalable robust BQCY baseline |
| Integrated berth / quay crane / shore power / energy (Journal of Cleaner Production, 18 August 2026) | Joint berth, crane, onshore power supply and multi-source energy dispatch; MILP reformulation and multiple solution strategies | Include electrical compatibility, energy dispatch and carbon boundaries rather than treating shore power as a binary berth attribute |
| Robust berth / yard / shore power (Ocean Engineering, 15 June 2026) | Uncertain vessel arrivals, robust planning and hierarchical multi-agent RL; paper reports 4–13% cost savings within its tested cases | Reproduce its scenario and objective definitions before comparing; do not transfer its percentage as a general port target |
| Time-variant quay / yard crane allocation (Transportation Research Part E, online 14 October 2026) | Two-stage robust allocation under external-truck arrival and yard-health uncertainty | Include gate/truck arrival uncertainty and yard equipment health in held-out disruption tests |

References:
- https://doi.org/10.1016/j.cie.2026.112092
- https://doi.org/10.1016/j.jclepro.2026.149105
- https://doi.org/10.1016/j.oceaneng.2026.125850
- https://doi.org/10.1016/j.tre.2026.105314

These papers optimize bounded terminal subproblems. They do not, by themselves, establish the quality of a cross-terminal, feeder, rail and final-receiver custody/identity assurance layer. Conversely, this PR's synthetic identity verifier is not an optimizer and cannot claim to outperform those methods. Use them as complementary baselines at the correct layer.
