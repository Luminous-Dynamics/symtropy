# Industry and standards comparator register — Maritime Cargo Lineage

## Existing standards to reuse

| Standard / ecosystem | What it contributes | What this benchmark adds without replacing it |
|---|---|---|
| DCSA Track & Trace | Shared data definitions and event domains for shipment, transport, equipment, IoT and reefer visibility | Exact source-local identity crosswalk qualification, alias validity/revision handling, conflict-preserving historical replay and conservative unique-box counting |
| GS1 EPCIS 2.0 | Common visibility-event vocabulary and What / When / Where / Why / How dimensions | Cross-source alias mapping and independently checkable rules for whether a receiver event can count as one delivered physical box |
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
- unique physical boxes delivered versus raw event-row/alias counts;
- planned/provider-asserted delivery promotions incorrectly counted as delivered;
- historical publication-cutoff leakage;
- runtime, peak memory and exact-oracle gap as the corpus scales.

A zero false-merge target is appropriate for the finite admitted corpus. On real feeds, publish a measured error rate and uncertainty interval; do not claim zero risk from a synthetic test.
