# LOCO finite-horizon risk spending V0

Dependency-free Rust reference for preallocating one family-wise chance-risk budget over a fixed locomotion horizon and accounting for each allocation without recycling.

Parent: [LOCO-RISK-SPENDING-001 / #4232](https://github.com/Luminous-Dynamics/symtropy/issues/4232).

## Why this is separate from the one-step chance gate

[#4010](https://github.com/Luminous-Dynamics/symtropy/issues/4010) screens one candidate action using five one-sided marginal bounds. Reusing the same per-step alpha on every footfall does not preserve that same risk bound over an episode.

For horizon H and five one-sided physical constraints per step, V0 preallocates integer alpha amounts:

    sum(t=1..H) sum(j=1..5) alpha[t,j] <= alpha_total

Given a valid conditional marginal tail bound for every event, the union bound gives:

    P(any declared violation anywhere in the horizon)
      <= sum(t,j) alpha[t,j]
      <= alpha_total

No independence between constraints or time steps is required. The marginal validity still must hold under the declared history/filtration and operating domain. This crate allocates **operational violation risk alpha**; it does not validate those probability models.

Keep alpha separate from **statistical confidence delta** in estimated marginal tail bounds. The one-step chance gate #4231 currently reports pointwise 95% Clopper–Pearson upper bounds per constraint; that is not simultaneous 95% confidence across all constraints or the horizon. A simultaneous horizon-wide confidence claim requires a separately declared delta budget allocated over the 5H marginal validation claims (and matching finite-sample bounds). The risk ledger V0 does not allocate delta and does not turn pointwise calibration intervals into a joint confidence guarantee.

## Deterministic allocation

V0 divides alpha_total_ppb equally across all 5H events using checked integer arithmetic. The quotient is given to every event; the indivisible remainder is distributed in canonical (step index, constraint index) order. Every event must receive a positive representable ppb allocation or plan construction fails.

Skipped risk is not transferred forward. The plan is immutable after construction; there is no method for refilling, recycling or increasing a later step's allocation.

## Allocation profiles

The default constructor creates a uniform per-event profile with deterministic quotient/remainder placement. A second constructor, `HorizonRiskPlan::from_preallocated_allocations`, accepts an immutable vector of five-element per-step allocations. This supports predeclared phase- or constraint-weighted profiles (for example, higher allocations at touchdown/support exchange) while preserving one exact total family-wise budget.

The nonuniform profile must be selected before the horizon begins, must allocate a positive representable amount to every (step, constraint) event, and must sum exactly to the declared total. The ledger exposes no post-outcome profile edit or risk transfer. Compare uniform and nonuniform profiles at the same total alpha; any adaptive profile must be predictable from prior history and fixed before the current event, not selected after its outcome.

## Execution lifecycle

Each step is tracked as one of:

- **Pending:** its immutable alpha allocation is reserved for that future step.
- **Committed:** an independently verified one-step admission receipt binds the exact episode, risk plan, step, action, predictive model, actuator model, operating domain, calibration and per-constraint alpha. Its allocation is reserved.
- **Consumed:** a verified action-execution receipt binds the preceding admission, exact action/digest, delivered-action identity, measured-actuation evidence, model/actuator/domain/calibration identities and execution time. The allocation is irreversibly consumed.
- **Skipped:** an independent no-delivery receipt proves no action was executed. The allocation remains unused and is not redistributed.
- **Unknown:** the committed action's delivery/execution evidence is unresolved. Its alpha remains locked, is not counted as unused, and the horizon cannot advance. It can only be reconciled by a matching verified execution receipt (Consumed) or a matching independent no-delivery receipt (Skipped).

Receipt IDs are unique within the in-memory ledger. Event timestamps are monotonic. Replayed, duplicate, wrong-step or identity-mismatched receipts are rejected. Trust flags and evidence digests are upstream inputs; this reference checks binding but does not cryptographically authenticate source artifacts.

## Accounting outputs

The summary keeps these amounts distinct:
- committed/reserved;
- consumed;
- unknown/locked;
- skipped/unused;
- pending/preallocated.

Their exact sum must equal the original family-wise alpha. Unknown alpha is never classified as unused. Pending and skipped alpha remain tied to their original steps and cannot be spent elsewhere.

## Synthetic tests

Twenty-seven deterministic Rust tests cover:
- horizons 1, 2, 3, 10, 127 and 1,000;
- quotient/remainder allocation and exact ppb conservation;
- positive allocation resolution and invalid horizon/budget limits;
- canonical five-constraint order;
- no transfer after skip or consume;
- committed versus consumed accounting;
- sequential step order and monotonic event time;
- admission, execution, skip and unknown receipt identity;
- wrong episode, action digest, model, actuator model, domain and calibration;
- unverified action delivery, execution and no-delivery receipts;
- duplicate receipt replay and attempted double consumption;
- Unknown locking and both allowed reconciliation paths;
- explicit accounting that Unknown is neither unused nor resolved;
- arithmetic/identity boundary cases.

Synthetic fixtures validate the allocation/state-machine contract only. They do not simulate a robot or establish per-step marginal probability validity.

## Commands

    cargo fmt --manifest-path research/loco-risk-spending-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-risk-spending-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-risk-spending-v0/Cargo.toml --locked --all-targets -- -D warnings

Dedicated workflow: `.github/workflows/loco-risk-spending.yml`.

## Integration

- #4010 supplies the one-step calibrated marginal chance screen.
- #4008 supplies action/clock/source-qualified contact-information evidence.
- #4119 supplies state-dependent certificate validity and deadline gating.
- #4058/#4061 supply robust set-based viability/recovery.
- #4100 owns the dual robust/probabilistic assurance design.

The runtime should require both the robust viability result and the finite-horizon chance-risk allocation result. Neither replaces certificate freshness or action-delivery verification.

## Claim ceiling

This crate proves arithmetic properties of its declared finite allocator and checks the supplied receipt identities and state transitions. The horizon union bound is conditional on valid event-wise marginal tail bounds. It is not a physical viability proof, CBF forward-invariance theorem, hardware safety case, or SOTA performance result.