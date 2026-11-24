# Symtropy Mobility Network Delivery Adapter

A read-only composition layer between mobility reserve/commitment evidence and the existing symtropy-dc-network-screen DC solver.

## Purpose

A mobility service offer is not automatically deliverable at the grid constraint merely because its local reserve/PCC arithmetic passes. The adapter pins the offer to its exact PCC bus, requires upstream gate receipts, and delegates the network equations and contingency screening to the existing DC-network crate.

It reports three different things without collapsing them:
- site-requested PCC trajectory volume (MW/MWh);
- maximum response that the DC network screen can accommodate under the declared branch/topology constraints (MW/MWh);
- adequacy contribution, which is not calculated here and remains a separate PRAS/ELCC study.

The input includes a read-only PCC baseline/metrology receipt, but the adapter does not claim an actual post-event meter response. Actual measured delivery requires later meter evidence.

## Mandatory upstream evidence

Exactly one receipt is required for each:
1. mobility flexibility feasibility;
2. reserve recovery;
3. commitment lease admission;
4. commitment journal reconciliation;
5. PCC baseline/metrology.

All receipts must match the exact offer, commitment epoch, site and PCC. The four resource/commitment gates must also match the exact asset and configuration. PCC metrology must bind the baseline and offered trajectory digests. The flexibility receipt must pin the exact network model and topology digests.

The adapter checks digest shape, uniqueness, identity and evidence validity intervals. It does not authenticate producers, verify signatures, or prove that a source generated the stated digest. A missing/mismatched/stale receipt is rejected; a failed receipt is rejected; an unknown receipt holds the offer without calling the DC screen.

## Run

    cargo run -p symtropy-mobility-network-adapter --bin mobility_network_delivery_screen -- crates/domains/symtropy-mobility-network-adapter/fixtures/mobility_network_adapter_v0_1.json

Test and lint:

    cargo test -p symtropy-mobility-network-adapter --locked
    cargo clippy -p symtropy-mobility-network-adapter --all-targets -- -D warnings

## Frozen known-truth fixture

The fixture uses the synthetic three-bus triangle from GRID-LOC-DELIV-001:
- Requested at PCC bus B: 30 MW for one hour (30 MWh).
- Existing DC network screen maximum deliverable response: 17.5 MW (17.5 MWh).
- Undeliverable/curtailed remainder: 12.5 MWh.
- Energy conservation: requested = network-deliverable + undeliverable.

This is deliberately a partial-deliverability case, not an optimizer or a dispatch recommendation. All gate receipts and network inputs are synthetic. The fixture is not field evidence.

## Claim ceiling

This crate delegates active-power DC screening only. It is not AC voltage/reactive validation, protection coordination, RMS/EMT stability, distribution hosting-capacity approval, ELCC/PRAS adequacy, market eligibility, utility interconnection approval, field qualification, or dispatch authority. These output flags remain false unconditionally.
