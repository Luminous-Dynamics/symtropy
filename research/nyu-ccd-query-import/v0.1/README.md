# NYU CCD Query Import v0.1

A dependency-free Rust importer for the public NYU continuous-collision-detection query representation. It preserves rational coordinates as exact numerator/denominator strings and does not convert source coordinates to f64 during import.

## Supported formats

The scalable dataset stores per-scene folders for queries, broadphase boxes, Mathematica Boolean ground truth and symbolic roots. The query CSV contains eight rows per query and six integer columns: numerator/denominator pairs for x, y and z. Edge-edge and vertex-face row order is selected explicitly by the caller. The Boolean ground-truth JSON stores JSON `true`/`false` tokens only, one Boolean per query, and is attached after parsing. Legacy CSV labels alone accept `0`/`1` as well as `false`/`true`. The boxes JSON is an array of unordered pairs of box IDs; the importer canonicalizes each pair so reversed order has the same identity.

The legacy 2021 sample format uses eight rows per query and seven columns, with the seventh column repeating the Boolean result on each row. The importer checks that all eight row labels agree.

## Exactness

The numerator and denominator are retained as strings, so values larger than u64/u128 are not truncated. Denominators must be positive nonzero decimal integers. Decimal, exponent, sign-plus and non-integer encodings are rejected. The importer does not implement arbitrary-precision arithmetic or evaluate geometric predicates; a later algorithm adapter must choose an appropriate exact or interval representation. This boundary prevents the dataset ingestion layer from silently discarding exact source information.

## Run

```sh
cargo fmt --manifest-path research/nyu-ccd-query-import/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/nyu-ccd-query-import/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/nyu-ccd-query-import/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Real upstream smoke fixture

The test suite now imports the public `armadillo-rollers/queries/3ee.csv`, `mma_bool/3ee_mma_bool.json` and `boxes/3ee.json` sample files. It verifies two queries, both false ground-truth labels, two broadphase box pairs, exact rational lexeme preservation and row-role mapping. The small sample excerpt is retained in `fixtures/` under the upstream Apache-2.0 license; its license text is included at `LICENSES/Apache-2.0.txt`. This is an import smoke test, not the full benchmark or a CCD-accuracy result.

## Sources

- NYU CCD benchmark and code/data index: https://continuous-collision-detection.github.io/
- Scalable CCD sample dataset and file format: https://github.com/Continuous-Collision-Detection/Sample-Scalable-CCD-Data
- Original benchmark dataset: https://archive.nyu.edu/handle/2451/61518
- Full scene/time-of-impact dataset: https://archive.nyu.edu/handle/2451/74508

The datasets are not copied into this repository. The import tests use small synthetic fixtures and preserve the original exact rational format.
