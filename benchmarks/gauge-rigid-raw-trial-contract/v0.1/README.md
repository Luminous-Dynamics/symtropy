# GAUGE raw rigid-body trial contract v0.1

**Parent:** [BENCH-GAUGE-RIGID-001 #6446](https://github.com/Luminous-Dynamics/symtropy/issues/6446)  
**Child issue:** [#6580](https://github.com/Luminous-Dynamics/symtropy/issues/6580)  
**Source revision:** `f95bb06e7d11142f88ffd3b6b56c7258deb8fff6`  
**Scope:** structural parser and derived-view contract only. No raw physical-trial data is vendored.

## Observed raw files

Two representative files in the public GAUGE dataset were inspected:
- `data/rigid/bouncing ball/json/1.json` (12.3 kB);
- `data/rigid/newton's cradle/json/1.json` (416 kB, multiple body streams).

Both declare `FPS: 30`, `translation unit: mm`, and `euler order: ZYX`. Each body stream provides arrays:
- translation: `x`, `y`, `z`;
- quaternion components: `qx`, `qy`, `qz`, `qw`;
- Euler components: `ex`, `ey`, `ez`.

The Newton's-cradle sample contains multiple named body streams (including `ball_passive`). This confirms that the released benchmark includes machine-readable rigid-body pose trajectories, not only video.

## Parser contract

This v0.1 parser supports only the observed declaration profile:
- positive finite FPS;
- translation unit exactly `mm`;
- Euler order exactly `ZYX`;
- one or more named body streams;
- all ten pose channels present, finite and equal-length, with at least one sample.

It rejects missing channels, mismatched lengths, invalid scalar declarations, malformed JSON and zero-norm quaternion samples. It reports maximum quaternion norm deviation; it does not normalize or rewrite the source values.

The parser uses only the Rust standard library. It validates the entire JSON document, enforces a maximum nesting depth, rejects duplicate object keys and trailing payloads, and decodes JSON string escapes (including surrogate pairs). This avoids adding a third-party dependency to the isolated benchmark crate.

## Acquisition rate versus export rate

The GAUGE paper reports that 16 motion-capture cameras acquire rigid-body trajectories at 180 Hz, then each recording is truncated to the valid interval and downsampled to 30 FPS for the released JSON. Rigid-body and textile simulator states are also exported at 30 Hz from simulations running at 180 Hz.

The raw JSON's `FPS: 30` therefore means **export cadence**, not original acquisition rate. The downsampling filter, sample phase/anchor, timestamp mapping and uncertainty propagation are not established by the inspected raw records. A 30 Hz export has 33.3 ms sample spacing and 15 Hz Nyquist; it cannot alone establish short impact peaks or sub-frame contact timing. Interpolation cannot reconstruct that lost high-frequency information.

## Derived views

- Position conversion: raw millimetres × 0.001 = metres.
- Relative sample time: zero-based frame index / declared FPS.

These conversions are derived views. The source JSON remains authoritative and unchanged. The derived time is **relative frame-index time only**, not a sensor timestamp, absolute clock, or proven capture start time.

## Explicitly unresolved semantics

The inspected files do not establish:
- quaternion handedness, algebra, multiplication convention or whether the transform maps body-to-world or world-to-body;
- world/body frame IDs and coordinate handedness;
- Euler angle unit and active/passive interpretation;
- absolute timestamp/clock origin and synchronization;
- per-sample measurement covariance or uncertainty;
- complete all-family missing-frame, outlier and repeated-recording policy.

The report marks these as unresolved. Its `XYZW` label describes the component key order (`qx/qy/qz/qw`) only; it is not a quaternion algebra or frame declaration.

## Claim boundary

A structurally valid file is not necessarily calibrated, physically accurate, licensed for arbitrary reuse, or eligible for physical scoring. This parser does not validate all eight rigid task families, import raw data, reproduce GAUGE metrics, or qualify Symtropy's physics. Complete family/material/trial schema review, file hashes, rights review and uncertainty/split audit remain gates under #6446.

## Exhaustive local source inventory CLI

The stacked follow-on adds `gauge-rigid-inventory`, which reuses the exact parser above rather than implementing a second JSON reader.

Run it against an already downloaded, immutable GAUGE rigid subtree:

```bash
cargo run --release --manifest-path benchmarks/gauge-rigid-raw-trial-contract/v0.1/Cargo.toml --bin gauge-rigid-inventory -- \
  --root /path/to/GAUGE-Dataset/data/rigid \
  --dataset-revision f95bb06e7d11142f88ffd3b6b56c7258deb8fff6 \
  --output gauge-rigid-inventory.jsonl
```

The inventory command:
- recursively enumerates regular `.json` files only, in deterministic root-relative path order;
- never follows symlinks and records any symlink/scan failure as a blocker;
- reads exact raw bytes, hashes them with the system `sha256sum` executable **before** UTF-8/JSON parsing, and fails closed if SHA-256 tooling is unavailable;
- invokes the existing `validate_trial_json` API unchanged;
- emits one header record, one record per JSON file and one summary record as JSONL;
- includes raw digest/byte length, source family/material/subtask path labels, parse disposition, dynamic sorted body stream IDs, sample counts, quaternion norm deviation and unresolved parser semantics;
- marks metadata mapping `NOT_ASSESSED` and physical trial/session grouping `UNKNOWN`;
- reports eight-family coverage, malformed/unsupported/unassessable counts and unknown-family paths;
- exits non-zero after writing the manifest when any file is invalid/unassessable, any family is absent, an unknown family is encountered, or a scan/symlink issue exists.

`sha256sum` is an explicit system requirement for this Linux inventory command; the binary does not implement cryptography itself. The dataset is not downloaded, copied, normalized or modified. Only the JSONL manifest is written to the requested output path.

**Important:** this is still a source-structure inventory, not an independent-trial manifest. File counts are not trial counts; filename numbering does not prove session/article independence. It does not join metadata, verify rights, resolve transform/timebase semantics, qualify measurement uncertainty or calculate physical scores. Review the generated manifest and separately source the recording/article/session grouping before creating data splits.


## Evidence-backed source mapping and grouping audit v0.1

The stacked \`gauge-rigid-grouping-audit\` binary is a separate gate for the \`#6657\` grouping/split specification. It consumes two independently reviewed TSV manifests and verifies that their claims are anchored to the exact raw and metadata JSON bytes in the pinned source tree.

\`\`\`bash
cargo run --release --manifest-path benchmarks/gauge-rigid-raw-trial-contract/v0.1/Cargo.toml --bin gauge-rigid-grouping-audit -- \\
  --dataset-revision f95bb06e7d11142f88ffd3b6b56c7258deb8fff6 \\
  --raw-root /path/to/GAUGE-Dataset/data/rigid \\
  --metadata-root /path/to/GAUGE-Dataset/metadata/rigid \\
  --mapping-tsv mapping.tsv \\
  --grouping-tsv grouping.tsv \\
  --output gauge-rigid-grouping-receipt.jsonl
\`\`\`

Both roots must be existing \`rigid\` directories. The tool recursively enumerates JSON files in bytewise path order, rejects symlinks, requires exactly one mapping and grouping row for every raw JSON file, and requires every metadata JSON file to be referenced by at least one accepted mapping. It verifies each raw and metadata SHA-256 against the bytes currently on disk. \`sha256sum\` must be installed; the command fails closed if it is unavailable.

### Mapping manifest

The first line of \`mapping.tsv\` must be exactly:

\`\`\`text
raw_path	raw_sha256	metadata_path	metadata_sha256	mapping_status	evidence_path	evidence_field	evidence_value	reviewer	review_id	family	subtask	material
\`\`\`

One row is required per raw JSON file. \`metadata_path\` is relative to \`metadata/rigid\`; \`raw_path\` is relative to \`data/rigid\`. Mapped rows use \`EXPLICIT_SOURCE_MAPPING\` or \`REVIEWED_MAPPING\`, provide the mapped metadata's exact relative path and digest, and identify the field-level join evidence. \`REVIEWED_MAPPING\` additionally requires a reviewer and review ID. Use \`AMBIGUOUS\` or \`UNMAPPED\` when the source does not support a unique join; these rows must not nominate a metadata file. Use literal \`UNKNOWN\` in family/subtask/material when the source does not establish the value.

### Grouping manifest

The first line of \`grouping.tsv\` must be exactly:

\`\`\`text
raw_path	raw_sha256	unit_disposition	exchangeability_unit_id	group_evidence_path	group_evidence_field	group_evidence_value	reviewer	review_id	dependence_keys	split
\`\`\`

A \`SOURCE_BACKED\` row requires a source-backed unit identifier, exact evidence locator and field, observed value, reviewer, review ID, and a split of \`FIT\`, \`CALIBRATION\`, or \`TEST\`. The evidence path must point to that row's exact hashed source file as \`raw/<raw_path>\` or \`metadata/<metadata_path>\`. Shared dependence keys use semicolon-separated, namespaced values such as \`article:specimen-07;session:session-03;camera_calibration:cal-02\`. Allowed namespaces are \`article\`, \`session\`, \`setup\`, \`fixture\`, \`camera_calibration\`, \`run_block\`, and \`operator_block\`. IDs must come from source evidence or a separately documented review; do not derive them from filename numbering.

For an unresolved physical identity, use \`UNKNOWN\`, leave all identity/evidence/dependence fields empty, and set \`split\` to \`UNASSIGNED\`. Unknown rows remain in the receipt but never increase the independent-unit count. An ambiguous/unmapped metadata join cannot be assigned a source-backed physical unit.

### Receipt semantics and claim boundary

The validator connects rows that share a declared exchangeability-unit ID, a declared dependence key, or identical raw content hashes. A connected component may occupy only one split; any fit/calibration/test overlap is a nonzero exit and is recorded as a leakage failure. The JSONL receipt preserves row-level evidence locators, file hashes, split/component assignments, task × subtask × material unit counts, and an explicit claim ceiling.

For each declared domain, fewer than 19 calibration components produces \`UNBOUNDED_REQUIRED_BELOW_19_INDEPENDENT_UNITS\` at the nominal 95% conformal rank. Nineteen is only the minimum count for a finite rank to be possible under the existing exact finite-sample rule; it is not proof of exchangeability, correct domain pooling, calibrated measurement uncertainty, or coverage.

**Important limitations:** this tool verifies source-file hashes, manifest completeness, evidence-locator presence/anchoring, and leakage under the declared grouping keys. It does not parse arbitrary metadata schemas, prove that a JSON field pointer resolves to the claimed value, adjudicate whether a human-supplied grouping is scientifically correct, establish statistical independence/exchangeability, check reuse rights, or qualify physical fidelity. Those remain explicit review gates. Do not run it with guessed IDs to force a finite conformal threshold.
