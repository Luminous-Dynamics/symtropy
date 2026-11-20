# Orbital Robust Compatibility Benchmark v0.1

**Status:** synthetic adapter and benchmark proposal; no hardware compatibility or physical metrology is claimed.  
**Owner:** [ORBITAL-ROBUST-COMPATIBILITY-001 #6351](https://github.com/Luminous-Dynamics/symtropy/issues/6351).  
**Parent:** [ORBITAL-INTERFACE-CONFORMANCE-001 #6329](https://github.com/Luminous-Dynamics/symtropy/issues/6329) / draft [PR #6336](https://github.com/Luminous-Dynamics/symtropy/pull/6336).

## Paradigm shift

Nominal profile containment is necessary but not enough. This adapter checks whether an independently qualified provider capability still contains a client's requirement after conservative uncertainty margins and exact lifecycle/configuration effectivity are applied.

It consumes a strict **robust-adapter envelope** intended to normalize receipts from the existing uncertainty owners. This envelope is not yet a canonical schema emitted by #5062/#3972; the cross-owner producer adapters remain future work. The validator does not compute GUM budgets, covariance, Monte Carlo propagation, calibration truth, model discrepancy or physical performance.

## Robust decision rules

### Scalar thresholds

Scalar comparisons are direction-specific. For minimum capability metrics (approach-speed capacity, contact-force capacity, retention force, receiver-accepted mass):

`robust_margin = (provider_nominal - provider_uncertainty_minus) - (client_nominal + client_uncertainty_plus)`

For maximum alignment error, which is an upper-bounded error (smaller is better):

`robust_margin = (client_tolerance - client_uncertainty_minus) - (provider_error + provider_uncertainty_plus)`

A non-negative margin is required in the applicable direction.

### Supported intervals

Shrink the provider interval and expand the client requirement:

`provider_safe = [provider_min + provider_u_plus, provider_max - provider_u_minus]`

`client_required = [client_min - client_u_minus, client_max + client_u_plus]`

The client-expanded interval must be wholly contained in the provider-shrunken interval. Both directional margins are reported separately.

### Uncertainty semantics

Only upstream `hard_bounded_error` receipts with Qualified disposition can participate in this deterministic v0.1 gate.

Standard uncertainty, expanded uncertainty, probabilistic coverage intervals and prediction intervals are distinct representations. v0.1 returns Indeterminate for them; there is no universal probability or risk threshold. An expanded uncertainty interval is not a deterministic physical bound.

The adapter requires explicit dependence evidence (joint model, evidence-backed independence, or no-shared-source assessment), scoped to **both** exact provider and client profile/configuration pairs and the context of use. Even a no-shared-source claim must name both sides; two separate self-scoped attestations do not establish pairwise independence. Receipts that claim the same dependency group must point to the same dependence evidence and identical profile-binding set. No default independence is assumed.

## Lifecycle/configuration binding

Each upstream uncertainty receipt binds exact profile ID, configuration revision, lifecycle-state ID, measurand, unit, context and validity window. A changed configuration, post-maintenance state or expired receipt invalidates prior qualification.

## Frozen corpus and independent replay

The corpus contains 32 synthetic known-answer cases:
- 5 robustly compatible;
- 13 rejected;
- 14 indeterminate;
- 0 expected-result mismatches in a separate JavaScript replay.

A nominal-only shortcut accepts 29/32 cases, including 24 cases that the robust oracle rejects or leaves indeterminate. The expanded corpus includes a shared calibration/dependence receipt scoped across both provider and client profiles, plus a missing-profile-binding control. This is an adversarial synthetic fixture result, not an industry false-accept rate.

Cases cover hard-bound positive and zero-margin controls; direction-aware alignment-error tolerance; scalar and two-sided interval guard-banding; uncertainty crossing; probabilistic/expanded-uncertainty abstention; missing, stale, malformed or wrong-scope receipts; dependence unknown and conflicting shared-source receipt identities; upstream dispositions; and lifecycle/configuration changes.

## Integration ownership

- #6329 / #6336: nominal interface profile compatibility.
- #5062 / #3972 / #2995: uncertainty receipt, dependence and propagation authorities.
- #6266: synchronized dynamic measurement where applicable.
- #6195: lifecycle degradation and service history.
- #6256 / #6303: future ground-cell evidence acquisition.

## Claim ceiling

A pass establishes only correct synthetic hard-bound adapter semantics over the adapter-envelope fixture. It does not establish that the upstream uncertainty owners have emitted or authenticated that envelope, the truth of a measurement bound, calibration, covariance, a probability distribution, physical compatibility, a safe docking envelope, seal integrity, flight qualification, mission reliability or operational availability.
