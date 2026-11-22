# UWB fault-hypothesis position envelope

**Status:** deterministic software reference kernel; exact-head workflow and independent review pending.

**Parent:** [NAV-UWB-FAULT-ISOLATION-001 #6900](https://github.com/Luminous-Dynamics/symtropy/issues/6900).

## Purpose

The existing independent posterior auditor checks whether a candidate point and absolute error radius are geometrically compatible with qualified ranges and whether anchor directions support the claimed spatial dimensions. It does not quantify how far a solution can move when a declared anchor/radio/clock/calibration dependency group is excluded or assigned a different bounded-fault model.

This module supplies the next narrow layer: it consumes a precomputed, externally qualified position set for each declared fault hypothesis, then constructs a conservative outer ball around the nominal solution that contains all supplied sets.

It is deliberately not a new position estimator, fault detector, cryptographic verifier, calibration engine or navigation authority.

## Mathematical contract

For nominal solution centre x0 and a hypothesis-specific bounded position set

X_h = { x : ||x - x_h||_2 <= r_h },

the farthest point in that set from the nominal centre is bounded by

R_h = ||x_h - x0||_2 + r_h.

For a complete declared hypothesis set H, the module returns

R = max over h in H of R_h.

The radius R describes a ball centred at x0 that contains every supplied hypothesis-specific set. It is conservative for that fixed centre; it is not necessarily the smallest enclosing ball over all possible centres.

The result is a **candidate deterministic outer envelope**, not a calibrated statistical protection level. It supports a protection-level claim only when every per-hypothesis position set is itself valid, the hypothesis inventory covers the relevant fault model, and the associated source, clock, calibration, anchor and dependency evidence has been independently qualified.

## Typed input

Each inventory carries:

- nonzero inventory and operating-profile identities;
- SHA-256 commitments for controller-visible inputs and the canonical hypothesis inventory;
- an expected hypothesis count and an explicit enumeration-complete flag;
- a declared maximum simultaneous fault-domain cardinality;
- exactly one nominal solution and one position set per declared hypothesis.

Each non-nominal hypothesis records stable dependency-group IDs, affected observation IDs, a fault family and either a bounded 3D position set or an explicit unbounded reason.

The digest fields are commitments supplied by the caller. This kernel checks that they are present and preserves them; it does not hash source data, authenticate the source, or prove that different group IDs are physically independent.

## Fail-closed semantics

A protection radius is emitted only when:

1. exactly one nominal hypothesis exists;
2. every hypothesis identity and affected scope is structurally valid;
3. the expected and evaluated hypothesis counts match;
4. the enumerator declares completion;
5. every hypothesis supplies a finite bounded position set;
6. the hypothesis and fault-cardinality budgets are respected.

An incomplete inventory or any unbounded position set returns **IntegrityUncertain** and **no protection radius**. The maximum over only the hypotheses that happened to fit the compute budget is not reported as a protection level.

A complete envelope above the profile-owned alert limit returns **ExceedsAlertLimit**. The module does not grant NORMAL, alter a service state or authorize an action.

## Monotonic properties tested

- Adding a hypothesis cannot shrink the returned outer radius.
- Widening a hypothesis-specific position set cannot shrink the returned outer radius.
- An incomplete inventory cannot produce a protection radius.
- An unbounded hypothesis suppresses the protection radius for the complete inventory.
- Canonical hypothesis output is invariant to input order.
- Duplicate hypothesis, dependency-group and observation identities are rejected.

## Deliberate boundaries and remaining proof obligations

- The caller supplies the hypothesis-specific solutions. This module does not compute leave-one-anchor-out estimators or solve the range equations.
- The module assumes supplied position-set radii are already valid for their stated fault hypothesis. It does not calibrate them.
- The completeness of the fault inventory is a caller assertion bound to a manifest. This implementation does not derive the dependency graph or prove that a physical failure mode was not omitted.
- A common clock/antenna/calibration group must be represented as one declared fault domain; identity labels alone do not prove independence.
- No statistical risk allocation or probability is invented. The output is deterministic and conditional.
- Unknown/unmodelled fault coverage requires an explicit bounded hypothesis or an unbounded disposition; it cannot silently disappear from the inventory.
- No physical Bregar or MILUV payload has been consumed by this kernel.

## Benchmark path

Compare with:
- all-in-view least squares / EKF;
- leave-one-anchor-out solution separation;
- the ATM adaptive temporal Kalman + solution-separation baseline in [#6733](https://github.com/Luminous-Dynamics/symtropy/issues/6733), after full method/config acquisition and faithful reproduction;
- Student-t and adaptive control-theoretic protection-level baselines in [#6470](https://github.com/Luminous-Dynamics/symtropy/issues/6470).

Report integrity containment, false acceptance, false exclusion, alert delay, availability/continuity, envelope width, hypothesis count, p99 latency, RAM and energy separately. Do not collapse them into a weighted score.

## Claim ceiling

Synthetic unit tests validate the deterministic envelope arithmetic and fail-closed input contract only. They do not establish real range/pose coverage, attack detection, rare-event integrity risk, cryptographic security, FiRa certification, physical navigation safety or superiority over SOTA.
