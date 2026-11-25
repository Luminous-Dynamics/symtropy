# SAFRB cargo to generator to service coupling v0.1

Status: executable synthetic coupling; no field data  
Language: Rust 2024, no external dependencies  
Horizon: 72 one-hour intervals  
Stack: cargo receipt ledger + existing fuel-service corpus + service-vector qualifier

## Paradigm and responsibility boundary

Three independent contracts are composed without merging ownership:

1. Cargo ledger owns whether physical receipt, assay admission, custody/title/contract, access authority, route movement, generator delivery and time-indexed fuel issue are valid.
2. Fuel-service model owns the downstream synthetic fuel-to-electricity calculation, generator rating/derating, network delivery ratio, hourly demand, uncertainty and critical floor.
3. Service vector owns the non-compensatory evidence-interval verdict for every hourly service window.

The coupling is an adapter/reference harness. It is not a new EMS, dispatch optimizer, terminal inventory system, power-flow engine or legal authority service.

## Hourly model

For each interval, the adapter takes the quantity of generator fuel issued in that hour from the cargo ledger, then applies the existing fuel-service model's synthetic parameters:

- chemical energy input = issued grams × LHV (milli-Wh/g);
- electrical energy before rating cap = chemical energy × efficiency;
- gross generation = min(unconstrained generation, rated kW × derate × one hour);
- network delivery = gross generation × network-delivery basis points;
- delivered lower/upper interval applies the existing uncertainty basis points and is capped by hourly demand.

Fuel issued in the interval is treated as the generator's engine feed for that interval. Fuel merely received at the generator, but not issued, remains available stock and does not automatically count as generation. Any fuel-energy input above the one-hour generator power cap is reported as power-cap curtailed Wh, not silently counted as delivered service.

The energy predicate uses Wh. Average active power uses W over exactly 3600 seconds, so the value is recomputed as average W for that one-hour interval; the same numeric result is not an implicit unit substitution. Both are required. Voltage, frequency, reactive power, waveform quality and downstream process output are explicitly Not Applicable here with reasons, because this model does not generate those measurements. They must be supplied by their own network/beneficiary evidence tracks before a broader service claim is made.

## Evidence and maturity

The vector qualifier is exercised using digest-shaped, synthetic source and verifier-receipt identifiers. These are stable test identities only; they are not cryptographic hashes, authenticated evidence or physical measurements. The workflow's independent standard-library Python checker recomputes generation and service classifications from the emitted event/parameter data. Evidence maturity remains synthetic_fixture.

Planner and evaluator outputs are separate JSONL artifacts. Planner output contains cargo manifests known at hour zero, generator capability and service requirements. It excludes actual receipts, assays, custody, title, contract, authority, transfers, fuel issues, calculated generation and service verdicts.

## Cases and assertions

Twelve cargo-ledger cases × 72 one-hour service intervals = 864 hourly coupling records.

Tests include:
- no generation before generator fuel issue;
- late receipt/transfer does not backdate generator availability or service;
- missing custody cannot produce generator service;
- stale/conflicting telemetry remains indeterminate even when the generation model produces energy;
- incompatible generator fuel grade cannot generate;
- generator power cap is enforced and excess fuel energy is reported as curtailed;
- each hourly service verdict contains two required predicates and five scoped N/A predicates;
- planner/evaluator separation;
- agreement between the existing fuel-service three-way rule and the service-vector qualifier.

## Run

cargo fmt --manifest-path tools/safrb-cargo-service-coupling/Cargo.toml -- --check  
cargo test --manifest-path tools/safrb-cargo-service-coupling/Cargo.toml --locked  
cargo clippy --manifest-path tools/safrb-cargo-service-coupling/Cargo.toml --all-targets --locked -- -D warnings  
cargo run --manifest-path tools/safrb-cargo-service-coupling/Cargo.toml --release --locked -- --check  
cargo run --manifest-path tools/safrb-cargo-service-coupling/Cargo.toml --release --locked -- --emit target/safrb-cargo-service-coupling-v0.1

## Claim boundary

The coupling uses synthetic ledger cases and the existing synthetic fuel-service parameters. It does not establish real fuel availability, title, contract rights, cargo movements, terminal or pipeline capacity, generator performance, network power quality, beneficiary continuity, standards conformance, HIL readiness or SOTA superiority. In particular, a GUARANTEED result means only that the synthetic hourly interval satisfies the synthetic profile under the declared model; it is not a real-world service guarantee.

Next qualification tiers: independent source/evidence verification; three-phase AC and protection; topology and common-cause holdouts; software/controller-in-loop; HIL; commissioning; and periodic requalification.
