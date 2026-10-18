# CIRC-MFG-001 — Evidence-gated circular construction/manufacturing benchmark

Tracking issue: https://github.com/Luminous-Dynamics/symtropy/issues/1984

## v0.1 scope

This first software slice is a **synthetic contract evaluator**, not a product design optimizer and not an engineering approval system. It verifies that required evidence, hard gates and a simple material mass balance are handled conservatively, and that outcomes are deterministic under semantically irrelevant row reordering.

`Eligible` means only “eligible to proceed to the next controlled benchmark stage.” It does **not** mean that a manufactured unit is qualified, a building meets code, an environmental claim is substantiated, or a safety-critical design is approved.

## Run

From any directory with a Rust toolchain and dependencies available:

```sh
cargo test --manifest-path tools/circ-mfg-001/Cargo.toml
cargo run --manifest-path tools/circ-mfg-001/Cargo.toml -- \
  tools/circ-mfg-001/fixtures/contract-v0.1.json \
  --output /tmp/circ-mfg-report.json
```

The CLI outputs a deterministic JSON report and returns nonzero if input is invalid or a fixture does not match its independent expected disposition. The report distinguishes:
- `raw_input_sha256`: exact source bytes;
- `canonical_input_sha256`: case/evidence/gate row order normalized before hashing;
- `result_sha256`: semantic result payload digest excluding the raw-byte hash, so row reordering does not change the decision digest.

The current manifest accepts `evidence_class: Synthetic` only. Measured or modeled inputs are rejected by v0.1; support for such evidence requires its own provenance, applicability and validation contract.

## Contract semantics

- A missing required record, stale evidence, unresolved gate or unknown mass quantity yields `Unresolved`.
- A digest mismatch, contradictory evidence, wrong unit, scope-invalid evidence, failed mandatory gate, invalid quantity or out-of-tolerance mass ledger yields `Blocked`.
- No mandatory gate can be compensated for by cost, circularity or another favorable metric.
- Every listed evidence record must be referenced exactly once by the case; unreferenced/duplicate records fail closed.
- Evidence marked `Verified` must include a payload whose SHA-256 matches the declared digest.
- Mass balance checks `abs(inputs_kg - accounted_outputs_kg) <= tolerance_kg`; the actual accounting boundary must be defined before physical measurements are used.
- Expected dispositions are test oracles only. They are not passed into any optimizer/policy as candidate guidance.

## Initial adversarial cases

| Fixture | Expected disposition | Control |
|---|---|---|
| `01-complete-evidence-balanced` | Eligible | complete hashes, gates pass and balanced ledger |
| `02-verified-payload-digest-mismatch` | Blocked | digest mismatch must not be accepted |
| `03-required-material-lot-missing` | Unresolved | unknown provenance is not assumed nominal |
| `04-mandatory-quality-gate-fails` | Blocked | failed hard gate dominates other metrics |
| `05-mass-balance-out-of-tolerance` | Blocked | unaccounted mass outside the declared tolerance |
| `06-required-evidence-stale` | Unresolved | stale evidence/currentness does not silently pass |

All cases are synthetic. They are not evidence of the frequency of real production defects or material loss.

## Next stage after the contract audit

1. Audit the exact active branches for Symtropy housing evidence (#1596), repairability compiler (#1773), Mycelix Manufacturing Node 0 (#4689) and physical unit/qualification provenance (#3084); record schema/field mappings and avoid forked ledgers.
2. Expand the manifest with frozen design/task scenarios and independently calculated cost/throughput/resource oracles.
3. Compare documented conventional practice, a strong parametric/off-site/circular baseline, and the evidence-guided candidate with equal data, engineering-time and simulation budgets.
4. Keep mandatory gates separate from Pareto objectives: lifecycle cost, schedule, first-pass yield, rework, material/energy/water use, service time, disassembly damage, independently accepted reuse mass, and uncertainty.
5. Add physical measurements only after tool calibration, measurement uncertainty, custody/provenance, safe test plans and the relevant qualified engineering reviews are established.

## Research grounding

- [ISO 20887:2020](https://www.iso.org/standard/69370.html) — design for disassembly/adaptability. The standard does not prescribe universal numeric targets; set project-specific gates and document how they were chosen.
- [NIST Digital Twins for Advanced Manufacturing](https://www.nist.gov/programs-projects/digital-twins-advanced-manufacturing) — trustworthy integration, verification/validation and quantified uncertainty.
- [NISTIR 8356](https://doi.org/10.6028/NIST.IR.8356) — trust/security considerations for digital twins.
- [OPC UA / MTConnect](https://opcfoundation.org/markets-collaboration/mtconnect/) — candidate open semantics for factory-device data; no single protocol should be assumed present on every machine.
- [European Commission Digital Product Passport roadmap](https://single-market-economy.ec.europa.eu/single-market/digital-product-passport_en) — planned sector-specific requirements are subject to applicable legal acts and can change. Do not claim legal compliance from this benchmark.

## Known limitations

- The evaluator is a small fixture contract, not a general JSON Schema engine. Run a Draft 2020-12 schema validator separately when validating arbitrary manifest files.
- SHA-256 proves byte consistency with the declared payload, not source truth, signer identity, physical measurement accuracy or source authorization.
- The mass balance is a basic accounting guard. Before real use, define process boundaries, moisture/content transformations, rounding, measurement calibration, waste streams and uncertainty.
- No engineering simulation, structural analysis, IFC round trip, machine actuation, life-cycle assessment, or physical trial is implemented here.
- Local compilation/test status must be reported separately from workflow status. Queued, skipped, stale or absent CI is not a pass.
