# Grid Transmission Portfolio Validator

A small standalone Rust CLI for semantic validation of the v0.1 route-portfolio contract.

It is intentionally separate from the Symtropy application workspace while the `symtropy-energy-systems` dependency chain is still on feature branches. It has no hardware, network, dispatch, protection or solver authority.

## Run

From the repository root:

```sh
cargo test --manifest-path tools/grid-transmission-portfolio-validator/Cargo.toml
cargo run --manifest-path tools/grid-transmission-portfolio-validator/Cargo.toml -- \
  docs/research/grid-transmission-portfolio/data/synthetic-route-portfolio-v0.1.json
```

Machine-readable report:

```sh
cargo run --manifest-path tools/grid-transmission-portfolio-validator/Cargo.toml -- \
  docs/research/grid-transmission-portfolio/data/synthetic-route-portfolio-v0.1.json --json
```

Exit codes: 0 = semantically valid, 1 = semantic violations, 2 = command/input/JSON error.

## What it checks

- Schema version and portfolio identity.
- Evidence catalogue identity and duplicate IDs.
- Evidence-state support: an analyst scenario cannot be promoted to measured, independently verified or field qualified; unknown claims cannot carry supporting evidence IDs.
- Maturity claims require evidence appropriate to the declared maturity.
- Typed service units and valid, non-overlapping time intervals.
- Unique route segment IDs and contiguous route sequence.
- Candidate identity and evidence state.
- Every design selects each route segment exactly once, in route order, and only chooses candidates declared for that segment.
- Source carrier matches the first selected segment.
- Every carrier change is bridged by an ordered, type-consistent transition chain.
- Final carrier matches the delivered-service carrier.
- Evidence references resolve and violations are emitted in stable order.

This is a semantic validator over JSON values. It is **not** a full JSON Schema Draft 2020-12 validator and does not validate electrical ratings, load flow, thermal limits, pipeline mechanics, protection, reliability, costs, site geology, environmental impacts, grid-code compliance or physical feasibility. Run a dedicated JSON Schema validator separately. The Rust tests exercise positive and adversarial semantic cases.

The fixture is synthetic. A validator PASS means only that the declared JSON contract is internally coherent under these rules; it does not mean the proposed route is physically viable or safe.
