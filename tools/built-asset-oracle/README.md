# Built Asset Oracle

Standalone Rust reference checker for the **synthetic v0.1** built-asset inspection corpus.

This is intentionally a small, isolated workspace. It does not join the large Symtropy application workspace and has no Bevy, robotics-controller, simulator, network or database dependency.

## Run from the repository root

```sh
cargo test --manifest-path tools/built-asset-oracle/Cargo.toml
cargo clippy --manifest-path tools/built-asset-oracle/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/built-asset-oracle/Cargo.toml -- \
  --corpus docs/research/built-asset/fixtures/v0.1/corpus.json \
  --output /tmp/built-asset-oracle-report.json
jq -e '.passed == true and .case_count == 16 and .mismatched_cases == 0' \
  /tmp/built-asset-oracle-report.json
```

The default corpus path is repository-root-relative. Use `--corpus PATH` when running from another working directory. Use `--output PATH` to write a normalized JSON report; otherwise the report is printed to stdout.

## What it checks

- Asset identity, declared units and coordinate frame.
- Calibration identity/currentness and design revision.
- Required region coverage (not image count).
- Duplicate upstream sensor lineage (not independent corroboration).
- Declared reference-measurement conflict threshold.
- Supported synthetic material profile.
- Human authorization and independent functional-test result for repair claims.
- Exact expected disposition **and reason-code** agreement for every frozen case.

The evaluator does not read `sealed_reference_truth`. That block is preserved in the corpus for referee/benchmark use and must not become a candidate inference feature. The oracle's rules are deliberately small and explicit; they are not a learned detector and do not establish physical sensor performance.

## Claim ceiling

A passing report means only that this implementation agrees with the finite synthetic v0.1 contract. It does not establish field perception, material diagnosis, structural integrity, building-code compliance, repair durability, robot safety, construction authority or return-to-service permission.

A digest binds the corpus bytes, not their truth. Any expansion of the corpus or evaluator logic requires review of the known answers and independent challenge cases.
