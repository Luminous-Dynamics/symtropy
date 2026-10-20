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
RUSTFLAGS="-Ctarget-cpu=haswell" cargo test --locked --manifest-path research/physical-analytical-adapter/Cargo.toml
RUSTFLAGS="-Ctarget-cpu=haswell" cargo clippy --locked --manifest-path research/physical-analytical-adapter/Cargo.toml --all-targets -- -D warnings
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


## Bounded analytical input uncertainty (API preview)

The [interval enclosure method](../../docs/research/PHYSICAL_ANALYTICAL_INTERVAL_METHOD_V0_1.md), tracked by [PHYS-UQ-001 #2292](https://github.com/Luminous-Dynamics/symtropy/issues/2292), is integrated into the public JSON request and receipt contract on the UQ branch. Each bound includes finite endpoints, an explicit SI unit, a provenance class, and a source locator. The receipt reports lower/nominal/upper and finite width, or leaves enclosures null when bounds were not supplied or propagation remains unresolved. Bounds are deterministic ranges, not statistical confidence intervals, and their declared provenance is not independently certified.


## Independent interval arithmetic oracle

The uncertainty module has a test-only dependency on **Inari 2.0.0** with default features disabled. Inari's documented x86-64 backend requires a Haswell-or-later target profile; the focused Ubuntu CI pins `RUSTFLAGS="-Ctarget-cpu=haswell"`. That CI configuration does not imply binaries built for older CPUs or other architectures are qualified. This is pinned in `Cargo.toml` and `Cargo.lock`; it is not a production/runtime dependency. See [PHYSICAL_INTERVAL_ORACLE_V0_1.md](../../docs/research/PHYSICAL_INTERVAL_ORACLE_V0_1.md) for its license/version, the pinned [ITF1788 framework revision](https://github.com/unageek/ITF1788/tree/d8c2a64478ebdc9cbde6ccef33eaad3bed60ed81), differential test scope, and limits.

For finite matching domains, Symtropy results must contain Inari's independent multiplication/division endpoints. The tests also compare positive powers built from interval products. Symtropy intentionally rejects operations that overflow or whose divisor contains zero; those are fail-closed API semantics and are not treated as a like-for-like IEEE interval result. This is a limited differential oracle, not an IEEE 1788 conformance claim.
