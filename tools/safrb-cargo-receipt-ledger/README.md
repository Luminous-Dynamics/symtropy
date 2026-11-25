# SAFRB time-indexed cargo receipt and availability ledger v0.1

**Status:** executable synthetic event-ledger contract; no field data  
**Language:** Rust 2024, no external dependencies  
**Horizon:** 72 service hours, half-open interval [0, 72)

## Paradigm shift

An ETA is not a receipt. A receipt is not admitted inventory. Admitted stock is not necessarily authorized. Authorized terminal stock is not yet delivered to the generator. Delivered fuel is not fuel issued to a generator.

This crate makes those states separate, time-indexed events:

1. Planner-visible manifest: planned quantity, declared ETA, product/grade, supplier, terminal and common-cause group, known at the observation cut-off.
2. Physical receipt: actual receipt time, batch identity, conservative lower/upper quantity, actual oracle quantity and receipt evidence state.
3. Quality admission: assay time, sample-chain reference, disposition and quantity released from the conservative receipt lower bound.
4. Custody/title/contract: independently evidenced custody window, quantity under custody, title disposition and contract state.
5. Access authority: a separate authorization window, encumbrance and maximum transferable quantity.
6. Terminal transfer: loading interval, dispatch, route capacity, transit delay, arrival, delivered mass and declared route loss.
7. Generator fuel issue: event linked to a specific transfer and occurring only after generator receipt.

The planner JSONL contains only manifest facts known at hour zero. Physical receipts, assay results, access decisions, transfer events, generator issues and hourly stock states are emitted only to the evaluator/oracle JSONL. The two outputs have separate checksums in CI.

## Accounting and temporal rules

- All quantities are integer grams; no implicit units or floating-point rounding.
- Receipt lower <= actual receipt <= receipt upper.
- Product, grade, terminal and cargo identity must match the manifest exactly.
- A verified receipt requires a source reference and digest. Missing or conflicting receipt evidence cannot be admitted.
- Only a passing assay with a sample-chain and evidence reference can release stock. Released quantity cannot exceed the conservative receipt lower bound.
- Custody/title/contract eligibility is independent from access authority. A transfer requires verified custody evidence, established title, an active contract, and a time window that covers loading/dispatch.
- Access authority is a separate event with a half-open validity interval [valid_from, valid_until). Withheld/conflicting authority grants zero transfer capacity.
- A transfer can start only after receipt and assay, while both custody/title/contract and authority are valid. Transfer quantity cannot exceed the quantity supported by custody, the authority maximum, or unencumbered admitted stock. The declared loading window is checked against route capacity per hour, including overlapping transfers.
- Hourly terminal-accessible stock is zero unless both independent custody/title/contract and authority gates hold.
- Route mass balance is source = delivered + declared loss; arrival cannot precede dispatch plus transit delay.
- Fuel issue must reference a known transfer, occur at or after arrival, and cannot exceed the transferred batch's delivered quantity.
- Event IDs are unique across event types. Duplicate/replayed events are rejected.
- Events at hour 72 are outside the service horizon. An event at hour h is effective at the start of hourly interval [h, h+1).
- Every hourly state reconciles admitted stock = terminal inventory + in-transit stock + generator delivery + route loss; generator delivery = generator available + fuel issued.

## Executable negative controls

Tests cover late arrival without backdating, exact horizon boundary, missing/conflicting receipt evidence, failed/pending assays, withheld authority, missing custody, unestablished title, inactive contract, duplicate/replayed event IDs, grade substitution, route throughput, transfer chronology, premature generator issue and interval-widening monotonicity.

## Run

```sh
cargo fmt --manifest-path tools/safrb-cargo-receipt-ledger/Cargo.toml -- --check
cargo test --manifest-path tools/safrb-cargo-receipt-ledger/Cargo.toml --locked
cargo clippy --manifest-path tools/safrb-cargo-receipt-ledger/Cargo.toml --all-targets --locked -- -D warnings
cargo run --manifest-path tools/safrb-cargo-receipt-ledger/Cargo.toml --release --locked -- --check
cargo run --manifest-path tools/safrb-cargo-receipt-ledger/Cargo.toml --release --locked -- --emit target/safrb-cargo-ledger-v0.1
```

## Claim boundary

This is an event-state and accounting reference model. Custody/title/contract dispositions and references are structural synthetic inputs, not legal determinations or cryptographic authentication. It does not model vessel/berth hydrodynamics, tank stratification, assay chemistry, pipeline hydraulics, pump transients, custody law, contract interpretation, refinery process dynamics, generator thermodynamics, power flow or beneficiary service physics. v0.1 fixtures cover refined diesel transferred from a synthetic terminal to a synthetic generator; crude-to-refinery conversion, multi-terminal routing and physical terminal constraints remain out of scope. It does not authenticate evidence bytes or signatures. All fixtures are synthetic. No South African cargo, SFF stock, terminal capability, contractual right, field service, standards conformance or SOTA superiority is claimed.

The next integration step is to bind the cargo ledger's hourly generator receipts to the existing SAFRB fuel-service corpus while keeping the latter's planner/oracle split and existing generator/network model as the downstream owner.
