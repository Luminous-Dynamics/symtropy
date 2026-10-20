# Physical mesh-refinement study v0.1

**Tracking:** [PHYS-SOLVER-QUAL-001 #2303](https://github.com/Luminous-Dynamics/symtropy/issues/2303)  
**Implementation:** stacked draft following [normalized numerical reference comparator #2312](https://github.com/Luminous-Dynamics/symtropy/pull/2312)  
**Status:** software protocol and synthetic known-answer controls; no external solver run in this tranche.

## Paradigm shift: one correct-looking mesh is not solution verification

The comparator can determine whether one normalized output lies within the selected reference tolerance. That does not establish whether the numerical solution is stable under discretization changes. Mesh refinement is a separate evidence dimension: we must see how primary error, solver observables and the declared degrees of freedom behave over a documented sequence of coarse-to-fine models.

The mesh study consumes exact-byte-bound comparison reports, checks report/reference/request/scene identities, then audits a predeclared refinement sequence. The mesh size and degree-of-freedom counts must strictly increase in resolution from each coarse level to the next. A minimum of three levels is mandatory.

## Inputs

The versioned `physical-mesh-refinement-study-v0.1` manifest records:

- exact request hash, request ID, scene digest and scene revision;
- exact reference-manifest digest, reference ID and comparison protocol;
- primary observable and exact unit inherited from the reference manifest;
- mesh family ID, mesh generator identity and refinement method;
- a coarse-to-fine ordered list of level ID, characteristic mesh size (h), degree-of-freedom count, and exact comparison-report SHA-256;
- minimum levels and whether strictly decreasing primary-observable error is a required acceptance criterion.

Every comparison report is supplied as an exact byte array and must match its predeclared digest. Each report must bind to the same request/scene/reference/protocol, declare verified output bytes and a producer PASS, preserve the reference observable inventory, and report internally consistent error arithmetic. The study does not accept a report that changes units or silently omits any reference observable.

## Metrics

For each level, record the reported observable value (u_h), reference value (u^*), absolute error, allowed error, and whether the output is within the declared tolerance:

```text
error(h) = |u_h - u*|
allowed_error = abs_tol + rel_tol * |u*|
```

Between consecutive levels, estimate observed order only when both errors are positive, finite and representable:

```text
p_observed = ln(error(h_coarse) / error(h_fine))
             / ln(h_coarse / h_fine)
```

This is a **pairwise observed order estimate**, not a proof of asymptotic convergence order. Zero error makes the logarithmic estimate undefined; underflow/overflow is reported instead of fabricating a value. Non-monotone error is preserved as a real diagnostic. It fails the study only when `require_strictly_decreasing_error = true` was declared in the study manifest.

The finest level must pass the complete numerical reference comparison, and its primary observable must lie within the exact reference tolerances, for the overall study to pass. Coarse levels may legitimately fail an accuracy threshold while showing decreasing error. A non-PASS producer, missing artifact bytes or unresolved comparison keeps the study `UNRESOLVED`; it is not a low-quality numerical score.

## Suggested external-solver acceptance profile

For the first linear-elastic cantilever/beam case under [PHYS-SOLVER-QUAL-001 #2303](https://github.com/Luminous-Dynamics/symtropy/issues/2303):

1. Freeze geometry and physical input, material model, boundary conditions, loads, output selectors and solver/build identity.
2. Use at least three mesh levels from one declared mesh family; record h and DOFs, element family/order, and all relevant solver tolerances/diagnostics separately.
3. Compare named quantities such as tip displacement and support reactions against independently calculated analytical results. Stress can be compared only where the output selector and stress definition match the analytic reference assumptions.
4. Track equilibrium/reaction balance as a distinct metric; do not let displacement agreement conceal a force imbalance.
5. Report runtime, memory and solver warnings separately from accuracy and convergence. Do not collapse these into a single realism score.
6. For measured validation later, separate calibration from held-out specimens and account for measurement uncertainty. Mesh convergence is solution verification, not validation against reality.

## Claim ceiling and benchmark targets

- 100% correct frozen-contract disposition for malformed manifests, stale report digests, inconsistent report identities, non-finite metrics, incorrect units, incomplete observable inventory and unsupported levels.
- Every declared level remains in the report; failures and non-monotone trends are never silently dropped.
- A PASS only means the **finest level** passes the declared numerical reference comparator and any explicitly required error-trend gate for this named case and mesh family.
- The report always emits `physical_result_qualified = false` and `real_world_validation_status = UNRESOLVED`.

This module does not execute a solver, independently authenticate report authorship or chronology, determine that a mesh family is physically appropriate, establish an asymptotic convergence regime, validate material properties, compare against measured reality, or authorize engineering use.
