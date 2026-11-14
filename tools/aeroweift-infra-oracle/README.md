# AeroWeft Infrastructure Recovery Oracle — Synthetic Slice 0.1

This dependency-free Rust workspace is a tiny exhaustive oracle for the infrastructure-resilience portfolio tracked by Symtropy issue #5224.

Golden fixture:
fixtures/aeroweift-infrastructure-resilience/tiny-portfolio-v0.json

## What it evaluates

- Two nominal north recovery sites that share three declared failure domains.
- One south recovery site with different weather, power and ground-access dependencies.
- Distinct landing availability versus complete repeat-service availability.
- Exhaustive enumeration of every subset of the six declared failure domains.
- Exact inclusion-minimal failure sets for each selected portfolio.
- Fail-closed behavior for a site whose operational status is unknown.
- A compound outage that disables both sites in the otherwise recovery-diverse portfolio.
- A monotonicity control: adding failures cannot restore a complete chain.

The fixture-local result is:

- North A+B: three minimal singleton cut sets (coastal weather, north power feeder, north bridge).
- North A+South C: nine minimal two-domain cut sets.
- A+South C survives every declared single-domain outage.
- The declared coastal-weather plus south-power compound outage disables both complete service chains.
- A geometry-only unknown-status site never counts as usable.

These are exact graph properties of the frozen fictional fixture, not real-world reliability probabilities or engineering qualification.

## Deliberate limitations

This is not an investment optimizer, site-selection product, power/microgrid solver, aircraft model, route scheduler, generic reliability package, live data adapter, airport approval system or operational flight-recovery service.

The oracle assumes the fixture's declared dependency completeness and monotone failure semantics. It does not validate that real infrastructure dependencies have been discovered or that real outage causes are independent. Unknown dependency inventory must remain incomplete outside the closed synthetic fixture.

## Exact-head commands

cargo fmt --manifest-path tools/aeroweift-infra-oracle/Cargo.toml -- --check

cargo clippy --locked --manifest-path tools/aeroweift-infra-oracle/Cargo.toml --all-targets -- -D warnings

cargo test --locked --manifest-path tools/aeroweift-infra-oracle/Cargo.toml

Source and tests are authored; no test pass is claimed until the exact-head workflow completes.
