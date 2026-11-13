# LOCO-MODAL-FAULT-SPACE-001 — Common-cause sensor and actuator fault envelope

**Status:** protocol and synthetic fixtures. No physical fault-model completeness claim.

Issue: [#4342](https://github.com/Luminous-Dynamics/symtropy/issues/4342).

## Objective

Produce an independently auditable conservative set of sensor/actuator health signatures for the recoverability-gated failover state machine in #4332/#4334.

The current failover reference consumes one source-supplied set-completeness receipt and exact transition receipts. This workstream defines how the finite possible-signature envelope itself is represented, checked, canonically enumerated and resource-bounded. It does not implement OOD classifiers, sensor health estimation, fault probabilities or physical viability.

## Paradigm shift

A single learned OOD scalar is not a representation of the physical fault space. A collection of independent per-sensor labels is also insufficient when channels share hardware, clocks, power or derivation paths.

V0 treats the fault condition as a **set-valued joint state** over sensors and actuators, with explicit common-cause relations and an explicit conservative-approximation boundary.

No marginal probability multiplication or probabilistic independence assumption is allowed.

## Component and health domains

Every component has a stable ID, kind (sensor, actuator/drive, power, clock, compute or communication), common-cause group ID and a non-empty finite domain of typed health states such as Valid, Degraded, OOD, Unavailable and Unknown.

The component set includes D1/D2/D3 and X1/X2, plus actuator/thermal/voltage and shared resources when relevant.

## Common-cause joint groups

A group provides:
- stable group ID and source/model/platform digest;
- a unique member-component list;
- the finite joint tuple relation over all member components;
- exact per-component state domains;
- a source-issued receipt bound to scenario, model, platform, actuator, calibration/diagnostic identity and validity interval;
- an explicit declaration that the joint relation is complete for its stated scope.

Every component belongs to exactly one group. Each tuple must contain every member component exactly once; every state must belong to that component's domain. Duplicate or empty tuples are invalid. A missing component, missing relation, stale receipt, identity mismatch or incomplete relation returns Unknown/Invalid.

Channels with common causes belong together: e.g. RGB+depth from one camera, position+derived velocity from one encoder, D1+D2 sharing a power rail, or thermal/voltage/torque states belonging to the same drive unit.

## Conservative cross-group product

The global envelope is the set product of the independently source-qualified joint group relations. This is a **set over-approximation**, not a probability factorization: it may include impossible combinations between groups, increasing conservatism, but cannot narrow away a group tuple.

If cross-group constraints are known, the affected components must be put into one joint group in V0. Do not use unqualified exclusion rules to shrink the envelope. A future symbolic factor-graph/BDD representation needs its own conservative-completeness proof.

## Bounded exact enumeration

The reference:
- validates component and group uniqueness/coverage;
- validates every group receipt and relation;
- canonicalizes tuple/component order;
- checks Cartesian product size before allocation using checked arithmetic;
- has hard caps on component count, tuples per group and global signature count;
- rejects canonical duplicates;
- emits sorted signatures and a canonical textual encoding;
- returns ResourceExhausted/Unknown with **no partial usable output** if any cap is exceeded.

The canonical encoding is input to an upstream cryptographic hasher. This reference does not compute/authenticate a cryptographic digest or prove that upstream group relations are physically complete.

## Output and failover integration

The output contains:
- complete canonical possible signatures;
- canonical signature IDs and exact component health states;
- sorted source group receipt IDs and result digests;
- group/global counts and configured resource bounds;
- conservative over-approximation flag;
- canonical encoding for a separate source hasher/authenticator.

#4334 must consume these exact signatures, and obtain one physical mode-transition viability receipt per signature. It may not regenerate a smaller set from a learned classifier confidence score.

## Matched comparison

Compare:
- single-channel OOD fallback (PROOF-style);
- independent Cartesian state product (conservative but correlation-ignorant);
- source-qualified joint common-cause groups;
- learned fault-context/meta-RL estimate as a proposal only;
- complete conservative fault envelope plus #4334 receipt intersection.

Report signature coverage/omission on injected faults, group relation completeness, over-approximation, signature count, enumeration time/memory, resource exhaustion, admitted fallback fraction, false-safe count, failover latency/recovery and complete #4326 adverse-event vector.

A smaller signature set is not automatically better; it is qualified only if upstream fault-injection evidence supports the narrower relation.

## Frozen fixtures

See [fixtures/known_answers_v0_1.json](fixtures/known_answers_v0_1.json). The fixtures cover joint RGB/depth, encoder/derived-velocity and D1/D2 shared-power relations; invalid tuples; duplicate/missing components; stale/incomplete/mismatched group receipts; resource-cap exhaustion; permutation invariance; no-probability-factorization; and conservative cross-group products.

## Proposed qualification targets (not achieved)

- 100% exact agreement with independent brute-force enumeration under the resource cap.
- Zero omitted or duplicate tuples/signatures.
- Zero silent truncation or partial usable output.
- Zero domain-invalid health state acceptance.
- Zero probability multiplication or independence assumptions.
- Exact canonical permutation invariance.
- Every failover receipt references one emitted conservative signature.
- Physical completeness of each joint relation established separately by held-out fault injection.

## Claim ceiling

V0 qualifies finite enumeration and group-relation composition over supplied source-qualified relations. It does not prove those relations cover every real fault, estimate probabilities, authenticate receipts, classify sensor health or establish robot safety.
