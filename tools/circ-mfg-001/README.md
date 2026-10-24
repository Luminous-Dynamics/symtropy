# CIRC-MFG-001 — Evidence-gated circular construction/manufacturing benchmark

Tracking issue: https://github.com/Luminous-Dynamics/symtropy/issues/1984

## v0.1 scope

This first software slice is a **synthetic contract evaluator**, not a product design optimizer and not an engineering approval system. It verifies required evidence dispositions, non-compensable gates, and exact external reference authority/identifier/role bindings. Mycelix remains authoritative for lifecycle events, custody semantics, and material-flow accounts; this harness does not duplicate quantities or those domain records.

The positive result is named `Conformant`, not `Eligible`: it means only that the synthetic record matches this harness's narrow contract. It never authorizes manufacturing, installation, site selection or structural use.

`Conformant` means only that the synthetic record matches this harness's narrow contract. It does **not** authorize manufacturing or installation, qualify a unit, establish code compliance, substantiate an environmental claim, or approve safety-critical use.

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
- `canonical_input_sha256`: case/evidence/gate/external-reference row order normalized before hashing;
- `result_sha256`: semantic result payload digest excluding the raw-byte hash, so row reordering does not change the decision digest.

The current manifest accepts `evidence_class: Synthetic` only. Measured or modeled inputs are rejected by v0.1; support for such evidence requires its own provenance, applicability and validation contract.

## Contract semantics

- A missing required record, stale evidence, unresolved gate, or missing required external reference yields `Unresolved`.
- A digest mismatch, contradictory evidence, wrong unit, scope-invalid evidence, failed mandatory gate, external-reference authority/role mismatch, or duplicate reference binding yields `Blocked`.
- No mandatory gate can be compensated for by cost, circularity or another favorable metric.
- Every listed evidence record and external reference must bind to a declared requirement exactly once; unreferenced/duplicate records fail closed.
- Evidence marked `Verified` must include a payload whose SHA-256 matches the declared digest.
- External references are checked for exact authority, identifier and declared role only. The authority enum is a **self-declared namespace**, not authentication of the issuer, registry membership, or a digital signature. Fixture role examples include Symthaea process-plan references, Mycelix process/accounting/custody references, Symtropy qualification references, and Sol-Atlas site-resource observations. These owner-role mappings are synthetic integration proposals and must be confirmed by the respective owners before measured data are admitted. The target record is not dereferenced or qualified by this tool; production integration must use the owning system's trusted resolver/verification path to check issuer, identity, freshness, scope, measurement quality and physical meaning.
- Expected dispositions are test oracles only. They are not passed into any optimizer/policy as candidate guidance.

## Initial adversarial cases

| Fixture | Expected disposition | Control |
|---|---|---|
| `01-complete-evidence-and-external-bindings` | Conformant | complete hashes, gates pass and required external authority/identifier/role bindings match |
| `02-verified-payload-digest-mismatch` | Blocked | digest mismatch must not be accepted |
| `03-required-material-lot-missing` | Unresolved | unknown provenance is not assumed nominal |
| `04-mandatory-quality-gate-fails` | Blocked | failed hard gate dominates other metrics |
| `05-required-material-flow-account-missing` | Unresolved | required external accounting reference is absent |
| `06-required-evidence-stale` | Unresolved | stale evidence/currentness does not silently pass |
| `07-critical-evidence-wrong-unit` | Blocked | unit-invalid evidence cannot satisfy a requirement |
| `08-critical-evidence-contradictory` | Blocked | conflicting critical evidence blocks eligibility |
| `09-critical-evidence-scope-invalid` | Blocked | evidence for another scope cannot be substituted |
| `10-duplicate-evidence-record` | Blocked | repeated identities do not silently collapse |
| `11-unreferenced-evidence-record` | Blocked | undeclared evidence does not silently enter a decision |
| `12-custody-reference-role-mismatch` | Blocked | a custody reference cannot substitute for a material-flow account or vice versa |
| `13-duplicate-custody-reference` | Blocked | duplicate lifecycle reference bindings fail closed |
| `14-external-reference-authority-mismatch` | Blocked | an identifier from another owning system cannot satisfy the reference |
| `15-site-resource-observation-missing` | Unresolved | missing site telemetry does not imply water, energy or service capacity |

All cases are synthetic. They are not evidence of the frequency of real production defects or material loss.

## Next stage after the contract audit

1. Audit exact active contracts for Symtropy housing evidence (#1596), repairability compiler (#1773), and Mycelix unit/qualification provenance (#3084); record field mappings and avoid parallel ledgers.
2. Coordinate the demonstrated construction/factory vertical with Symthaea [HABITAT-VERTICAL-001 #8051](https://github.com/Luminous-Dynamics/symthaea/issues/8051).
3. Feed qualified evidence into Symthaea [MFG-VALUE-001 #7608](https://github.com/Luminous-Dynamics/symthaea/issues/7608), the owner of matched-baseline production-value experiments. This package must not grow a second cost/quality/throughput evaluator.
4. Keep Mycelix [CIRC-MFG-001 #3095](https://github.com/Luminous-Dynamics/mycelix/issues/3095) and [CIRC-MFG-001B #3099](https://github.com/Luminous-Dynamics/mycelix/issues/3099) authoritative for lifecycle event binding, chain-of-custody and material-flow account references; use Symtropy [WOOD-LIFE-001 #1741](https://github.com/Luminous-Dynamics/symtropy/issues/1741) where timber/yield-specific oracles apply.
5. Keep mandatory qualification gates separate from Pareto objectives. Downstream evaluation can report lifecycle cost, schedule, first-pass yield, rework, material/energy/water use, service time, disassembly damage, independently accepted reuse mass, and uncertainty.
6. Add measured inputs only after tool calibration, measurement uncertainty, custody/provenance, safe test plans and relevant qualified engineering reviews are established.

## Research grounding (reviewed 2026-10-18)

- [NISTIR 8620 — Digital Twins Workshops Summary Report (2026-07-21)](https://doi.org/10.6028/NIST.IR.8620) identifies manufacturing digital-twin research needs including interoperability, verification/validation/uncertainty quantification, cybersecurity and workforce readiness.
- [NIST — Manufacturing in a Circular Economy: Research Needs in Design, Systems Modeling, and Digital Thread (2026-09-08)](https://www.nist.gov/publications/manufacturing-circular-economy-research-needs-design-systems-modeling-and-digital) grounds the cross-lifecycle and value-retention direction. This is a research-needs report, not proof that any candidate performs better.
- [NIST Digital Twins for Advanced Manufacturing](https://www.nist.gov/programs-projects/digital-twins-advanced-manufacturing) — trustworthy integration, verification/validation and quantified uncertainty.
- [ISO 20887:2020](https://www.iso.org/standard/69370.html) — design for disassembly/adaptability. ISO lists this edition as confirmed in 2025 and current. It does not prescribe universal numeric performance targets; set project-specific gates and document why they are appropriate.
- [EN 15978:2026 — CEN-CENELEC overview](https://www.cencenelec.eu/news-events/news/2026/en-in-the-spotlight/2026-04-17-en-15978-2026/) — harmonized building environmental-performance assessment using life-cycle assessment and quantified data. For a building LCA, keep life-cycle modules, declared boundaries, EPD input quality and uncertainty explicit; this reference-binding harness is not an LCA.
- [OPC UA / MTConnect](https://opcfoundation.org/markets-collaboration/mtconnect/) — candidate open semantics for factory-device data; no single protocol should be assumed present on every machine.
- [European Commission Digital Product Passport roadmap](https://single-market-economy.ec.europa.eu/single-market/digital-product-passport_en) — planned sector-specific requirements are subject to applicable legal acts and can change. Do not claim legal compliance from this benchmark.

## Known limitations

- The evaluator is a small fixture contract, not a general JSON Schema engine. Run a Draft 2020-12 schema validator separately when validating arbitrary manifest files.
- SHA-256 proves byte consistency with the declared payload, not source truth, signer identity, physical measurement accuracy or source authorization.
- External-reference presence, authority and role agreement do not prove that the referenced records are true or qualified. The harness deliberately does not copy material-flow quantities from Mycelix's accounting owner.
- No engineering simulation, structural analysis, IFC round trip, machine actuation, life-cycle assessment, or physical trial is implemented here.
- Local compilation/test status must be reported separately from workflow status. Queued, skipped, stale or absent CI is not a pass.
