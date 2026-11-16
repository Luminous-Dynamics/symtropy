# NH3-to-H2 service evaluator

Standalone, offline Rust evaluator for the synthetic contract in the Symtropy NH3-H2 process twin.

## Run from the repository root

cargo test --locked --manifest-path crates/tools/nh3-h2-service-evaluator/Cargo.toml

cargo run --locked --manifest-path crates/tools/nh3-h2-service-evaluator/Cargo.toml -- benchmarks/ammonia/nh3-h2-service-v0.json

cargo clippy --locked --manifest-path crates/tools/nh3-h2-service-evaluator/Cargo.toml --all-targets -- -D warnings

## What it checks

- NH3 feed, reaction, recycle, purge and slip mass closure.
- Stoichiometric consistency for 2 NH3 -> N2 + 3 H2.
- H2 generation, rejection, recovery and loss accounting.
- N2 recovery and purge accounting.
- Accepted H2 mass, purity, pressure, delivery interval and measurement identity.
- Complete non-overlapping grid-side meters for power conversion, conversion device, feed conditioning, gas handling, NH3 recycle, H2 separation, compression, thermal management, and controls/standby.
- Source-case and source-operating-point identity, including cross-pair refusal.
- Customer-boundary dynamic response, distinct from ACR/OES or plasma-state response.
- Comparability only when feed, product, energy boundary and duty-cycle contracts match.

## Deliberate claim ceiling

The evaluator accepts synthetic fixtures only. Its SYNTHETIC_ACCOUNTING_PASS and SYNTHETIC_COMPARABLE_ACCOUNTING_ONLY dispositions establish arithmetic and contract behavior only. They are not plasma or electrochemical simulations, measured H2 production, product purity, plant efficiency, cost, lifetime, safety qualification or operating authority.

The literature registry is source metadata only. Source-reported process points are not filled with synthetic plant data, and missing power, recycle, purity, compression or reliability fields remain NOT_ESTABLISHED. The evaluator does not authenticate external paper supplements, calibration certificates or plant metering artifacts.
