# CalculiX T3D2 axial-bar smoke benchmark v0.1

## Status and scope

This is the first planned invocation of an external FEM executable in the Symtropy physical-solver-contract path. It tests one tiny, exactly solvable axial bar with two nodes and one T3D2 truss element. The CI pins the Ubuntu 24.04 package version `calculix-ccx=2.21-1`, records the canonical solver executable SHA-256 and package inventory, and retains the request, scene, generated CalculiX input deck, raw solver files, normalized outputs, receipt and comparison reports.

This is only a smoke/code-verification case. It does not qualify CalculiX generally, the adapter for arbitrary geometry/loads, or any real engineering design. A single element and a hand-calculated exact solution cannot establish mesh convergence, model credibility, physical validity or intended-use safety.

## Narrow envelope

The adapter deliberately fails closed outside:
- exactly two scene nodes aligned along +x;
- one T3D2 element connecting the two nodes, with positive cross-sectional area;
- one isotropic linear-elastic material;
- one positive axial point force at the tip;
- three fixed translations at the root and a transverse guide at the tip (constraining y and z, allowing x);
- SI units (m, N, Pa) and only `static_linear_elastic` physics.

It generates a small `bar.inp` input deck rather than accepting user-supplied deck directives. This prevents arbitrary keywords, unsupported load types or unseen solver controls from silently expanding the v0.1 capability envelope.

## Closed-form reference

For the one-dimensional axial bar:

```text
L = 1 m
A = 1e-4 m^2
E = 2e11 Pa
F = +1000 N

u_tip_x = F * L / (E * A) = +5e-5 m
R_root_x = -F = -1000 N
sigma_x = F / A = +1e7 Pa
```

The current runner compares the normalized tip x-displacement and root x-reaction against independent static reference templates. It retains the analytical identities and tolerances in the reference manifests. The initial v0.1 adapter does not score stress and does not claim mesh-refinement results.

## Receipt/evidence flow

1. Bind the request to the exact scene bytes via `scene_sha256`.
2. Validate the request contract and the adapter-specific two-node/one-element envelope.
3. Generate the CCX input deck from the declared request and scene; record its exact SHA-256.
4. Run the pinned package executable with fixed solver arguments and single-thread environment settings.
5. Preserve stdout/stderr, `.dat`, `.sta`, input deck and package/binary fingerprints, including when the process exits unsuccessfully.
6. Parse node displacement and reaction-force tables only if the solver exits successfully and the required tables contain the named node IDs.
7. Emit normalized observations bound to the exact request bytes, scene digest/revision and case ID.
8. Emit a solver receipt that binds the scene, generated input deck and normalized outputs by SHA-256. Verify its schema/identity and every declared artifact byte.
9. Independently compare displacement and reaction against the frozen exact-reference templates.
10. Keep `physical_result_qualified=false` and `real_world_validation_status=UNRESOLVED` for all outcomes.

The binary hash, package version and configuration hash are separate fields. The exact request, scene and input deck are separately recorded as inputs, so the solver configuration fingerprint does not change just because a later eligible load-only case changes its force vector.

## Why CalculiX first

The official Ubuntu 24.04 package index currently lists `calculix-ccx 2.21-1`, making it possible to pin the installed package version on a named runner image rather than floating on `apt install latest`. The public CalculiX Examples project is MIT licensed and includes a T3D2 truss input pattern using `*SOLID SECTION` area and `*NODE PRINT` output. This repository authors an independent minimal deck and keeps the closed-form reference separate from the adapter.

References:
- Ubuntu 24.04 package entry: https://packages.ubuntu.com/noble/calculix-ccx
- Public CalculiX Examples (MIT): https://github.com/calculix/CalculiX-Examples
- CalculiX example demonstrating T3D2 and `*SOLID SECTION`: https://feacluster.com/CalculiX/ccx_2.18/doc/ccx/input_deck_viewer.php?input_deck=truss.inp
- NAFEMS standard benchmark programme for later independent case families: https://www.nafems.org/publications/resource_center/p18/

## Gate sequence after smoke

- **G0 environment:** pinned package, solver binary digest, build/runtime manifest and exact input.
- **G1 request/receipt:** exact scene and input-deck provenance, no malformed outputs, no false PASS on corrupted data.
- **G2 analytic:** displacement and reaction agree with closed-form expectations at preregistered tolerances.
- **G3 invariants:** add force and moment equilibrium where the model provides the required reaction moments; add load-scaling and superposition only with applicable preconditions.
- **G4 discretization:** select a 3+-level refinement family suitable for a continuum benchmark; a one-element T3D2 case cannot pass this gate.
- **G5 external benchmark:** add selected NAFEMS analytical cases with element/formulation assumptions checked against solver capabilities, then independent holdouts and cost-inclusive comparisons.

No gate above G2 is claimed by this smoke benchmark. The workflow is a positive milestone toward external qualification, not solver qualification itself.
