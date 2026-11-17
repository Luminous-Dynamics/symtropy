# Material-Accounted Routes — Stage 0 Conformance Tool

**Parent:** Symtropy [MATERIAL-ACCOUNTED-ROUTES-001 #5875](https://github.com/Luminous-Dynamics/symtropy/issues/5875)  
**Implementation tranche:** [#5879](https://github.com/Luminous-Dynamics/symtropy/issues/5879)  
**Specification:** [draft PR #5876](https://github.com/Luminous-Dynamics/symtropy/pull/5876)  
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

The crate contains ten unit tests covering frozen-corpus agreement, missing receipts, exact subject/scope joins, duplicate receipt IDs within and across cases, malformed/missing digests, orphan receipts, unknown status values and receipt-order determinism. The default executable evaluates the frozen case/receipt corpus under `fixtures/`, emits one TSV row per case and exits non-zero if any actual disposition differs from the frozen expected disposition.

## Frozen fixture corpus

- 31 deterministic synthetic cases.
- 8 required gate types.
- 245 receipt rows; three cases intentionally omit one required receipt. One adversarial case reuses a receipt ID across manifests and must block both cases.
- Positive controls for fully accounted synthetic acceptance and no-recovery-in-scope.
- Negative controls for mass imbalance, impossible interval status, double allocation, unqualified recovered feedstock, impossible utility, inspection rejection, incomplete provenance, physical-claim escalation, mismatched subject/scope/manifest, duplicate receipt identity within and across cases, and malformed digest.
- Indeterminate controls for uncertain mass/utility, missing receipts, unresolved grade, unknown route, unknown allocation and missing source digest.

The repeated SHA-256-shaped fixture value is a fixed test token; it is not the digest of the fixture file or external owner evidence. The tool reports `FORMAT_ONLY_NOT_AUTHENTICATED` where all receipt digest fields are syntactically well-formed.

## Current upstream audit snapshot

Audited against public GitHub metadata and exact file/PR heads on 17 November 2026:

| Owner surface | Exact observed PR head | Base | Observed state / changed surface | Integration consequence |
|---|---|---|---|---|
| Symtropy matched routes #5434 | `c35e9e2e56c43b947dea17e52ce2e05b4528b625` | `240914e56939eade580a2b4245c5cf5592ddd276` | Open draft; `tools/factory-matched-routes` Rust crate | Route library exists separately; no shared receipt API yet |
| Symthaea PIE-001A oracle #1609 | `672de96181c7e352763e62983334e9deb8f14cc9` | `980d0bfca1f67143a3095dff25437f913ac10be7` | Open draft; independent Python oracle + evidence note | Reference arithmetic/status semantics only |
| Symthaea PIE-001B #1614 | `b5d2a0a48665926beb398b60c6f58b8cfbe13401` | `b7305085fc5bbb7aed400c2a39f0c5f597a11035` | Open draft; production mass-balance module; stacked on PIE-000 branch | Do not assume it is on `main` |
| Symthaea PIE-002A oracle #1617 | `31f6e7d32e65843f0cdfc9043607751565a5b769` | `980d0bfca1f67143a3095dff25437f913ac10be7` | Open draft; independent utility/thermal oracle + evidence note | Reference screening semantics only |
| Symthaea PIE-002 kernel #2724 | `1965f58fcea91ae83a73714fa93f0f22ad1abc20` | `b5d2a0a48665926beb398b60c6f58b8cfbe13401` | Open draft; production utility kernel; stacked on PIE-001B | Do not assume it is on `main` |
| Symthaea PIE-002B #2759 | `0212804ae03f9db538332e72cde7a1d2743edbeb` | `1965f58fcea91ae83a73714fa93f0f22ad1abc20` | Open draft; process utility projection; stacked on #2724 | Do not assume it is on `main` |
| Mycelix accounted loop #3107 | `97d04cd8180ada98591ad6bb27a764253bc984ff` | `a54ce9712850d0e4327da16e1e861a98b365bf85` | Open draft; accounted closed-loop test, stacked on material-flow branch | No claim that the API is on `main` |
| Symthaea process semantics #5686 | issue only | — | Open research | Process/material semantics remain owned upstream |
| Mycelix circular manufacturing #3095 | issue only | — | Open research | Lifecycle/circularity semantics remain owned upstream |

All hashes above are full observed 40-character Git commit IDs, not shortened display values. The stacked dependency chain is significant: PIE-001B is based on PIE-000; PIE-002 on PIE-001B; PIE-002B on PIE-002; Mycelix #3107 is based on a separate material-flow branch. Verify PR state, branch head, changed files and CI independently before relying on any surface.

## Evidence class and claim ceiling

Passing this tool establishes only that the fixture's receipt composition and exact-reference rules behave as specified. It does not verify the underlying mass-balance or utility calculations, product quality, manufacturing capability, environmental impact, economics, legal compliance, worker safety, physical production or superiority to industry SOTA.

This is Stage 0 synthetic conformance, not production release authority.
