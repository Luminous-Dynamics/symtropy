# NYU CCD Query Import v0.1

A dependency-free Rust importer for the public NYU continuous-collision-detection query representation. It preserves rational coordinates as exact numerator/denominator strings and does not convert source coordinates to f64 during import.

## Supported formats

The scalable dataset stores per-scene folders for queries, broadphase boxes, Mathematica Boolean ground truth and symbolic roots. The query CSV contains eight rows per query and six integer columns: numerator/denominator pairs for x, y and z. Edge-edge and vertex-face row order is selected explicitly by the caller. The Boolean ground-truth JSON stores one Boolean per query and is attached after parsing. The boxes JSON is an array of pairs of box IDs.

The legacy 2021 sample format uses eight rows per query and seven columns, with the seventh column repeating the Boolean result on each row. The importer checks that all eight row labels agree.

## Exactness

The numerator and denominator are retained as strings, so values larger than u64/u128 are not truncated. Denominators must be positive nonzero decimal integers. Decimal, exponent, sign-plus and non-integer encodings are rejected. The importer does not implement arbitrary-precision arithmetic or evaluate geometric predicates; a later algorithm adapter must choose an appropriate exact or interval representation. This boundary prevents the dataset ingestion layer from silently discarding exact source information.

## Run

```sh
cargo fmt --manifest-path research/nyu-ccd-query-import/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/nyu-ccd-query-import/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/nyu-ccd-query-import/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Sources

- NYU CCD benchmark and code/data index: https://continuous-collision-detection.github.io/
- Scalable CCD sample dataset and file format: https://github.com/Continuous-Collision-Detection/Sample-Scalable-CCD-Data
- Original benchmark dataset: https://archive.nyu.edu/handle/2451/61518
- Full scene/time-of-impact dataset: https://archive.nyu.edu/handle/2451/74508

The datasets are not copied into this repository. The import tests use small synthetic fixtures and preserve the original exact rational format.
