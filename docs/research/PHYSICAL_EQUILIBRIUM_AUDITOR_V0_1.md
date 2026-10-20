# Receipt-bound static equilibrium auditor v0.1

**Implementation:** Rust evaluator in `research/physical-solver-contract/src/equilibrium.rs`.  
**Qualification authority:** [PHYS-INVARIANT-001 #2341](https://github.com/Luminous-Dynamics/symtropy/issues/2341) and [external FEM gate #2321](https://github.com/Luminous-Dynamics/symtropy/issues/2321).  
**Status:** synthetic contract/oracle tests only; no external FEM solver is executed by this module.

## Purpose

A scalar displacement comparator can miss a semantic error in reaction signs, units, force application points, reaction couples, output identity, or coordinate frames. This module adds an independent static-equilibrium invariant using applied loads from a frozen manifest and reaction forces/moments from exact receipt-declared output bytes.

The result is a necessary check for the declared balance statement, not evidence that a solver is correct, that a constitutive model is realistic, that all fields are mesh-adequate, or that a structure is safe.

## Scope and assumptions

Version 0.1 supports only:
- linear-static, explicitly declared point forces;
- SI length/force/moment units: m, N, N*m;
- one named coordinate frame with no implicit frame transform;
- force and reaction-force observations each with one position in the same coordinate frame;
- support reaction couples with explicit stable IDs;
- predeclared absolute and relative/scale-aware tolerances.

Distributed forces, pressure, body acceleration, follower loads, dynamics, thermal coupling, nonlinear materials, contacts and implicit transformations must remain outside the qualified envelope until a dedicated normalization and invariant model exists. A request with load types that cannot be independently reduced to the declared point-force representation is unresolved.

## Evidence flow

1. Parse the exact request and frozen equilibrium manifest.
2. Require manifest request digest, request ID, scene digest/revision and coordinate-frame ID to match the exact request.
3. Require every request load to be a point force; cross-check each load ID, target, frame and force vector with the request. The position and support/load couple interpretation is declared independently by the manifest's source reference.
4. Run the existing receipt and output artifact-byte verifier. Both reaction-force and reaction-moment outputs must be requested, present in the receipt, correctly typed and byte-verified beneath the canonical artifact root.
5. Re-read and hash the selected output bytes to detect changes after the initial verifier pass.
6. Bind each output to case ID, request ID/digest, scene digest/revision, coordinate frame and units.
7. Require the exact preregistered reaction-ID inventories. Missing, duplicate or unexpected rows are not filled with zeros.
8. Compute vector residuals, magnitude scales, allowed residuals and normalized residuals. Output both force and moment outcomes separately.
9. Preserve `physical_result_qualified=false` and `real_world_validation_status=UNRESOLVED` regardless of an equilibrium PASS.

## Equations

With all vectors in the declared global frame and origin (o):

```text
r_F = sum(F_applied) + sum(R_support)

r_M = sum((x_applied - o) × F_applied)
    + sum(C_applied)
    + sum((x_reaction - o) × R_support)
    + sum(M_reaction)
```

The v0.1 norm checks use:

```text
force_limit  = abs_force_tol  + rel_force_tol  * force_scale
moment_limit = abs_moment_tol + rel_moment_tol * moment_scale
```

The scale is the sum of the norms of the declared applied and reaction contributions for that dimension. The report includes the residual vector, residual norm, scale, relative normalized residual when defined, the exact tolerances and the allowed residual.

When scale is zero, the normalized residual is explicitly `undefined_zero_scale` rather than NaN or an invented zero. The absolute criterion still governs. Floating-point overflow or non-finite intermediate results produce `UNRESOLVED`.

## Coordinate-frame metamorphic invariant

The moment origin is arbitrary if every force application point uses the same coordinate frame. When the total force residual is zero, translating the moment reference origin must preserve the total moment residual:

```text
M(o + d) = M(o) - d × sum(F)
```

Therefore changing the origin alone is not a valid negative control for a balanced free body. v0.1 has a test that checks this invariance. Valid future metamorphic controls should rotate or translate all positions and forces consistently and preserve balance; invalid controls should deliberately rotate only one vector family, mislabel a frame, or alter a point without its corresponding transform. Proper frame transforms must use a right-handed orthonormal transform for both polar force/position vectors and axial moment/couple vectors.

## CLI

After building the pinned contract crate, run the audit against one request, one solver receipt and the exact artifact bundle:

```sh
cargo run --locked --manifest-path research/physical-solver-contract/Cargo.toml -- \
  --request ./run/request.json \
  --receipt ./run/receipt.json \
  --artifact-root ./run/artifacts \
  --equilibrium-manifest ./benchmarks/cantilever-equilibrium.json \
  --reaction-force-output-id reactions-force-v1 \
  --reaction-moment-output-id reactions-moment-v1
```

The receipt must declare:
- an output with the selected force-output ID and kind `reaction_force`, unit `N`;
- an output with the selected moment-output ID and kind `reaction_moment`, unit `N*m`;
- exact JSON artifact digests and relative paths for both files.

The request must require both output kinds. Receipt status and byte verification must both pass before the numerical audit is scoreable.

## Current synthetic test oracle

A transparent hand-calculated free body defines:
- a point force ((0,-1000,0)) N at ((2,0,0)) m;
- a support reaction ((0,1000,0)) N at the origin;
- a support couple ((0,0,2000)) N*m.

Expected balances are (r_F=(0,0,0)) N and (r_M=(0,0,0)) N*m. Negative controls reverse the reaction force (force residual magnitude 2000 N), omit the support couple (moment residual magnitude 2000 N*m), omit a required reaction ID (UNRESOLVED) and tamper with a receipt-bound output file (UNRESOLVED). These are synthetic unit tests, not solver-run results.

## Further tests before any external solver qualification

- frame rotation of all vectors versus rotating force vectors only;
- moment origin translation invariance and explicit output frame mismatch;
- zero force/moment scales and negative/zero tolerance controls;
- force in N vs kN with matching receipt digests but incorrect normalized output metadata;
- duplicate/extra/missing reaction identifiers;
- moment component sign, ordering and cross-product axis controls;
- extremely large but finite values that overflow a cross product or accumulation;
- stale scene/request binding, path traversal, symlink escape and time-of-check/time-of-use tampering;
- exact CLI positive and negative fixtures on the same pinned commit as format, unit tests and strict Clippy.

## Claim ceiling

The report proves only that exact receipt-bound reaction artifacts were parsed under the declared v0.1 schema and that their resulting force/moment residuals met specified arithmetic tolerances. It cannot prove that the load mapping is correct, the FEM solver truly produced the bytes, the continuum model represents reality, or the structure is safe. Physical validation remains a separate, measurement-backed gate.