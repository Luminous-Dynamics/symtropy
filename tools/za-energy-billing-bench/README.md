# ZA Energy Billing Reference

Standalone, dependency-free Rust evaluator for deterministic City Power 2026/27 Industrial TOU billing-component conformance.

## Scope

- Requires City Power account identity, an Industrial TOU approval reference and supply capacity strictly greater than 100 kVA.
- Uses source-supplied local date, day class and local interval boundaries. Does not infer public holidays or timezone conversion.
- Preserves the Saturday 18:00–20:00 source-text ambiguity until an approved account/tariff confirmation reference is supplied.
- Calculates energy, service, monthly network, billed demand, reactive-energy and optional per-kWh network surcharge components.
- Requires exactly 12 contiguous prior monthly peak-demand records for the rolling demand ratchet.
- Accepts the Business/LPU account surcharge and VAT only as externally assessed invoice lines with evidence references. It does not guess their bases or hardcode VAT.
- Returns INCOMPLETE_INVOICE_RECONCILIATION and a known-component subtotal whenever a bill line is unknown.

The 70 kVA minimum demand floor and the 70–1,000 kVA LV LPU service-connection range are not TOU eligibility rules. The Industrial TOU tariff clause 5.4.3 uses a strict >100 kVA supply-capacity gate plus application/approval.

## Run

\`\`\`
cargo test --manifest-path tools/za-energy-billing-bench/Cargo.toml
cargo run --manifest-path tools/za-energy-billing-bench/Cargo.toml
\`\`\`

The CLI emits a machine-readable synthetic receipt. Its FNV-1a-64 value is a reproducible non-cryptographic input fingerprint, not an authenticity signature. The reference is not a production customer billing system and makes no real-facility saving claim.
