# Physical Analytical Adapter v0.1

**Tracking:** [PHYS-ANALYTICAL-001 #2284](https://github.com/Luminous-Dynamics/symtropy/issues/2284)  
**Status:** research implementation; exact-head CI qualification required.

## What it does

This adapter invokes the core reference kernel for three idealized closed-form cases and includes one explicit reduced-matrix interval model:

1. **Axial bar:** constant cross-section, uniform axial load, static linear elasticity.
2. **Cantilever tip load:** small-deflection Euler-Bernoulli beam with constant section and point load at the free tip.
3. **Simply supported uniform load:** small-deflection Euler-Bernoulli prismatic beam under a uniform load over the full span.
4. **Asymmetric two-bar planar truss:** two restrained base supports and a free apex, with point loads and axial member stiffnesses. This case uses a reduced 2-DOF 2x2 stiffness system and is not a continuum FEM model.

The first three cases reuse the implementation at `research/physical-transformation-reference/src/mechanics.rs`; their formulas are not copied into a second mechanics implementation. The planar-truss model has its own explicitly defined reduced-stiffness oracle in `src/uncertainty.rs`.

## Request and receipt

Request schema: [`physical-analytical-request-v0.1.schema.json`](../../schemas/physical-transformation/physical-analytical-request-v0.1.schema.json). The fixtures are under `schemas/physical-transformation/analytical-fixtures/`.

Each request binds a stable request ID, source revision, case ID, exact assumption profile, SI units, requested physics and case-specific numeric inputs. Each JSON receipt binds the SHA-256 of the exact request bytes, source revision, adapter revision, output units, pinned acceptance tolerances, diagnostics and claim ceiling.

A well-formed request that includes unsupported physics (fracture, dynamic response, contact or thermal coupling) returns `UNRESOLVED` with no output rather than silently evaluating only the linear formula and claiming success. Malformed contracts are rejected. Finite-but-out-of-domain values that overflow the formula produce an `UNRESOLVED` receipt.

## Run

```sh
cargo fmt --manifest-path research/physical-analytical-adapter/Cargo.toml --all -- --check
cargo test --locked --manifest-path research/physical-analytical-adapter/Cargo.toml
cargo clippy --locked --manifest-path research/physical-analytical-adapter/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path research/physical-analytical-adapter/Cargo.toml -- \
  --request schemas/physical-transformation/analytical-fixtures/analytical-bar-v0.1.json \
  --adapter-revision "$(git rev-parse HEAD)"
```

The focused workflow runs all three supported requests and one unsupported-fracture control at the exact checkout commit, asserts the output domains, and uploads deterministic receipts.

## Evidence and claim limits

- `PASS` means the declared closed-form analytical formula evaluated successfully under its named assumption profile.
- `UNRESOLVED` means the requested physics is unsupported or the formula could not be evaluated in its finite/domain envelope.
- `analytical_reference` means a reproducible formula calculation, not measured evidence.

These formulas do not prove that a real member has the declared geometry, material properties, boundary conditions or load. They do not validate real connection behavior, nonlinear material response, fracture initiation/propagation, dynamic collapse, contact, soil interaction, construction-code compliance, structural stability or demolition safety. A passing receipt is not construction authorization or machine-control approval.

## Bounded analytical input uncertainty: planar truss

The bounded `planar_truss_two_bar` case propagates declared span, apex position/height, member axial rigidities `EA` (units N) and apex load bounds through the reduced 2x2 stiffness matrix. Effective member stiffnesses `EA/L` are derived from each geometry interval rather than treated as independent of uncertain length. It publishes interval enclosures for the matrix entries, determinant, bar lengths, displacements and condition indicators.

The determinant uses the geometry-specific identity

```text
det(K) = kA * kB * (span * height / (LA * LB))^2
```

rather than `Kxx*Kyy - Kxy*Kxy`, which can lose correlation and become uninformative near collinearity. If the determinant cannot be proven strictly positive, or the point calculation underflows / becomes non-finite, the receipt is `UNRESOLVED` with no outputs or finite output enclosures.

The current workflow exercises two fixtures: a finite narrow interval box which must contain the nominal solution, and a near-mechanism case in the subnormal height range which must abstain. The nominal bounds are explicitly tagged as `scenario_bound`; no probability distribution, calibration status, or physical validity is inferred. The corner sweep is a falsification aid rather than a proof of interval soundness; the interval-conformance work remains an independent dependency.

## Next gates

1. Complete exact-head formatting, tests, strict Clippy and receipt checks.
2. Add request-schema conformance checks in CI, beyond JSON syntax.
3. Add a solver dispatcher with an explicit allowlisted capability manifest.
4. Qualify one specialist solver using the same receipt contract and an independently reproduced known-answer case.
5. Validate against a rights-cleared measured dataset before claiming physical accuracy or state-of-the-art improvement.


## Industry comparison and next qualification gates

See [PHYSICAL_ANALYTICAL_VV_ROADMAP_V0_1.md](../../docs/research/PHYSICAL_ANALYTICAL_VV_ROADMAP_V0_1.md) for the matched comparison ladder against the published SOFiSTiK 2026 verification catalog, NIST tiered verification/validation/calibration guidance, and the open 2026 synthetic SHM robustness benchmark. The current adapter is at the closed-form equation-contract tier only; numerical solver verification, measured validation, and uncertainty qualification remain separate gates.


## Bounded analytical input uncertainty (API preview)

The [interval enclosure method](../../docs/research/PHYSICAL_ANALYTICAL_INTERVAL_METHOD_V0_1.md), tracked by [PHYS-UQ-001 #2292](https://github.com/Luminous-Dynamics/symtropy/issues/2292), is integrated into the public JSON request and receipt contract on the UQ branch. Each bound includes finite endpoints, an explicit SI unit, a provenance class, and a source locator. The receipt reports lower/nominal/upper and finite width, or leaves enclosures null when bounds were not supplied or propagation remains unresolved. Bounds are deterministic ranges, not statistical confidence intervals, and their declared provenance is not independently certified.
