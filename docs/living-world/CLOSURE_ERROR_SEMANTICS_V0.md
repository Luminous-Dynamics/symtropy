# Living World Closure Error Semantics V0

Status: normative design contract; documentation only.

## Purpose

A scalar approximation error is not scientifically interpretable without naming the observable, metric, temporal horizon, aggregation semantics, and domain over which that error was established.

## Core invariant

> A closure is qualified only for the typed observable × metric × horizon × domain contract actually established by evidence. Numeric similarity between unrelated error claims creates no authority.

## Typed qualification

A closure qualification SHOULD identify at least:

- closure/model version;
- observable or derived quantity being evaluated;
- error metric identity/version;
- bound/value and unit/scale semantics;
- temporal evaluation horizon;
- aggregation/statistical semantics;
- spatial/biological applicability domain;
- evidence lineage/status.

## Non-interchangeability

The following claims are distinct even when they carry the same numeric bound:

- mean biomass relative error;
- maximum biomass error;
- extinction-probability calibration error;
- false-negative rate near a mortality threshold;
- spatial distribution distance;
- one-step state error;
- accumulated trajectory drift over 10,000 ticks.

A process declares the error contract it accepts. The evaluator MUST NOT infer compatibility merely from a common scalar such as ppm.

## Exact evidence

Exact capability for the same required information semantics may satisfy a closure-permitted requirement without using the closure path.

Exactness does not erase independent spatial/temporal/authority requirements.

## Horizon

Evidence established for one-step prediction does not authorize repeated long-horizon closure execution unless the qualification explicitly covers the required horizon or a separate composition theorem/evidence contract applies.

Repeated fidelity transitions are assessed for accumulated drift, not only local one-transition error.

## Zero-reference domains

Relative error metrics require explicit denominator semantics. They MUST NOT be applied blindly to observables that may be zero or near zero.

Absolute-unit or hybrid bounds SHOULD be used when relative error is undefined or biologically misleading.

## Tail / threshold behavior

Mean/RMS error does not authorize a threshold-sensitive process when rare/tail errors can alter canonical events such as extinction, infection, fracture, or recruitment.

A threshold process may require its own event-probability or false-positive/false-negative qualification.

## Evidence revocation/versioning

Metric definitions and observable semantics are versioned. Stale or unknown metric/qualification identities fail closed through the information-policy registry.

## Qualification fixtures

- same numeric bound, different metric -> incompatible;
- same metric, different observable -> incompatible;
- one-step qualification cannot satisfy 1,000-tick requirement without explicit horizon support;
- mean error cannot satisfy max-error requirement;
- threshold process rejects aggregate-mean-only evidence;
- relative-error policy handles zero-reference domain explicitly;
- exact capability can satisfy closure-permitted requirement for same information semantics;
- registry rejects stale/unknown qualification identity;
- save/reload preserves the exact closure qualification used by canonical policy;
- Observatory reports observable × metric × horizon behind each closure-enabled decision.

## Relationship

This contract strengthens closure evidence used by process sufficiency, spatiotemporal qualification, and canonical fidelity selection. It does not require one universal error metric for ecology.

Relates to #263, #269, #270, #272, #273, #252.
