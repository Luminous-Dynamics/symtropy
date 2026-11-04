# Aerodynamic applicability-domain contract v0.1

## Purpose and evidence ceiling

An aerodynamic prediction is not validated for every geometry and operating point merely because it was compared with experiments somewhere.

This contract makes the declared support region explicit and replayable. It is deliberately narrower than a complete aerodynamic domain-of-validity theorem. It implements a deterministic union-of-regions membership check for one exact model, quantity, geometry family, flow regime, solver identity and output definition.

A status of `supported_within_declared_domain` means only that the request lies in one declared dense-support region whose supporting evidence references are bound to that region. It does not prove that the source is authentic, that the evidence is adequate, that the solver is correct, or that the prediction is physically accurate.

## Paradigm shift

Do not use:

```text
one model ID + one validated=true flag
```

Use:

```text
exact model and quantity identity
+ explicit geometry family and flow regime
+ exact feature names and units
+ union of non-convex support regions
+ supporting evidence artifacts and exact scope digests
+ an explicit sparse/boundary/unknown/out-of-domain result
```

This v0.1 contract rejects implicit unit conversion, missing-feature imputation, silent solver-version reuse, convex-hull gap filling, and favourable selection among overlapping regions.

## Manifest

A manifest binds:

- model ID and quantity identity/unit;
- solver executable/build identity digest;
- output-definition digest (quantity, convention, normalization and extraction semantics);
- feature schema (feature name and exact unit);
- one or more support regions;
- supporting evidence references.

Each evidence reference binds an evidence ID/kind, source reference, license/applicability note, artifact SHA-256 and scope SHA-256. Each region references one or more evidence IDs.

The scope digest is recomputed by the implementation from the exact model, quantity, solver, output definition, geometry family, flow regime and all region feature bounds. A caller cannot take a digest scoped to one operating region and attach it to another region.

The source reference and artifact digest are identity/binding inputs only. No signature, trust-root authorization, source authenticity, physical truth or independent scientific adequacy is inferred.

## Regions and membership

Each region requires:
- exact geometry-family label;
- exact flow-regime label;
- a rationale;
- one inclusive finite interval for every declared feature;
- one or more supporting evidence IDs.

The region set is an explicit union. Points inside a convex hull but outside all declared regions remain out of domain. The implementation does not infer intermediate coverage.

Support classes are:
- `dense`: declared dense support;
- `sparse`: declared support with sparse observations;
- `boundary_or_regime_transition`: a boundary/transition region that requires separate review before claim promotion.

The support class is a supplied, evidence-bound classification, not something this primitive estimates statistically. In particular, boundary/transition status is not automatically eligible for claim admission.

Intervals are inclusive and all feature units must match exactly. v0.1 applies no scale transform or unit conversion. A future version may add frozen transforms only with versioned, independently tested conversion semantics.

If more than one region matches, the result is `unknown_due_to_missing_or_unqualified_inputs`. The system must not choose the most favourable support class. If no region matches the exact geometry family and flow regime, or the point falls in a gap between regions, the result is `out_of_domain`.

## Query result

The typed result is one of:

- `supported_within_declared_domain`
- `supported_but_sparse`
- `boundary_or_regime_transition`
- `out_of_domain`
- `unknown_due_to_missing_or_unqualified_inputs`

The assessment binds canonical digests of the manifest and query, matched region IDs, status, reason and evidence ceiling.

Missing/extra features, duplicate features, wrong units, non-finite inputs, model/quantity mismatch, and solver/output identity mismatch produce Unknown. These are not converted into out-of-domain claims because the requested comparison is not well-defined.

## Explicit non-claims

This primitive does not implement:
- convex-hull, Mahalanobis, nearest-neighbour, density, one-class or learned OOD scoring;
- uncertainty calibration, local interpolation error, model-discrepancy estimation or confidence bounds;
- validation-data acquisition or experiment value-of-information;
- authenticated provenance, signatures, source trust, calibration authority or verifier trust;
- a flight envelope, airworthiness conclusion, certification, or release authority.

A dense support label must be justified by its referenced evidence and reviewed under the risk-proportionate credibility policy. No finite software test proves future physical coverage.

## Initial adversarial corpus

The Rust module includes synthetic fixtures for:
- disconnected support regions with a query in the convex-hull gap;
- dense, sparse and boundary statuses;
- missing features, wrong units and changed solver identity;
- overlapping regions (must return Unknown);
- changed geometry-family and flow-regime categories;
- row-order invariant manifest/query identity;
- tampered manifests and altered queries;
- evidence references missing from a region;
- region evidence with a mismatched scope digest;
- orphan evidence references not used by any region.

All current examples are synthetic contract tests. They are not NASA TMR, DPW-8, AePW-4 or physical-validation results.

## Advancement benchmark

The initial Rust implementation now has an independent Python stdlib oracle at `scripts/verify-aero-applicability-independent.py`. The oracle reconstructs the canonical manifest, evidence-region scope digests, query digest, membership decision and assessment digest from normalized CSV exports; a Rust test invokes it on a synthetic known-answer fixture. This is independent language implementation/replay, not independent physical data validation.

Before using this as a production claim-admission control:
1. Add exact source-admitted NASA TMR / DPW-8 / AePW-4 data cases and record their licensing/applicability limits.
2. Freeze blind geometry-family and operating-condition holdouts.
3. Report false-in-domain and false-out-of-domain rates, risk-coverage curves and subgroup results by geometry family and flow regime.
4. Report independent sample counts and confidence bounds; zero errors on a small synthetic corpus is only a finite software test result.
5. Compose the assessment into claim admission so OutOfDomain, Unknown and Boundary/Transition cannot silently be promoted for high-consequence uses.

## Relationship to related work

- #3495: AERO-APPLICABILITY-001 (parent contract).
- #3413: protocol-aware SU2 run evidence and claim-specific admission.
- #3428: risk-aware model credibility.
- #2890: decision-value experiment selection under a fixed budget.
- #3494: source-built SU2 discrete-adjoint baseline qualification.
- #3498: measured solver resource telemetry.

