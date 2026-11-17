# Material-Accounted Routes — Stage 0 Conformance Tool

**Parent:** Symtropy [MATERIAL-ACCOUNTED-ROUTES-001 #5875](https://github.com/Luminous-Dynamics/symtropy/issues/5875)  
**Implementation tranche:** [#5879](https://github.com/Luminous-Dynamics/symtropy/issues/5879)  
**Specification:** [MATERIAL_ACCOUNTED_FACTORY_ROUTES_V0_1.md](../../docs/research/MATERIAL_ACCOUNTED_FACTORY_ROUTES_V0_1.md)  
**Status:** Standalone Rust prototype; synthetic receipt fixtures only.

## What it does

This small Rust tool checks whether owner-issued status receipts are bound to the exact case manifest, route/process/material/output subjects and declared scope. It then composes eight hard gates into one conservative synthetic disposition:

1. Route feasibility.
2. Process-boundary mass-balance status.
3. Utility and thermal-feasibility status.
4. Material-grade admission.
5. Circular allocation / no-double-count status.
6. Independent inspection disposition.
7. Output provenance completeness.
8. Synthetic-only claim ceiling.

A synthetic output is accepted only when every required receipt is present, its manifest/subject/scope bindings match exactly, each status is positive, and the source-digest field has the expected SHA-256 textual shape.

Concrete negative evidence produces `BLOCKED`; unresolved, uncertain or missing evidence produces `INDETERMINATE`. Neither can be silently upgraded to accepted.

## Critical boundary: this is not a second physics or accounting engine

The tool **does not calculate** mass balance, chemistry, utility/thermal feasibility, product conformity, material grade, industrial cost, process yield or route scheduling. It consumes statuses and exact references from the domain owners and tests whether the cross-owner composition policy is conservative.

The owner interfaces are not yet stable/merged. The receipt status vocabulary in this Stage 0 fixture is a provisional adapter contract, not a declaration that the upstream PRs already expose these exact types or wire formats. The next integration step must replace any provisional mapping with pinned canonical adapters after the owner schema audit.

Digest validation is shape-only. This tool does not retrieve source content, recompute SHA-256, authenticate issuers, validate signatures, or prove that a receipt is true. A syntactically valid digest is not authenticity or qualification.

## Run

From this directory:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo run --bin material-accounted-routes
```

The default executable evaluates the frozen case/receipt corpus under `fixtures/`, emits one TSV row per case and exits non-zero if any actual disposition differs from the frozen expected disposition.

## Frozen fixture corpus

- 30 deterministic synthetic cases.
- 8 required gate types.
- 237 receipt rows; three cases intentionally omit one required receipt.
- Positive controls for fully accounted synthetic acceptance and no-recovery-in-scope.
- Negative controls for mass imbalance, impossible interval status, double allocation, unqualified recovered feedstock, impossible utility, inspection rejection, incomplete provenance, physical-claim escalation, mismatched subject/scope/manifest, duplicate receipt identity and malformed digest.
- Indeterminate controls for uncertain mass/utility, missing receipts, unresolved grade, unknown route, unknown allocation and missing source digest.

The fixture digest token identifies the frozen synthetic fixture dataset; it is not a digest of external owner evidence. The tool reports `FORMAT_ONLY_NOT_AUTHENTICATED` where all receipt digest fields are syntactically well-formed.

## Current upstream audit snapshot

Audited against public GitHub metadata and exact file/PR heads on 17 November 2026:

| Owner surface | Exact observed PR head | Observed state | Integration consequence |
|---|---|---|---|
| Symtropy matched routes #5434 | `c35e9e2e56ce2e05b4528b625` | Open draft; branch `research/factory-matched-routes-v0-1` | Route library exists in a separate tool crate; no shared receipt API yet |
| Symthaea PIE-001A #1609 | `672de96181c7e352763e62983334e9deb8f14cc9` | Open draft; independent Python oracle | Use its status semantics only as a reference until adapter is agreed |
| Symthaea PIE-002A #1617 | `31f6e7d32e65843f0cdfc9043607751565a5b769` | Open draft; independent Python oracle | Use its conservative utility/thermal statuses only as reference |
| Mycelix accounted loop #3107 | `97d04cd8180ada98591ad6bb27a764253bc984ff` | Open draft; stacked base `circ-mfg-001c-material-flow-accounting` | No claim that the Mycelix API is on `main` |
| Symthaea process semantics #5686 | issue only | Open research | Process/material semantics remain owned upstream |
| Mycelix circular manufacturing #3095 | issue only | Open research | Lifecycle/circularity semantics remain owned upstream |

The hashes above are full observed heads where returned by GitHub metadata; shortened values in this table are for readability. Verify the linked PR before relying on any branch or schema. PR state, branch head and CI state can change independently.

## Evidence class and claim ceiling

Passing this tool establishes only that the fixture's receipt composition and exact-reference rules behave as specified. It does not verify the underlying mass-balance or utility calculations, product quality, manufacturing capability, environmental impact, economics, legal compliance, worker safety, physical production or superiority to industry SOTA.

This is Stage 0 synthetic conformance, not production release authority.
