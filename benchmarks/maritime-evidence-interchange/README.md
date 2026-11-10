# Maritime Evidence Interchange Validator

Rust-first, offline semantic conformance checker for the synthetic v0.1 corpus.

## Run

```sh
cargo test --locked --all-targets --manifest-path benchmarks/maritime-evidence-interchange/Cargo.toml
cargo run --locked --manifest-path benchmarks/maritime-evidence-interchange/Cargo.toml -- \
  benchmarks/maritime/maritime-evidence-interchange-conformance-v0.1.json
```

The CLI writes a JSON report to stdout. Exit codes: 0 means all expected fixture dispositions and hard negative controls matched; 1 means one or more conformance cases/gates failed; 2 means the corpus could not be loaded or parsed safely.

## What it checks

- Corpus identity, fixture count and unique fixture/record IDs.
- Bounded inline base64 source payloads, exact byte length and SHA-256.
- Asset/configuration identity and mandatory source/adapter/mapping provenance.
- Canonical UTC event/receipt timestamps, pinned evaluation time, time-basis and clock metadata.
- Native channel IDs and mapping state; ambiguous/unmapped records abstain.
- Missing values remain valueless; derived/reconstructed/imputed values require transformation lineage.
- Vendor/model diagnostic assertions retain a distinct record type, explicit claim ceiling and `authority_granted=false`.
- Support/contradiction/derived-from links resolve to records in the corpus.
- Duplicate sequence detection within an asset/source/clock stream in a fixture.

## Deliberate limitations

- This is a semantic corpus oracle, **not** a general Draft 2020-12 JSON Schema validator.
- It does not execute a third-party adapter and therefore does not measure adapter round-trip fidelity, semantic-mapping precision, information loss or event-order preservation. Those report fields remain null until an adapter is actually run.
- The offline runner rejects unresolved external `content_ref` payloads. A bounded resolver and content-store policy are a later implementation task.
- v0.1 timestamp evaluation currently requires canonical whole-second UTC `YYYY-MM-DDTHH:MM:SSZ` values; relative-monotonic observations are accepted only with a monotonic counter.
- No live ship OT integration, controller/PLC access, maintenance authority, ODD/mode authority, diagnostic accuracy or physical validation is provided.

A green corpus run means only that this validator reproduced the corpus's expected semantic dispositions and hard negative controls. It does not certify an adapter, vessel or machinery capability.
