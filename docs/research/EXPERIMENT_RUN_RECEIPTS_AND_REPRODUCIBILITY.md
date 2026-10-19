# Experiment run receipts and reproducibility

**Status:** Implemented prototype in `symtropy-experiment-core`; compile/runtime checks and performance benchmarks have not yet been executed.

## Design goal

Every emitted experiment result should carry a compact receipt that binds the exact source bytes, the parsed model representation, the output payload, and the settings that affect execution. The receipt is a reproducibility and accidental-tampering aid, not a scientific-validity claim or an authenticated attestation. A static known-answer fixture fixes the exact raw input bytes, parsed model/output serialization, intermediate SHA-256 values, and final receipt digest; the expected values were computed outside Rust with Python `hashlib.sha256`.

## Current implementation

The `provenance` module uses SHA-256 and a versioned JSON receipt payload. The CLI emits receipts for arithmetic runs, graph inspection, one-variable sweeps, oscillator simulation, oscillator convergence studies, and model-validation/execution failures where a typed model is available. The `verify-receipt <result-envelope.json> <original-input.json>` command verifies the receipt's internal digest and checks the exact input-byte, serialized-model, and output-payload digests.

The receipt contains:

- Receipt schema version and explicit digest algorithm.
- Execution mode and package version.
- Optional caller-reported source revision from `SYMTROPY_SOURCE_REVISION`.
- Optional experiment ID and model schema version.
- Unit registry version for arithmetic-graph paths; unset for the separate oscillator schema.
- SHA-256 of the exact UTF-8 input file bytes read by the CLI.
- SHA-256 of the parsed model's `serde_json` serialization.
- SHA-256 of the result/error payload's `serde_json` serialization before CLI-envelope rendering.
- Execution settings that may not be fully represented in the model, such as a sweep variable/value sequence or convergence time-step sequence.
- Runtime OS, architecture, build profile and optional caller-reported `SYMTROPY_RUSTC_VERSION`.
- A receipt digest over the serialized receipt payload, excluding the receipt-digest field itself.

The exact input digest deliberately differs when source bytes differ (for example, formatting whitespace), even if the parsed-model digest stays the same. The parsed-model digest is an implementation-level serialization digest, **not** a cross-version canonical-JSON standard. Preserve both; do not collapse artifact identity and semantic model identity into a single field.

The environment values supplied through `SYMTROPY_SOURCE_REVISION` and `SYMTROPY_RUSTC_VERSION` are claims made by the build/run environment. This prototype records them but does not independently authenticate them. No trusted timestamp, signature, build attestation, compiler binary digest, dependency-tree digest, hardware fingerprint, or process isolation guarantee is provided.

## Verification boundary

`RunReceipt::verify()` recalculates the receipt's own payload digest. `verify_artifact_bytes()` additionally checks exact input bytes, the caller-supplied serialized parsed model, and the caller-supplied serialized output payload. The CLI verifier parses the saved result envelope, reconstructs the corresponding typed payload, rereads the original input artifact, checks experiment/model-schema metadata against the parsed model, and reports each digest link separately. It does not authenticate who produced the record, prove that the represented execution actually occurred, or validate scientific correctness.

A hash can bind identifiers to bytes; it cannot show that the model is correct. Receipts must therefore keep outcome labels separate: planned, rejected, executed, independently reproduced, empirically validated, and accepted for a stated purpose are not interchangeable statuses. A successful serialization or receipt verification is never itself a model-credibility pass.

## Independent known-answer fixture

The static fixture files are [the raw input](../../crates/domains/symtropy-experiment-core/fixtures/receipt-known-answer-input.json), [the raw output example](../../crates/domains/symtropy-experiment-core/fixtures/receipt-known-answer-output.json), [the expected receipt](../../crates/domains/symtropy-experiment-core/fixtures/receipt-known-answer.json), and [fixture methodology](../../crates/domains/symtropy-experiment-core/fixtures/RECEIPT_KNOWN_ANSWER.md). The test compares four fixed digest strings produced with Python `hashlib` against the Rust receipt parser/verifier, validates all artifact links, and proves that a formatting-only edit changes the raw input digest while leaving the parsed-model digest unchanged. These are authored tests; the Rust test suite has not yet run here.

## External standards and integration path

- The [in-toto Statement v1](https://github.com/in-toto/attestation/blob/main/spec/v1/statement.md) binds subject artifact names to digests and identifies a typed predicate. A future adapter could publish an experiment output as a subject and the Symtropy receipt as a domain-specific predicate. The current JSON structure is **not** an in-toto attestation.
- [SLSA provenance v1](https://github.com/slsa-framework/slsa/blob/main/spec/build-provenance.md) describes how software artifacts were produced and is useful for the *build* provenance layer. It should complement experiment-run provenance rather than being misrepresented as a scientific-execution schema.
- The [W3C PROV primer](https://www.w3.org/TR/prov-primer/) supplies a model for entities, activities and agents. A later evidence graph can link a model, input dataset, execution activity, solver, receipt, result artifact and reviewer without conflating them.
- [NASA-STD-7009B](https://standards.nasa.gov/standard/NASA/NASA-STD-7009) treats credibility and acceptance criteria as part of model-and-simulation practice. A receipt supplies some evidence about which computational objects were involved; it does not replace verification, validation, uncertainty qualification, or acceptance criteria.

## Benchmark gates

These are proposed targets, not achieved measurements.

| Gate | Procedure | Target |
|---|---|---|
| Known-answer cryptography | Published SHA-256 test vectors | 100% exact matches |
| Tamper detection | Mutate each receipt field without updating digest | 100% detected |
| Provenance sensitivity | Change input bytes, parsed model, output payload, mode or solver settings independently | Each meaningful change alters the receipt digest |
| Formatting distinction | Whitespace-only change to JSON artifact while parsed model is constant | Exact input digest changes; parsed-model digest remains constant |
| Determinism | Repeat identical model/input/output/settings on the same binary and environment | Identical receipt bytes/digest |
| Execution-mode coverage | Known-answer fixture through each CLI mode plus representative failure paths | Every result envelope that claims a receipt validates it; command-line/JSON parse failures are explicitly documented as pre-receipt errors |
| Artifact-link verifier | Valid receipt, edited input bytes, edited model, edited output payload | All valid digest links accepted; each mutated link rejected and identified independently |
| Overhead | Release-mode p50/p95 on 1 KB, 100 KB, and 10 MB payloads | Report separately from solver cost; initial target less than 5% overhead for 100 KB arithmetic-result payloads |
| Reproducibility | Rebuild and rerun from recorded commit, lockfile, input digest and settings | Exact output digest for deterministic cases; declared tolerance report for floating-point paths |
| Trust | Change a reported revision/compiler string or replace a receipt and recompute its digest | Demonstrate that unsigned hashes do not authenticate authorship; signed attestations require a separate trusted-key workflow |

The performance goal must be measured against an equivalent run without receipt construction, with compiler version, optimization profile, operating system, payload digest and sample count recorded. Do not optimize before obtaining a valid baseline.

## Next steps

1. Execute the authored suite on a trusted Rust toolchain; run formatting/lints and inspect the lockfile diff before review.
2. Add a public fixture file containing a receipt generated from known inputs and verify it with an independently implemented SHA-256 verifier.
3. Include the dependency lockfile digest and an authenticated source revision from the build pipeline; define which fields are self-reported and which are independently verified.
4. Define an experiment-run predicate and map it to an in-toto Statement, while keeping SLSA build provenance as a separate layer.
5. Store immutable output payload artifacts separately and verify their exact bytes rather than only the reconstructed payload inside the CLI envelope.
6. Only then consider signed attestations, hardware/environment details, trusted timestamps and remote verification policy.
