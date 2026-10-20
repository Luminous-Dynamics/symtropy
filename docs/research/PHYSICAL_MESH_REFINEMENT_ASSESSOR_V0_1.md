# Physical mesh-refinement assessor v0.1

**Implementation owner:** [PHYS-EXTQUAL-001 #2321](https://github.com/Luminous-Dynamics/symtropy/issues/2321)  
**Composes with:** [output byte verifier #2307](https://github.com/Luminous-Dynamics/symtropy/pull/2307), [numerical reference comparator #2312](https://github.com/Luminous-Dynamics/symtropy/pull/2312)  
**Status:** Rust evaluator plus synthetic test fixtures. No external FEM solver is invoked here.

## Paradigm shift

Do not accept a single close-looking answer as evidence that a finite-element solution is mesh-converged. Require a traceable sequence of at least three systematically refined meshes; verify each mesh, request, receipt and normalized output; analyze the actual quantity of interest; and keep discretization diagnostics separate from absolute reference agreement, physical validation and intended-use approval.

This follows established V&V practice, not a claim of a new convergence theory. NAFEMS' standard benchmarks were developed as independent cross-code tests and use analytical target values where feasible ([NAFEMS P18](https://www.nafems.org/publications/resource_center/p18/)). Richardson extrapolation and the Grid Convergence Index (GCI) provide a widely used framework for reporting mesh-refinement uncertainty under their assumptions (Roache, *Journal of Fluids Engineering*, 1994, [doi:10.1115/1.2910291](https://doi.org/10.1115/1.2910291)). FEM-specific solution-uncertainty methods can also consider degrees of freedom, element choice and asymptotic behavior ([NIST, 2018](https://www.nist.gov/publications/finite-element-method-solution-uncertainty-asymptotic-solution-and-new-approach)).

## Manifest and evidence inputs

    {
      "schema_version": "physical-mesh-refinement-study-v0.1",
      "study_id": "cantilever-tip-deflection",
      "case_id": "linear-cantilever-reference",
      "observable_id": "tip_deflection_m",
      "unit": "m",
      "dimension": 3,
      "trend_expectation": "monotone_expected",
      "refinement_ratio_relative_tolerance": 0.05,
      "expected_order_min": null,
      "reference": {
        "value": 0.001,
        "unit": "m",
        "method": "analytical_exact",
        "identity": "closed-form Euler-Bernoulli reference",
        "source_ref": "repo://frozen-known-answer/cantilever-v1"
      },
      "acceptance": {
        "max_fine_absolute_error": 0.00001,
        "max_fine_relative_error": 0.02
      },
      "levels": [
        {
          "level_id": "coarse",
          "h_m": 0.04,
          "element_count": 100,
          "dof_count": 900,
          "mesh_sha256": "64 lowercase hexadecimal characters",
          "mesh_ref": "levels/coarse/mesh.inp",
          "request_ref": "levels/coarse/request.json",
          "receipt_ref": "levels/coarse/receipt.json",
          "output_id": "normalized"
        }
      ]
    }

The example shows the manifest shape, not a valid complete three-level study or benchmark result. A real manifest must list at least three levels, each ordered coarse-to-fine, with strictly decreasing characteristic mesh size and increasing element and degree-of-freedom counts. v0.1 supports one scalar quantity of interest per study and 3–10 levels.

Every level:
- hashes the actual mesh bytes incrementally and compares against mesh_sha256;
- reads the exact request/receipt under the study root;
- requires receipt consistency, producer PASS, solver-reported convergence and verified output artifact bytes;
- checks the normalized observation against exact request ID, request digest, scene digest/revision, case ID, quantity ID and unit;
- requires a stable source revision and matching adapter/build/configuration identity across levels.

Paths must remain below the canonical study root. Unsupported, malformed, missing or changed evidence results in a non-PASS/unresolved study, not a default zero. The report retains per-level request, mesh, receipt and output digests.

## Numerical method

All levels must use one exact observable and unit. For each level i, the assessor reports absolute error against the declared reference:

    e_i = |phi_i - phi_ref|

Relative error is reported only when the reference is nonzero. It is not forced to zero around a zero reference.

The implementation first calculates refinement ratios r_i = h_i / h_(i+1). v0.1 estimates apparent order only when those ratios are sufficiently uniform under the manifest's predeclared spread tolerance, and the finest three solutions form a monotone, contracting sequence. For that three-level, near-uniform-ratio case:

    p = ln(|phi_c - phi_m| / |phi_m - phi_f|) / ln(r)

If the preconditions fail, apparent order and extrapolation are withheld. Oscillatory/divergent data are reported as FAIL when monotone convergence was declared expected, otherwise as UNRESOLVED with diagnostics. The evaluator does not assume that every quantity converges monotonically.

Under these assumptions, the evaluator reports a conditional Richardson extrapolation and a GCI-like estimate using a 1.25 safety factor for the three-grid case. It explicitly labels the estimate as conditional; it is not a validated uncertainty distribution, confidence interval, physical uncertainty estimate or safety factor for a design. Local stress singularities, contact transitions, mixed element orders and non-asymptotic meshes can violate the assumptions and must not be hidden by the metric.

Fine-mesh accuracy acceptance is reported independently:
- PASS / FAIL only when the manifest preregisters the relevant absolute and/or relative error limit;
- NOT_SPECIFIED where no fine-mesh limit was specified;
- UNRESOLVED where the requested relative-error test is undefined.

A mesh-convergence PASS never upgrades physical_result_qualified; real-world validation remains UNRESOLVED.

## CLI

    cargo run --locked --manifest-path research/physical-solver-contract/Cargo.toml -- \
      --mesh-study study.json \
      --artifact-root ./study-bundle

The artifact root must contain every manifest-referenced mesh/request/receipt and every artifact referenced by those receipts. Each file path is resolved below the canonical root.

## Initial gates and stretch targets

Software contract targets:
- zero false PASS across malformed paths, wrong mesh digest, stale/tampered normalized bytes, invalid receipt, non-converged producer, identity drift and unit mismatch controls;
- exact report bytes for repeated same-input runs in a pinned toolchain;
- all levels' exact artifact digests and adapter identity retained in the report;
- no apparent order or GCI emitted for unsupported/oscillatory/non-contracting studies.

External FEM qualification targets (not results):
- at least three independently generated meshes of the same declared model, with mesh input digests, element/DOF counts and a preregistered quantity-of-interest definition;
- displacement, stress/resultant and reaction balance where meaningful;
- compare against independently derived analytic reference values;
- report each level and every failed/unresolved solve; do not discard coarse, divergent or malformed runs;
- report wall time and peak memory in a separate performance table, never as an accuracy substitute;
- later compare selected permissible NAFEMS cases only after provenance/license review.

## Claim ceiling

A report can establish that the input artifacts matched their declared digests and that the series satisfied the numerical diagnostics declared by this v0.1 method. It does not prove the external solver's implementation is correct, that the analytic reference is independently authored, that the continuum model represents reality, that the mesh is adequate for all quantities, or that a component is safe to build, demolish or operate. Freeze reference values, quantity definitions, thresholds and study manifests in reviewed source control before running the candidate solver; a digest alone cannot prove preregistration chronology.
