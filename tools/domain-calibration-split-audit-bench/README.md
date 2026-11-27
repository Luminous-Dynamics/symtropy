# Domain-calibration split audit v0.1

A standalone Rust audit tool for the *data independence and translation-lineage preconditions* of spacecraft pose-calibration experiments.

It is the first executable component of [#7785](https://github.com/Luminous-Dynamics/symtropy/issues/7785). It composes with #7723, #7744 and #7782; it is not a pose estimator, conformal calibrator, dataset-shift detector or coverage scorecard.

## What it checks

Each TSV row identifies a sample, target, capture session, sequence, domain and partition, plus a declared split grouping level:

- `TARGET`: target identity must not occur in more than one of TRAIN/CALIBRATION/TEST;
- `SESSION`: capture session identity must not cross partitions;
- `SEQUENCE`: sequence identity must not cross partitions.

The selected `split_group_id` must exactly match the corresponding ID field. The group kind must be constant for the entire manifest. Separate benchmark lanes should run at the identity level required by their claim (for example, one target-disjoint manifest and one session-disjoint manifest); a sequence-disjoint split is not proof of target-disjoint generalization.

The auditor also rejects:
- duplicate sample IDs;
- frame SHA-256 or source-image SHA-256 values reused across partitions;
- missing TRAIN/CALIBRATION/TEST partitions;
- translation metadata without a declared translated-image lineage;
- translated samples without translator, translated-image and geometry/label-audit digests;
- translated samples whose label-audit state is FAIL or PENDING.

## Explicit claim ceiling

The `PASS_SPLIT_INTEGRITY` disposition means only that the supplied manifest has no detected duplicate/sample/group/digest leakage under the selected split grouping and that translated samples carry a declared PASS audit receipt.

This tool does **not** inspect images, verify geometry preservation, authenticate dataset bytes, prove that the splits are statistically independent, detect all near-duplicates, measure domain shift, establish conformal exchangeability, calculate coverage or qualify a real spacecraft domain. The label-audit state and digest are upstream claims; `translation_geometry_verified_here` remains false in every receipt.

The tool is conservative about known identities but cannot establish independence from identifiers alone. A session split can still share target geometry; a sequence split can still share capture sessions; both must be reported as their own challenge lanes where relevant.

## Reproduce

```sh
cargo fmt --manifest-path tools/domain-calibration-split-audit-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/domain-calibration-split-audit-bench/Cargo.toml
cargo clippy --locked --manifest-path tools/domain-calibration-split-audit-bench/Cargo.toml --all-targets -- -D warnings

MANIFEST=tools/domain-calibration-split-audit-bench/fixtures/clean-sequence-splits-v0_1.tsv
DIGEST=$(sha256sum "$MANIFEST" | cut -d' ' -f1)
cargo run --quiet --locked --manifest-path tools/domain-calibration-split-audit-bench/Cargo.toml -- "$MANIFEST" "$GIT_COMMIT" "$DIGEST"
python3 tools/domain-calibration-split-audit-bench/tests/reference_oracle.py "$MANIFEST" "$GIT_COMMIT" "$DIGEST"
```

The independent Python challenger implements the manifest checks separately, verifies the corpus SHA-256 from raw bytes, and emits a complete JSON receipt. The dedicated workflow compares parsed Rust and Python JSON for all frozen scenarios.

## Frozen scenarios

- `clean-sequence-splits-v0_1.tsv`: positive control with clean disjoint splits and a translated calibration sample carrying a PASS audit receipt.
- `split-leakage-v0_1.tsv`: cross-partition sequence, frame-digest and image-digest leakage.
- `pending-translation-v0_1.tsv`: translation lineage exists but geometric-label audit is pending.
- `duplicate-sample-v0_1.tsv`: duplicate sample identity.

Rust unit tests also cover each identity mode, malformed group metadata, incomplete partitions, invalid translation lineage, and empty manifests.

## Next qualification steps

This slice only establishes data-manifest prerequisites. #7785 still needs:
1. independent geometric verification that image translation preserves silhouettes, landmarks, visibility and crop state;
2. per-domain/target/sequence calibration and test score distributions;
3. held-out coverage, empty-set and silent-miss metrics with cluster-aware uncertainty;
4. shift alarms and fail-closed OOD/low-support promotion policy;
5. SPEED+ reproduction followed by distinct SPADES, SpaceSense-Bench, and HIL lanes.

SpaceSense-Bench is licensed CC-BY-NC-4.0; respect its non-commercial restriction unless additional rights are granted. The dataset card reports a 19 May 2026 pose-timing correction, so downstream manifests must pin the corrected release and verify sensor/pose alignment.
