# Living World Adaptive Information Fidelity v0

Status: companion fidelity-policy contract. This document builds on process information requirements and qualified closure models.

## Purpose

Distance is a useful compute-budget signal, but it is a poor universal proxy for ecological importance or approximation error.

A distant epidemic near a bifurcation may need richer state than a nearby quiet meadow. A remote fire front, extinction bottleneck, invasive-species transition, or tightly coupled predator-prey oscillation may be highly sensitive to information that a coarse closure normally discards.

Living World should therefore refine not only because something is near the player, but because the **current representation is approaching the limit of what its evidence says it can predict**.

## Core policy

For every authoritative process allowed to use a qualified closure, define:

- the observables the closure predicts;
- the state/domain over which it is qualified;
- a conservative error bound or runtime error indicator;
- the maximum process tolerance;
- promotion and demotion hysteresis.

Then:

```text
closure error safely below tolerance
        -> remain coarse

closure error approaches promotion threshold
        -> request richer representation before authoritative error exceeds tolerance

richer representation remains safely unnecessary long enough
        -> collapse/demote if all process-information contracts remain satisfied
```

## Error is process-relative

There is no single global "fidelity error" scalar that is automatically meaningful.

Examples:

- disease dynamics may care about infection prevalence and spatial correlation;
- forest succession may care about age/size distribution and canopy gaps;
- hydrology may care about water/nutrient flux;
- predator-prey dynamics may care about abundance, spatial overlap, and kill rate;
- a narrative organism may require exact persistent identity even if population-level ecological error is tiny.

Each process exposes the observables/tolerances it actually claims.

## Exact requirements bypass approximation policy

If a process declares an information requirement as exact, no error budget may waive it.

For example:

```text
canonical player projectile hits projected animal
```

requires Level-A realization before hit resolution. A closure estimate that "probably one deer is there" cannot substitute for canonical individual authority.

Adaptive error budgeting applies only where the process contract explicitly permits a qualified closure.

## Closure applicability domain

A closure's evidence should bind to a domain such as:

- species/community regime;
- density/biomass range;
- spatial scale;
- temporal step/cadence;
- environmental range;
- interaction strengths;
- disease prevalence;
- disturbance regime;
- model version.

Leaving the qualified domain is itself a promotion trigger or fail-closed condition even before a numeric error estimator is available.

## Runtime indicators

Possible deterministic indicators include:

- distance from calibrated state-domain boundary;
- covariance/heterogeneity measures approaching a closure's validity limit;
- population count approaching extinction/bottleneck thresholds;
- rapidly increasing gradients or flux imbalance;
- divergence between independent coarse estimators;
- conserved-quantity residual diagnostics in approximate field models;
- sensitivity/Jacobian proxies where available;
- variance/entropy of represented strata;
- event density/disturbance intensity;
- recent closure-vs-refined discrepancy retained from observatory/calibration runs.

These indicators are model evidence, not renderer timing or wall-clock measurements.

## Promotion hysteresis

Promotion and demotion thresholds should differ to prevent representation thrashing.

Conceptually:

```text
promote when predicted_error >= 0.7 * tolerance
collapse only when predicted_error <= 0.3 * tolerance
              for a qualified minimum ecological duration
```

The constants are process/policy choices, not normative here.

The invariant is that hysteresis changes compute representation, not biological truth.

## Error budget composition

Multiple enabled processes may consume different portions of a representation's approximation budget.

A fidelity controller evaluates all active requirements and uses the strongest resulting demand.

Conceptually:

```text
required_fidelity(region)
  = join(requirement(process_1), ..., requirement(process_n))
```

where the join accounts for exact requirements, closure compatibility, and error tolerances.

A process may therefore force refinement for everyone sharing the same canonical state if its required information cannot be isolated safely in a smaller partition.

## Spatially localized refinement

Promotion need not refine an entire global population.

Where authority can be partitioned safely, the system may refine only:

- a spatial patch;
- a disease hotspot;
- a breeding cohort;
- a disturbance front;
- a threatened lineage;
- an interaction region.

The partition selection remains canonical and conservative. Renderer visibility is not the authority selector.

## Temporal refinement

Some processes may need finer temporal resolution only during rapid transients.

The same principle applies:

```text
slow stable process -> coarse cadence / closure
rapid transient      -> finer cadence or explicit substeps
```

Changing cadence must preserve the authoritative clock and use qualified integration/error rules rather than wall-clock frame rate.

## Global compute budget

A product may have finite simulation compute.

When all requested refinements cannot fit simultaneously, the runtime needs a declared resource policy rather than silently degrading arbitrary regions.

Priority can consider:

- exact-authority requirements that cannot be approximated;
- predicted error/tolerance ratio;
- causal/narrative importance;
- irreversible/extinction risk;
- player/agent interaction relevance;
- scientific observability policy;
- fairness/network authority constraints.

If a mandatory exact process cannot be supported under the available budget, the system must choose an explicit fallback (pause/defer/fail/transfer authority/etc.) instead of pretending a cheaper representation is equivalent.

## Observatory feedback loop

The Living World Observatory can improve runtime fidelity policy empirically:

1. run matched fine/coarse scenarios;
2. measure closure error across state regimes;
3. fit conservative applicability/error envelopes;
4. freeze evidence lineage/model version;
5. use those envelopes as runtime promotion indicators;
6. requalify whenever process/closure equations change.

This is a controlled feedback loop from evidence to compute allocation, not runtime self-modification of normative models.

## Rare events and tails

Mean-error metrics can hide rare catastrophic divergence.

Qualification should separately examine tail events such as:

- extinction versus survival;
- epidemic takeoff versus fadeout;
- invasion establishment;
- fire spread across a barrier;
- trophic cascade onset;
- reproductive rescue;
- critical infrastructure/ecosystem failure.

A closure that matches average biomass but misclassifies these branch outcomes is not adequate for processes where those branches matter.

## Determinism

Given the same canonical state, enabled process set, closure evidence versions, and fidelity policy, promotion/demotion decisions must be reproducible.

Invalid inputs include:

- CPU load timing;
- render frame rate;
- nondeterministic thread completion order;
- GPU visibility timing;
- wall clock.

A separately declared dynamic compute-budget policy may react to resource availability, but then that policy itself becomes part of the replay/evidence context and cannot silently alter canonical biological outcomes where exact requirements apply.

## Qualification requirements

Evidence must establish at least:

1. closure use is impossible outside its declared applicability/evidence version;
2. exact process requirements cannot be waived by an error budget;
3. deterministic indicators trigger promotion before the declared tolerance is exceeded under qualification fixtures;
4. demotion hysteresis prevents rapid refine/collapse thrashing;
5. matched fine/coarse scenarios validate runtime error envelopes over the claimed domain;
6. tail/branch outcomes are included where scientifically or gameplay-relevant;
7. adding an enabled process updates fidelity demand deterministically;
8. localized refinement conserves authority/count/extensive quantities exactly;
9. frame rate/thread timing cannot alter fidelity decisions;
10. stale closure evidence fails closed after model changes;
11. global compute-budget fallback behavior is explicit and observable;
12. the Observatory records why each promotion/demotion happened and which process requirement/error indicator caused it.

## Design principle

**Spend simulation detail where the evidence says approximation is becoming unsafe, not merely where the camera happens to be. Fidelity is an adaptive claim about information sufficiency under bounded error.**
