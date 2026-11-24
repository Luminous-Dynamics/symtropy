# SAFRB v0.7 — physical capacity versus accessible capacity

**Status:** standalone read-only constraint evaluator; synthetic fixtures only  
**Research cut:** 24 November 2026  
**Language:** Rust 2024, dependency-free  
**Parent:** SAFRB v0.6 multi-carrier service boundary

## Paradigm shift

A resource can be physically operable but inaccessible to a participant. It can also be contractually entitled but physically unavailable. A berth, tank, pipeline, terminal, power feeder or depot therefore has two independent realities:

- **Physical feasibility:** installed and operable capacity at the relevant resource/time boundary.
- **Access feasibility:** legal or contractual entitlement, provider offer, existing reservations/commitments and delivered use.

Neither reality implies the other. SAFRB v0.7 evaluates the intersection without deciding who should receive the resource or authorizing an effect.

## Capacity model

The evaluator computes:
1. Physical limit = min(installed capacity, operable capacity).
2. Access limit = min(physical limit, qualified entitlement, qualified provider offer).
3. Residual schedulable capacity = access limit − existing reservations/commitments/deliveries that consume the same resource and time window.
4. Candidate feasibility = whether the requested quantity is within that evidence-bounded residual.

It preserves claim IDs and counted allocation IDs in the result. Cancelled and expired reservations do not consume capacity. Duplicate allocation IDs are rejected even if their states differ; state transitions must be represented as one current authoritative state per allocation.

## Percentages are not capacities

An access percentage is meaningless without its exact denominator, resource scope, time period and evidence. For example, the reported 10%, 13% and 27.8% Island View Precinct statements from Parliament describe different policy/contract/future-phase claims. They must not be added or compared as though they share a denominator.

The share contract therefore requires:
- basis points;
- explicit denominator identity;
- numeric denominator quantity in the same physical unit;
- separately qualified denominator evidence;
- source-qualified access claim.

An unknown denominator is a hard block, not a zero or an inferred total. Share conversion rounds down to a whole physical unit so the computed cap cannot exceed the stated fraction.

## Evidence boundaries and ownership

- Physical installed/operable quantities require measured evidence with a digest reference, or an explicitly namespaced synthetic fixture.
- Access entitlements and provider offers require an upstream Mycelix-qualified authority receipt, or a synthetic fixture.
- Existing capacity-consuming allocations require a qualified authority receipt, or a synthetic fixture.
- This crate validates metadata and shape only. It does not verify source signatures, raw bytes, legal interpretation or credential revocation. An actual deployment must consume verified Mycelix outputs and preserve the verification receipt.
- Mycelix remains the owner of provider identity, access rights, offer, reservation, commitment and settlement authority.
- Symtropy owns physical capacity, operational envelopes and deterministic feasibility.
- Sol-Atlas provides spatial/time context.
- Symthaea may identify conflicts and request further evidence but does not authorize access.

## Deliberate limitations

- This evaluator does not allocate scarce capacity, rank participants or express fairness policy.
- It does not create reservations or provider commitments.
- It does not model fluid hydraulics, pipe pressure, pipeline batching, berth simulation or AC power flow. Such engineering limits must arrive as source-qualified physical capacity observations from the appropriate model/asset.
- Capacity is evaluated per exact resource ID and period ID; no hidden aggregation across resources, time or operators.
- Existing allocation records are a frozen snapshot. Concurrency control, atomic admission, expiry and revocation remain provider/Mycelix responsibilities.
- Synthetic values are not actual IVP/MPP capacity or legal entitlements.

## Run

```sh
cargo fmt --manifest-path Cargo.toml -- --check
cargo test --manifest-path Cargo.toml --locked
cargo clippy --manifest-path Cargo.toml --all-targets --locked -- -D warnings
cargo run --manifest-path Cargo.toml --release --locked
```

## Benchmark targets (future gates, not achieved results)

- Zero acceptance of reported-only or denominator-unknown access claims in the frozen negative suite.
- Zero overbooking acceptance in exact resource/time-window tests.
- Zero unit, scope or period mismatch accepted.
- 100% detection of duplicate allocation IDs and stale allocation identity mutations.
- Exact known-answer residual capacity arithmetic.
- Independent comparison with Mycelix's provider-authority verifier and concurrency/consume-once tests.
- No claim that capacity entitlement improves fairness or competition without a separate rights/access outcome benchmark.

## Claim ceiling

A passing run establishes the deterministic arithmetic of the read-only envelope for the supplied evidence classes and synthetic fixtures. It does not establish the physical capacity of a terminal, legality of a contract, actual access rights, market fairness, provider commitment, or authorization to execute a flow.
