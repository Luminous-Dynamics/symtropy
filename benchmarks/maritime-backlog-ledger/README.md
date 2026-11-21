# Maritime Backlog Ledger

A dependency-free, read-only Rust reference verifier for two conserved stock ledgers:

1. **Vessel-call queue** — counts of eligible vessel calls waiting for first berth entry or another declared exit.
2. **Container inventory** — physical container-box units held within a declared terminal boundary.

This implements the first slice of [MARITIME-BACKLOG-LEDGER-001 #6493](https://github.com/Luminous-Dynamics/symtropy/issues/6493), under [MARITIME-QUEUE-RECOVERY-001 #4007](https://github.com/Luminous-Dynamics/symtropy/issues/4007).

## Why stock–flow accounting comes first

Throughput is a flow; congestion is a stock. An increase in moves over a reporting interval does not prove that the vessel queue or container inventory is shrinking. Before building or comparing recovery optimizers, we need a ledger that can demonstrate where each unit entered, left, was corrected, or remained unaccounted for.

## Run

From this directory:

```bash
sha256sum --check data/SHA256SUMS
cargo fmt --all --check
cargo test --offline --locked
cargo run --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
```

The package has no third-party dependencies and is isolated by its own Cargo workspace. The 32-case frozen synthetic corpus contains 16 ACCEPT and 16 REJECT expectations.

## Explicit unit boundaries

| Ledger | Unit | What is counted |
|---|---|---|
| Vessel queue | `VESSEL_CALL` | One eligible vessel call in the declared queue population |
| Container stock | `PHYSICAL_BOX` | One physical container box; a 40-foot box remains one box |

This package does not convert physical boxes to TEU-equivalent, moves, tonnes or train paths. A rehandle is a handling move but has zero net container-stock delta.

## Source/event semantics

- Event time and observation time are separate.
- A case has an explicit historical observation cutoff.
- Only records observed by that cutoff participate in reconstruction.
- Exact duplicate source-record replays are idempotent.
- Reusing a source-record ID with different content rejects.
- For a canonical event ID, a visible revision supersedes—not adds to—its earlier revisions.
- Revisions must start at one and be contiguous; source, terminal and stock-unit identity remain stable across revisions.
- A correction marked `VOIDED` removes the event's prior stock effect.
- Planned, requested and estimated observations cannot mutate actual stock.
- Terminal identity and quantity units must match the case.
- Actual events must be inside the declared window and cannot be observed before they occur.
- Intermediate stock may never become negative at a declared time-bucket boundary. Events sharing one tick are aggregated atomically; no lexical event-ID order is invented.
- The final calculated stock must equal the reported closing stock.

Ticks in the synthetic corpus are integer simulation ticks. They are not UTC timestamps, civil timestamps, or a timezone conversion implementation.

## Claim ceiling

Passing this corpus demonstrates only the verifier's synthetic event/revision/stock-flow semantics. It does not authenticate provider data, establish data rights, measure a real terminal queue, infer real vessel waiting time, validate berth or yard capacity, prove causality, or authorize berth booking, dispatch, navigation or cargo release.
