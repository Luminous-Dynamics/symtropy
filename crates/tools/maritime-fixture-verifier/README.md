# Maritime Synthetic Fixture Verifier

This standalone Rust tool verifies the frozen, synthetic v0.1.0 maritime/floating-site fixture document. It is a data-contract checker, not a simulation engine, digital-twin executor, engineering validator, or physical-control component.

## Run

From the Symtropy repository root with the pinned Rust toolchain:

```sh
cargo test -p maritime-fixture-verifier --locked
cargo run -p maritime-fixture-verifier --locked -- docs/tech/MARITIME_FLOATING_SITE_SCENARIO_V0_1.json
```

The report includes the SHA-256 digest of the exact input bytes and a source-commit binding. Local runs without `MARITIME_SOURCE_COMMIT` are explicitly marked `UNBOUND_LOCAL_SOURCE`; they are not repository receipts. CI supplies the candidate PR head SHA.

## Fail-closed behavior

The verifier rejects duplicate JSON object keys, unknown or missing fields, unsupported schema revisions, altered v0.1.0 frozen values, duplicate or reordered fixture IDs, unknown units, non-finite/unrepresentable values, impossible charging efficiency, mismatched expected outcomes, and input exceeding 1 MB. Failures exit nonzero and emit no success report.

## Claim ceiling

A successful report means only that all six frozen synthetic fixture inputs, declared contracts, arithmetic and expected dispositions agree under this verifier build. It does not establish model fidelity, seaworthiness, berth or grid safety, nuclear safety/security, cyber resilience, emergency evacuation adequacy, certification, external approval, or authority to operate physical equipment.

The tool is intentionally separate from Symtropy's production physics and requires no physical-model execution. It must not be used to bypass the ENG-SYNTH / ENG-SYNTH-CRED capability census, independent qualification or fail-closed engineering-admission gates.
