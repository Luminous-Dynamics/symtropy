# Physical Analytical Adapter v0.1

**Tracking:** [PHYS-ANALYTICAL-001 #2284](https://github.com/Luminous-Dynamics/symtropy/issues/2284)  
**Status:** research implementation; exact-head CI qualification required.

## What it does

This adapter invokes the existing functions in the core reference kernel for three idealized closed-form cases:

1. **Axial bar:** constant cross-section, uniform axial load, static linear elasticity.
2. **Cantilever tip load:** small-deflection Euler-Bernoulli beam with constant section and point load at the free tip.
3. **Simply supported uniform load:** small-deflection Euler-Bernoulli prismatic beam under a uniform load over the full span.

It reuses the implementation at `research/physical-transformation-reference/src/mechanics.rs`; the formulas are not copied into a second mechanics implementation.

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

## Next gates

1. Complete exact-head formatting, tests, strict Clippy and receipt checks.
2. Add request-schema conformance checks in CI, beyond JSON syntax.
3. Add a solver dispatcher with an explicit allowlisted capability manifest.
4. Qualify one specialist solver using the same receipt contract and an independently reproduced known-answer case.
5. Validate against a rights-cleared measured dataset before claiming physical accuracy or state-of-the-art improvement.


## Industry comparison and next qualification gates

See [PHYSICAL_ANALYTICAL_VV_ROADMAP_V0_1.md](../../docs/research/PHYSICAL_ANALYTICAL_VV_ROADMAP_V0_1.md) for the matched comparison ladder against the published SOFiSTiK 2026 verification catalog, NIST tiered verification/validation/calibration guidance, and the open 2026 synthetic SHM robustness benchmark. The current adapter is at the closed-form equation-contract tier only; numerical solver verification, measured validation, and uncertainty qualification remain separate gates.


## Bounded analytical input uncertainty v0.2

The bounded-input contract is introduced in `physical-analytical-request-v0.2` and `physical-analytical-receipt-v0.2`, tracked by [PHYS-UQ-001 #2292](https://github.com/Luminous-Dynamics/symtropy/issues/2292). Point-only v0.1 requests keep their existing receipt shape. V0.2 callers may supply `input_bounds` with a lower/upper value, unit, evidence class and source reference for every model input. When bounds are supplied, the receipt carries per-output lower/nominal/upper values; when absent, interval fields are null and no bounds are invented.

The underlying [interval enclosure method](../../docs/research/PHYSICAL_ANALYTICAL_INTERVAL_METHOD_V0_1.md) uses outward-rounded arithmetic and rejects malformed ranges, nominal inputs outside bounds, invalid denominator ranges and non-finite calculations. These are deterministic enclosures conditional on declared inputs, not statistical confidence intervals or claims about omitted measurement/model-form uncertainty.


## Bounded-enclosure stress tests v0.1

The interval module now adds two supplementary checks beyond exact endpoints:

- A deterministic xorshift64* stream samples 4,096 interior input points for each of the three supported case families (12,288 total point evaluations) and asserts every output stays inside the computed interval.
- A load-scaling metamorphic test doubles the entire positive load interval for each model and checks that every lower and upper output endpoint doubles within a small floating-point tolerance.

These tests are deliberately deterministic, dependency-free and reproducible. They are **not** a proof that the interval implementation encloses all mathematical values; the exact formula structure, outward rounding, known-answer tests and endpoint/zero-crossing controls remain necessary. Sampled coverage must never be described as a physical validation rate or confidence level.
