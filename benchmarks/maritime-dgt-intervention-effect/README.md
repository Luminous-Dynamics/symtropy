# Maritime DGT Intervention Effect

A dependency-free Rust v0.1 evidence-lifecycle verifier for recovery measures that cross port, terminal, vessel, rail and inland handoffs.

This is the first executable slice of [MARITIME-DGT-INTERVENTION-EFFECT-001 #6578](https://github.com/Luminous-Dynamics/symtropy/issues/6578). It composes with, rather than duplicates:

- [DGT source admission #4003](https://github.com/Luminous-Dynamics/symtropy/issues/4003);
- [queue and recovery accounting #4007](https://github.com/Luminous-Dynamics/symtropy/issues/4007);
- [coastal-to-inland service chain #3683](https://github.com/Luminous-Dynamics/symtropy/issues/3683);
- [causal action-realization receipts #3272](https://github.com/Luminous-Dynamics/symtropy/issues/3272);
- [historical disruption replay #1671](https://github.com/Luminous-Dynamics/symtropy/issues/1671).

## Run

From this directory:

```sh
cargo fmt --all --check
cargo test --offline --locked
cargo run --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
```

## Current evidence ceiling

The frozen public-source fixture contains six statements from Transnet's 18 November 2026 DGT media statement:

https://www.transnet.net/getFile.ashx?id=34389

All six are recorded as **ANNOUNCED**. The CSV SHA-256 is recorded in `data/SHA256SUMS`; this digest covers the CSV fixture only. The underlying Transnet PDF bytes have not been independently acquired and hashed by this benchmark. See `data/manifest.json`.

In particular, an announcement that Berth 102 is available, a crane is handed over, or trains are available is not evidence that a vessel used that berth, a crane made productive moves, a train departed, or cargo arrived at its destination.

## Lifecycle contract

`ANNOUNCED → OFFERED_SCHEDULED → COMMITTED → AVAILABLE → ACTIVATED → REALIZED → DELIVERED`

Evidence types are stage-specific:

| Stage | Minimum evidence type | Required issuer role |
|---|---|---|
| ANNOUNCED | Public statement | Public source or provider |
| OFFERED_SCHEDULED | Provider schedule | Provider or authorized operator |
| COMMITTED | Authorized commitment | Authorized operator |
| AVAILABLE | Capability inspection | Authorized operator or independent observer |
| ACTIVATED | Operator activation | Authorized operator |
| REALIZED | Independent physical observation | Independent observer |
| DELIVERED | Independent handoff receipt | Receiving party or independent observer |

The v0.1 verifier enforces source/evidence IDs, resource identity, SHA-256 shape, issuer role, separate event/publication/receipt times, source-publication cutoffs, monotonic stage progression, idempotent assertion replay and conflict rejection. When an input starts at a later stage, skipped predecessors are reported as unobserved—not inferred.

A stage is an evidence-qualified claim, not a success score. Partial, failed, revoked and unknown outcomes must remain separate outcomes in the full register; they are not lifecycle stages. Unknown is the default whenever evidence is insufficient.

## Current tests

The authored tests cover public-announcement claim ceilings, unauthorized stage promotion, provider-versus-independent evidence, delivery handoff receipts, resource identity continuity, stage regression, exact and conflicting replay IDs, pre-event information cutoffs, future scheduled windows, unobserved lifecycle predecessors and digest/clock validity.

The DGT CSV fixture is real public-source wording normalized into a frozen claim register; the synthetic seven-stage sequence in the CLI and test suite is only a semantic demonstration. Neither is a measured intervention history, operational feed, causal estimate, or field validation.

## Next empirical gate

Before a real DGT claim can advance beyond ANNOUNCED, obtain rights-qualified provider records and independent evidence for resource ID, effective window, capability envelope, activation, actual physical output, cargo/visit identity and downstream receipt. For NCT diversions, reconcile the unique cargo unit through NCT discharge, coastwise transfer, Berth 102 / Point Terminal and final inland delivery exactly once. Count donor-terminal capacity displacement and any added feeder-vessel, port, rail, storage, cost and emissions burden.

No operational dispatch, berth booking, train path, cargo-priority or control authority is exposed by this crate.

 
## Candidate primary DGT operational snapshots (not ingested)

The source-admission investigation in [#4003](https://github.com/Luminous-Dynamics/symtropy/issues/4003) has identified three DGT/ICTSI-hosted PDF candidates:

- Vessel update stamped 21 November 2026 05:00: https://cdnweb-dgt.ictsi.com/users/user36/DGT%20VESSELS%20UPDATE%20%4005H00_3.pdf
- Firm 21-day berth plan stamped 20 November 2026 13:19:36: https://cdnweb-dgt.ictsi.com/users/user13/DGT%20Firm%2021%20day%20berth%20plan%2020.11.2026.pdf
- Export stack dates confirmed as of 20 November 2026: https://cdnweb-dgt.ictsi.com/users/user13/DGT%20EXPORTS%20STACK%20DATES%202026-11-20.pdf

See [source-register.md](./source-register.md) for the source field/state boundaries and admission gates. These files were inspected as PDFs but their bytes are not in this crate, have not been hashed here and have not been rights-cleared for reuse. The v0.1 fixture remains the six-row public-announcement CSV only. Do not convert DGT's raw WORKING, COMPLETED, NIT, FIRM, PROVISIONAL or CANCELLED values to physical port-call/cargo outcomes without a terminal field dictionary and independent evidence.
