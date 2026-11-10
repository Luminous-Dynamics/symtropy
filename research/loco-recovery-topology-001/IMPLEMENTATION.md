# LOCO-RECOVERY-TOPOLOGY-001 — auditor implementation notes

**Protocol:** \`README.md\`  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4511  
**Implementation PR:** https://github.com/Luminous-Dynamics/symtropy/pull/4540

## Purpose and boundary

The dependency-free Rust library in \`src/lib.rs\` composes caller-supplied viability and transition evidence into a finite fault-domain-aware recovery topology vector. It does not compute physical transitions, determine viability, select actions, diagnose faults, authenticate upstream receipts, infer complete common-cause coverage, or certify robot safety.

## Definitions

A physical route class is keyed by the ordered physical edge identity sequence and terminal recovery-region identity. Action IDs, policy labels and gait names do not contribute to physical-path identity.

For each route class \`r\`, \`D(r)\` is the set of source-qualified common-cause failure-group IDs obtained by expanding its component/domain dependencies through the supplied domain manifest.

The maximum independent-route count is the exact finite set-packing result:

\`max |S| such that D(r_i) ∩ D(r_j) = ∅ for every distinct i,j in S\`

The minimal cut sets are all inclusion-minimal hitting sets \`C\` satisfying:

\`C ∩ D(r) ≠ ∅ for every qualifying physical route r\`

The minimum cut-set size is the cardinality of the smallest returned hitting set. The auditor reports every inclusion-minimal hitting set, which can include larger minimal sets as well as globally minimum-cardinality sets.

These are graph/domain-manifest properties, not physical safety probabilities or universal fault-tolerance guarantees.

## Admission contract

A route is considered only when all of the following hold:

- upstream action admission is explicitly Admitted;
- physical branch coverage is complete;
- every physically possible successor is explicitly Viable (probabilities do not remove possible branches);
- intermediate-state and latency-prefix inventories are complete, and every supplied intermediate state is Viable;
- viability and transition receipts are present, context-matched and fresh at the evaluation tick;
- worst-case recovery steps and energy are measured and within declared hard budgets;
- every failure-domain ID resolves to a source-qualified common-cause group.

A known Nonviable branch or intermediate state rejects the route. An Unknown, missing or stale item makes the route Unknown. Context mismatches, duplicate identities and malformed canonical path inputs are invalid input errors.

If any unresolved route remains, the auditor reports known qualifying routes only as a lower bound and suppresses global independent-route and cut-set metrics. Missing evidence is never converted into assumed independence.

## Alias handling

Qualified records with the same canonical physical path form one route class, regardless of action-label differences. To avoid overstating resilience, dependencies from all qualified aliases are conservatively unioned; maximum worst-case steps and energy are retained. A future version may represent alias-specific independently selectable execution variants, but only with evidence proving their separation.

## Bounded computation

- 1,024 input route records;
- 256 component failure-domain records;
- 24 distinct physical route classes;
- 16 common-cause failure groups for exact minimal-cut enumeration;
- 4,096 branch records and 4,096 intermediate states per route.

Inputs outside these limits return explicit errors. The algorithms do not silently truncate or interpolate.

## Tests

Nineteen unit tests cover action-alias collapse, shared power/clock cuts, zero-probability-but-physically-possible dead ends, nonviable intermediate latency states, missing failure-domain identity, energy limits, stale evidence, context mismatch, common-cause group collapse, permutation invariance, monotonicity under added physical possibilities, resource bounds, two disjoint recovery routes and conservative alias dependency union.

## Verification status

The intended exact-head check is Rust 1.96 formatting, locked unit tests and strict Clippy. The local authoring environment does not have \`cargo\`, \`rustc\` or \`rustfmt\`, so no local compilation/test/format/Clippy pass is claimed. The GitHub workflow must finish on the exact PR head before a Rust pass is reported.

A green workflow would qualify only finite arithmetic, boundedness and supplied-record semantics. It would not prove that the upstream transition abstraction covers the physical robot, the failure-domain manifest contains all real common causes, receipts are authentic, routes are safe in continuous time, or any robot is physically fault tolerant.
