# Legal Evidence Benchmark (synthetic-only v0.1)

This isolated Rust utility is the first executable evaluation slice for LEGAL-EVAL-001. It tests evidence-routing and source-applicability behaviors that should be stable across Symthaea, Mycelix, Symtropy, and Sol-Atlas adapters. It does not provide legal advice, decide real legal questions, or establish real-world compliance.

## Why this benchmark exists

Existing efforts cover complementary slices of the problem:

- LegalBench covers a broad set of legal reasoning tasks.
- LegalBench-RAG evaluates retrieval of precise legal text spans.
- COLIEE evaluates case retrieval and legal entailment tasks.
- LegalCiteBench (2026 preprint) studies citation recovery, verification, and case matching.
- AR-BENCH (January 2026 preprint) explores legal-error detection, classification, and correction in finalized criminal judgments; it demonstrates a useful *review-after-generation* task family, but its reported data concern Mainland China criminal cases and must not be treated as a jurisdiction-neutral corpus.
- ELI demonstrates structured legal identifiers, metadata, and synchronization.
- NIST AI RMF provides a trustworthiness risk-management frame.

This first tranche does not copy or redistribute those datasets. It uses a small, original synthetic fixture to exercise temporal applicability, jurisdictional uncertainty, authority classification, citation integrity, source-use restrictions, and safe abstention. Any later adapter must verify the source dataset's current terms, licenses, provenance, and permitted processing before use.

## Run

From the repository root:

- Validate the corpus with `cargo run --manifest-path crates/tools/legal-evidence-benchmark/Cargo.toml -- validate crates/tools/legal-evidence-benchmark/fixtures/legal-knowledge-v0.1.json`.
- Run unit tests with `cargo test --manifest-path crates/tools/legal-evidence-benchmark/Cargo.toml`.
- Evaluate a candidate submission with `cargo run --manifest-path crates/tools/legal-evidence-benchmark/Cargo.toml -- evaluate crates/tools/legal-evidence-benchmark/fixtures/legal-knowledge-v0.1.json path/to/submission.json`.

The CLI prints machine-readable JSON. Validation errors exit with code 2; a failed evaluation gate exits with code 1; a fixture-gate pass exits with code 0. The command is deterministic and performs no network access.

## Reproducible receipts

Validation output records the SHA-256 digest of the exact corpus file bytes. Evaluation output records SHA-256 digests of the exact corpus and submission file bytes plus the evaluator package version. Preserve this receipt alongside the source revision and CI run URL when comparing results. Input digests identify the evaluated files; they do not authenticate who published a source or prove that the source is authoritative. The unit-level convenience wrapper hashes deterministic serialization of its in-memory fixture objects; the CLI hashes raw input-file bytes.

The CI workflow also evaluates `fixtures/reference-submission-v0.1.json`. This is a synthetic oracle/control submission intended to exercise the evaluator itself; it is **not a measurement of any Symthaea or other production model**.

## Submission contract

A submission has schema version 0.1.0, a non-empty system identifier, and exactly one result per benchmark case. Each result includes:

- **case_id**
- **action:** one of answer, clarify, abstain, or reject_source
- **claim_codes:** bounded semantic labels emitted by the candidate adapter
- **citations:** authority ID, authority role, and exact span ID

The candidate adapter is responsible for mapping a system's structured evidence output into this contract. The evaluator does not call a model and does not grade free-form prose.

## Fail-closed checks

The evaluator reports and gates on:

- Missing, duplicate, and unexpected case results.
- Action, claim-code, and citation-set exact-match rates.
- Unknown authorities and citations to sources not made available for that case.
- Incorrect authority-role classification and unknown span identifiers.
- Unresolved conflicts between simultaneously applicable authorities, where no conflict-resolution rule has been provided.
- Sources outside their half-open effective interval (valid_from <= as_of < valid_to).
- Source citations with no date for evaluating a time-bounded authority.
- Citations to prohibited-use or unverified authority records.
- Duplicate citations, duplicate claim codes, and answers without citations.

A passing synthetic run means only that one candidate submission matches the expected labels and evidence references in this fixture. The fixture's labels are not evidence of real legal correctness. A correct citation identifier also does not, by itself, establish that a passage supports every natural-language claim.

## Architecture principles

1. A legal claim needs a jurisdiction, a relevant date, and traceable authority.
2. Geography alone does not prove subject-matter competence or adjudicative authority.
3. A declared governing law or forum is not proof that the declaration is valid or enforceable.
4. Foreign decisions, organizational policies, proposed instruments, and binding local authorities must remain different types.
5. Public readability is not permission to scrape, train, cache, or redistribute.
6. A missing or duplicate result never counts as a pass.
7. Aggregate scores may not hide critical failures. Release decisions must retain the per-case failures and the exact corpus/version identity.
8. Legal interpretation and consequential deployment decisions require qualified human review.

## Proposed staged targets

These are development targets, not claims of current performance:

| Gate | Target |
|---|---|
| Synthetic integrity suite | 100% expected actions, claim codes, and citation/span identities |
| Fabricated or unavailable authority in the release suite | 0 |
| Critical temporal/jurisdiction/source-rights negative controls | 0 failures |
| Time-bounded authority without a material date | Clarify or abstain; never silently infer currentness |
| Citation/claim support on a future expert-adjudicated corpus | At least 98% of scored claims supported by cited passages; zero critical unsupported claims in the release set |
| Precise retrieval on a rights-cleared legal corpus | Target at least 95% span-level precision and recall, reported separately by jurisdiction and task |
| Procedural deadlines and filing requirements | 100% verified against an applicable source or explicitly abstained/handed off |
| Data-use compliance | Zero use outside each source's explicit permission profile |

The first three gates are machine-checkable here. The expert-support, retrieval, and procedural targets require a later rights-cleared, independently adjudicated corpus; this v0.1 utility does not claim to measure them.

## Cross-project division of responsibility

- **Symthaea:** retrieve evidence, reason over the case, expose assumptions and counterarguments, and produce structured claims/citations.
- **Mycelix:** own canonical governance/authority/profile contracts and durable provenance, building on LEX-NET and MYC-JUR work rather than duplicating it.
- **Symtropy:** host synthetic/adversarial evaluations, deterministic oracles, and rights-aware outcome-validation methods.
- **Sol-Atlas:** contribute location and administrative-boundary observations as evidence, never as standalone proof of legal competence.

The shared contract should identify authority/profile snapshots by immutable revision or digest. It must never let a score, model confidence, polygon containment, signature, or protocol conformance silently create legal authority.

## References

- LegalBench: https://arxiv.org/abs/2308.11462
- LegalBench-RAG: https://arxiv.org/abs/2408.10343
- COLIEE 2026 task overview: https://coliee.org/
- LegalCiteBench (2026 preprint): https://arxiv.org/abs/2605.10186
- AR-BENCH (2026 preprint): https://arxiv.org/abs/2601.22742
- European Legislation Identifier: https://eur-lex.europa.eu/eli-register/what_is_eli.html
- NIST AI Risk Management Framework: https://www.nist.gov/itl/ai-risk-management-framework/ai-rmf-development
- SAFLII terms of use: https://www.saflii.org/content/terms.html

SAFLII's current terms restrict automated copying of case law and AI-related use of its collection. The benchmark therefore contains only original fictional records. Do not add legal source text or external benchmark data until its permitted use is verified and documented.
