# Maritime Custody Timeline Scaling Benchmark Protocol v0.1

Issue: [MARITIME-CUSTODY-TIMELINE-SCALING-001](https://github.com/Luminous-Dynamics/symtropy/issues/6725)

## Correctness contract

For one canonical box, let each effective ACTUAL event have a qualified closed interval [e_i, l_i] on the same normalized time basis. The strict total-order claim is valid exactly when the intervals are pairwise disjoint and can be ordered by time.

Sort by (earliest, latest, exact source-event reference). Scan in that order and compare the current earliest endpoint against the greatest latest endpoint among earlier intervals:

max(previous latest endpoints) < current earliest

A greater-than-or-equal result rejects the chain. This rejects both overlap and touching; an equal singleton timestamp is not a simultaneity certificate. A differing time-basis identifier rejects as incomparable before ordering.

A prefix maximum is retained as the direct invariant: the current interval must begin strictly after every prior interval, not merely after a nominal event time. This is O(n log n) for sorting plus O(n) for the scan, rather than the O(n^2) pair enumeration used as the independent test oracle.

## Why the proof is equivalent

If the scan accepts, every prior interval's latest endpoint is no greater than the prefix maximum, and that maximum is strictly less than the current earliest endpoint. Therefore every prior interval is strictly before the current interval. This holds for every current event, so all pairs are strictly ordered.

If any pair overlaps or touches, take the later-starting interval of that pair. At the point it is scanned, the earlier interval's latest endpoint is included in the prefix maximum, so the scan rejects. Thus acceptance and rejection are equivalent to the pairwise oracle under the required common time basis and proof-grade enclosure conditions.

This optimization changes only the complexity of the proof step. It does not weaken clock qualification, source-event/revision binding, as-of visibility, event identity, or any custody-state transition.

## Independent oracle

Tests compare the optimized scan against the existing pairwise compare_time_evidence predicate for a finite Cartesian set of interval centers and radii. The corpus includes:

- ordered disjoint intervals;
- endpoint touching;
- same-start and same-end intervals;
- identical singleton intervals;
- nested and partially overlapping intervals;
- a long interval containing several short intervals;
- input permutation;
- a different normalized time basis.

The pairwise comparator remains available as a diagnostic primitive. It is not called by the optimized production scan.

## Complexity and memory

For n effective actual events in one box:

- prior pairwise proof: n(n-1)/2 comparisons, O(n^2) time;
- optimized proof: O(n log n) comparison sort plus O(n) scan;
- auxiliary storage: O(n) for the existing event vector and O(1) scan state beyond sorting.

A box with 100,000 events would require nearly five billion unordered pair checks under the former method; the optimized order stage requires sorting and a linear pass. This is an algorithmic operation-count illustration, not a measured runtime claim.

## Performance experiment

The next repeatable benchmark should generate 10, 100, 1,000, 10,000 and 100,000 event timelines per box, with separate distributions for:

1. strictly disjoint ordered intervals;
2. same-start and nested overlapping intervals;
3. a late-arriving long enclosure;
4. one touching pair near the end;
5. mixed clock bases;
6. reversed input order.

Record the exact commit SHA, Rust compiler, OS, CPU model, optimization flags, input seed, interval distribution, sort time, scan time, end-to-end timeline qualification time and peak memory. Run repeated trials on a pinned runner and report median, p95 and p99; do not promote unpinned CI wall-clock timings to a product SLO.

## Acceptance and claim ceiling

- The exhaustive finite pairwise oracle and optimized algorithm agree on every test set.
- Any overlap, touching, different time basis, missing event-time binding, estimate-only evidence or unqualified clock remains a hard rejection.
- Offender reporting is deterministic under input permutation.
- Exact-head format, locked tests, strict Clippy and read-only custody evaluation pass.
- No wall-clock speedup is claimed until a reproducible pinned-runner measurement is published.
