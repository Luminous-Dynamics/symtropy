# Orbital Fluid Transfer Reference Benchmark v0.1

Standalone, Rust-first reference validator for Symtropy issue [#6178](https://github.com/Luminous-Dynamics/symtropy/issues/6178).

It tests strict fluid-class applicability, accepted receiver state, exact integer mass accounting and evidence/claim boundaries. It intentionally does **not** model CFD, phase change, heat transfer, pressure-vessel stress, cryogenic chilldown, boil-off dynamics, coupler mechanics or flight operations.

## Run

```sh
cargo fmt --manifest-path tools/orbital-fluid-transfer-bench/Cargo.toml -- --check
cargo test --manifest-path tools/orbital-fluid-transfer-bench/Cargo.toml
cargo clippy --manifest-path tools/orbital-fluid-transfer-bench/Cargo.toml --all-targets -- -D warnings
cargo run --quiet --manifest-path tools/orbital-fluid-transfer-bench/Cargo.toml -- \
  tools/orbital-fluid-transfer-bench/fixtures/v0.1/reference.json \
  --source-commit "$(git rev-parse HEAD)"
```

The receipt binds the exact fixture bytes by SHA-256 and the supplied 40-character source commit. It contains no wall-clock timestamp.

## V0 contracts

- Storable liquid, cryogenic liquid, and high-pressure/supercritical gas remain distinct profiles.
- A storage-only demonstration cannot be upgraded to a transfer claim.
- A transferred product is accepted only against declared final phase, temperature and pressure limits.
- Source inventory and every transfer/residual stream are reconciled in integer grams.
- Accepted receiver mass is never inferred from source tank mass loss alone.
- Unknown/contradictory evidence is Indeterminate or Rejected, never a default pass.
- Event identity and evidence references are required.

## Claim ceiling

A pass establishes only the software accounting/evidence contract for the synthetic fixture. It does not establish any real fluid transfer, cryogenic performance, heat leak/boil-off, coupler compatibility, pressure-vessel safety, materials compatibility, mission service, regulatory acceptance or flight qualification.


## Standards and applicability references

These are source references for later requirements/evidence profiles, not certification marks conferred by the v0.1 validator:

- **ANSI/AIAA S-157-2025**, *In-Space Storable Fluid Transfer for Prepared Spacecraft*: prepared-spacecraft design, verification and operation for storable (non-cryogenic) propellants and associated pressurants. Do not apply its scope to cryogenic fluid transfer without separate requirements and qualification. https://webstore.ansi.org/standards/aiaa/ansiaiaa1572025
- **ISO 14624-5:2023**, materials reactivity with aerospace propellants, applicable to materials used in storage, transfer and flight systems. https://www.iso.org/standard/67296.html
- **ECSS-E-ST-32-02C Rev.2 (15 October 2025)**, structural design and verification of pressurized hardware, including vessels, valves, pumps, lines, fittings and hoses. Its stated exclusions mean additional cleanliness/functional requirements still need explicit treatment. https://ecss.nl/standard/ecss-e-st-32-02c-rev-2-structural-design-and-verification-of-pressurized-hardware-15-november-2008/
- **ECSS-Q-ST-70-01C Rev.1 (15 October 2025)**, cleanliness and contamination control across design, production, test and operation. https://ecss.nl/standard/ecss-q-st-70-01c-cleanliness-and-contamination-control-15-october-2025/

The benchmark maps these to separate obligations: interface/conops, material compatibility, pressure-boundary integrity and cleanliness/contamination acceptance. No one standard covers the complete transfer service, and none of these references supplies a cryogenic boil-off or thermal-fluid model.
