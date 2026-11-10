# LOCO-RECOVERY-TOPOLOGY-001 — Rust reference auditor

**Status:** bounded finite reference implementation, v0.1  
**Protocol:** [#4511](https://github.com/Luminous-Dynamics/symtropy/issues/4511)  
**Protocol PR:** [#4512](https://github.com/Luminous-Dynamics/symtropy/pull/4512)

## Purpose

This crate calculates a finite **fault-domain-aware recovery topology vector** from caller-supplied evidence. It does not compute physical transitions, determine viability, select actions, diagnose faults, authenticate upstream receipts, or certify robot safety.

The caller must first qualify transition and viability evidence through the existing authorities. This crate composes those decisions and measures the resulting recovery-path topology.

## Exact finite definitions

A physical route class is keyed by:

\`(ordered physical edge identity sequence, terminal recovery-region identity)\`

Action IDs, policy labels and gait names do not contribute to physical-path identity.

For each route class \`r\`, \`D(r)\` is the set of source-qualified common-cause failure-group IDs obtained by expanding its component/domain dependencies through the supplied domain manifest.

The maximum independent-route count is the exact finite set-packing result:

\`max |S| such that D(r_i) ∩ D(r_j) = ∅ for every distinct i,j in S\`

The minimal cut sets are all inclusion-minimal hitting sets \`C\` satisfying:

\`C ∩ D(r) ≠ ∅ for every qualifying physical route r\`

The minimum cut-set size is the cardinality of the smallest returned hitting set.

These are graph/domain-manifest properties, not physical safety probabilities or universal fault-tolerance guarantees.

## Admission contract

A route is considered only when all of the following hold:

- the upstream action admission is explicitly Admitted;
- physical branch coverage is complete;
- every physically possible successor is explicitly Viable (probabilities do not remove possible branches);
- the intermediate-state/latency inventory is complete and every supplied intermediate state is Viable;
- viability and transition receipts are present, context-matched and fresh at the evaluation tick;
- worst-case recovery steps and energy are measured and within the declared hard budgets;
- every declared failure-domain ID resolves to a source-qualified failure group.

A known Nonviable branch or intermediate state rejects the route. An Unknown, missing or stale item makes the route Unknown. Context mismatches, duplicate identities and malformed canonical path inputs are Invalid errors.

If any unresolved route remains, the auditor reports known qualifying routes only as a lower bound and suppresses the global independent-route count and cut-set metrics. It does not convert missing information into assumed independence.

## Alias treatment

Qualified records with the same canonical physical path are one route class, regardless of action-label differences. To avoid overstating resilience, the dependencies of all qualified aliases are conservatively unioned, and the maximum worst-case steps and energy are retained. This is intentionally conservative: a future version may represent independently selectable alias-specific execution variants, but it must prove their separation without inflating route count.

## Bounded computation

- Up to 1,024 input route records.
- Up to 256 component failure-domain records.
- Up to 24 distinct physical route classes.
- Up to 16 common-cause failure groups for exact minimal-cut enumeration.
- Up to 4,096 branch records and 4,096 intermediate states per route.

Inputs outside these limits return an explicit error. The algorithms do not silently truncate or interpolate.

## Test coverage

The unit suite covers:
- R01 action-alias collapse;
- R02 shared power/clock cut sets;
- R03 zero-mass but physically possible dead-end branch;
- R04 nonviable intermediate latency state;
- R05 missing failure-domain identity suppressing independence;
- R06 energy-limited recovery;
- permutation invariance;
- stale receipts, duplicate IDs and context mismatch;
- common-cause group collapse;
- monotonicity under additional physically possible outcomes;
- missing measurements and resource bounds;
- two genuinely domain-disjoint recovery routes;
- conservative dependency union across aliases.

## Verification

The standalone crate has no third-party dependencies. The intended exact-head check is Rust 1.96 formatting, locked unit tests and strict Clippy. This authoring environment does not have \`cargo\`, \`rustc\` or \`rustfmt\`; therefore no local Rust compilation/test/format/Clippy pass is claimed. The workflow must complete against the exact PR head before any such claim is made.

## Claim ceiling

A green workflow qualifies only the finite arithmetic, boundedness and supplied-record semantics. It does not prove:
- that the upstream transition abstraction soundly covers a physical robot;
- that the failure-domain manifest contains every common cause;
- that receipts are authentic rather than merely identity-consistent;
- that the route is safe in continuous time;
- that any physical robot has the reported recovery reserve;
- that a robot is safe, fault tolerant or standards certified.

Hardware experiments remain under the canonical matched fault campaign #4358.
