# Bregar UWB v2 streaming CSV ingestion and controller/evaluator separation

**Status:** strict positional one-line parser and deterministic tests implemented; raw Zenodo v2.0 archive has not been downloaded, hashed or inspected.

**Dataset candidate:** [Zenodo DOI 10.5281/zenodo.8280736](https://doi.org/10.5281/zenodo.8280736), candidate only in [registry PR #6779](https://github.com/Luminous-Dynamics/symtropy/pull/6779).

## 1. Upstream format convention

The authors' pinned converter `KlemenBr/uwb_positioning@a39472cf5bf828dce0ee1a403438744ea76cd1a8/preprocess/save_raw_data_to_json.py`:
- skips the first two physical lines of each per-capture CSV;
- reads subsequent lines with no header interpretation (`header=None`);
- uses a fixed 26-field metadata map, in order;
- starts the complex CIR sample sequence at zero-based field index 26.

The parser therefore preserves the first two lines for provenance but deliberately does **not** treat their displayed labels as the schema authority. This matches the authors' own positional reader and avoids rejecting a valid file because of a missing/renamed human-readable header cell. The supported field mapping is pinned in Rust source.

## 2. Positional row map

| Index | Field | Boundary |
|---:|---|---|
| 0 | TAG_ID | Controller identity |
| 1 | ANCHOR_ID | Controller identity |
| 2–4 | X_TAG, Y_TAG, Z_TAG | Evaluator-only position |
| 5–7 | X_ANCHOR, Y_ANCHOR, Z_ANCHOR | Evaluator-only anchor coordinate |
| 8 | NLOS | Evaluator-only label |
| 9 | RANGE | Measured range observation, controller-facing only as an unqualified measurement |
| 10 | FP_INDEX | Controller-facing radio feature |
| 11–12 | RSS, RSS_FP | Controller-facing received-power features |
| 13–15 | FP_POINT1/2/3 | Controller-facing first-path features |
| 16–18 | STDEV_NOISE, CIR_POWER, MAX_NOISE | Controller-facing radio quality metadata |
| 19 | RXPACC | Controller-facing CIR accumulation metadata |
| 20–25 | CHANNEL_NUMBER, FRAME_LENGTH, PREAMBLE_LENGTH, BITRATE, PRFR, PREAMBLE_CODE | Controller-facing radio configuration metadata |
| 26 onward | CIR complex samples | Controller-facing raw CIR |

This positional table reflects the authors' reference converter, not an inspection of the actual v2.0 payload. A raw file may include additional version-specific variations; those must be checked during payload admission.

## 3. Controller/evaluator split

The parser returns two separate structs:

**Controller view** includes source file identity/row index, tag and anchor IDs, measured range, FP_INDEX, RSS/RSS_FP, first-path values, noise/accumulation metadata, radio configuration and complex CIR samples.

**Evaluator view** contains NLOS label plus tag/anchor XYZ coordinates.

The controller view can be converted to `BregarRawCirObservation` with an explicit externally qualified sample period. It cannot access the evaluator fields through this typed API. The parser does not itself establish a process/security boundary; downstream estimator code must be passed only the controller view.

The measured RANGE remains an unqualified observation until its timestamp, calibration and provenance semantics are audited. The evaluator NLOS and position fields must not be used for feature extraction, tuning or controller filtering.

## 4. Streaming and parsing

- The API accepts one physical CSV line at a time; it does not load the 4.4GB archive or a whole environment into memory. Callers must supply explicit maximum record-byte and CIR-sample limits, and rows exceeding either bound are rejected before CIR allocation.
- It supports quoted cells, doubled quote escapes and the complex string forms used by the authors' Python converter.
- It rejects malformed quote state, missing metadata/CIR, invalid required values, partial coordinate triplets, invalid RXPACC and malformed/nonfinite complex values.
- Optional RSS/feature metadata can be absent or NaN and remains explicitly unavailable.
- It preserves source file and row identity; it does not create an evaluator-derived row filter.

The caller remains responsible for opening the exact publisher archive, selecting files, hashing bytes and assigning stable measurement IDs. The parser does not open ZIP archives or traverse dataset directories.

## 5. Tests

Thirteen deterministic synthetic tests cover:
1. Two-line preamble preservation and fixed positional prefix.
2. Human-readable header variation without changing positional row parsing.
3. Empty/malformed preamble rejection.
4. Controller/evaluator label and coordinate separation.
5. Missing optional signal features.
6. Partial truth-coordinate triplet rejection.
7. Python complex values, signs and exponents.
8. Connection from controller row to explicit-sample-period CIR observation.
9. File/row identity and short-row rejection.
10. Invalid required metadata and complex cell rejection.
11. Quoted CIR cells and invalid quoting after close.
12. Controller-view to CIR-feature integration without evaluator labels.
13. Explicit record-byte and CIR-sample resource limits.

These validate parser logic on synthetic fixtures, not the actual Zenodo archive.

## 6. Remaining admission work

1. Download only the exact v2.0 record after verifying the version/rights metadata.
2. Record byte counts and SHA-256 for every file; upstream MD5 is not our SHA-256 receipt.
3. Inspect actual preamble/version lines and compare field order against this pinned converter.
4. Compare sample row parsing against the authors' Python converter on an independently selected file.
5. Confirm CIR complex encoding, sample counts, missing values and PRFR/sample-time semantics.
6. Audit whether row counts and controller event IDs are invariant under NLOS/position-label mutation.
7. Audit anchor surveys, calibration, range reference and NLOS label lineage.
8. Freeze environment-held-out splits before model selection.

Until complete, Bregar v2.0 remains `candidate_not_admitted` and `admission_scope=none`.

## Claim boundary

This parser is a deterministic source-layout and API-separation implementation based on the authors' reference converter. It is not a payload validation, physical calibration, ground-truth qualification, NLOS classifier or navigation integrity result.
