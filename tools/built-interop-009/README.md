# BUILT-INTEROP-009 — Oracle-bound IFC mutation campaign

A Rust benchmark runner for the schema-directed structural diff in BUILT-INTEROP-008. It turns metamorphic unit cases into a scored, machine-readable execution campaign with explicit oracle rationale, pinned input identity and per-case outcomes.

## Run

Build the pinned diff tool and the campaign crate, then run:

```sh
cargo build --locked --manifest-path tools/built-interop-008/Cargo.toml
cargo run --locked --manifest-path tools/built-interop-009/Cargo.toml -- \
  --diff-bin /path/to/built-interop-semantic-diff \
  --diff-bin-sha256 $(sha256sum /path/to/built-interop-semantic-diff | awk '{print $1}') \
  --manifest tools/built-interop-009/campaign-v1.json \
  --run-id manual-campaign-001 \
  --reported-at 2026-10-19T15:00:00Z \
  --diff-tool-source-commit-sha 2700ec3ed321ef6d1389441222c5adc12d6a42a9 \
  --evaluator-source-commit-sha 0123456789abcdef0123456789abcdef01234567 \
  --output /tmp/interop-009-report.json
```

Use actual commit SHAs and the exact binary digest. The campaign exits 0 only when every observed status matches its predeclared outcome, 1 on outcome disagreement, and 64 for invalid metadata, manifest, fixture, binary or report contract.

## Oracle matrix

The checked-in `campaign-v1.json` is strictly parsed; the Rust code independently validates the exact ordered mutation IDs and expected status mapping before invoking the tool. The base fixture SHA-256 is compiled into the runner and the manifest. The runner refuses changed fixture bytes, case reordering, duplicate IDs/mutations, unknown mutations, altered expected outcomes, empty oracle reasoning, and `oracle_class` values other than `synthetic-metamorphic-v1`.

The 11 cases are:
- **Expected EQUAL (4):** identical bytes, timestamp-only header mutation, IfcRoot STEP-number renumbering, and DATA record reorder.
- **Expected DIFFERENT (3):** IFCWALL.Name change, root removal, and a valid GlobalId change.
- **Expected UNRESOLVED (4):** invalid GlobalId, duplicate GlobalId, unknown IFC schema token, and malformed STEP input.

These oracle labels are authored from the mutation semantics before tool invocation. The runner does not derive expectations from the tool.

## Scores

Every case records exact source/target SHA-256, target byte length, expected vs observed status, semantic result digest, decision reason, timing and oracle rationale. The aggregate separates:
- exact status agreement and mismatch rate;
- **false acceptance:** expected DIFFERENT but observed EQUAL;
- **false rejection:** expected EQUAL but observed DIFFERENT;
- **unsafe certainty:** expected UNRESOLVED but observed EQUAL/DIFFERENT;
- **unresolved instead of a decision:** expected EQUAL/DIFFERENT but observed UNRESOLVED.

Tool exit status must agree with its JSON status. The runner checks that the tool's reported source and target hashes/byte lengths match the exact bytes supplied. It also checks stable tool version and configuration across all cases.

## CI

The focused workflow checks out BUILT-INTEROP-008 at an immutable source commit, builds that exact binary with its own committed lock, checks its SHA-256 at runtime, tests this runner, executes the full 11-case campaign, and lints with warnings denied. The first workflow run bootstraps the runner's lockfile and displays rustfmt edits; after capturing those outputs, this step is replaced by a pinned lockfile checksum and strict `--locked` checks.

## Scope / limitations

This is a synthetic metamorphic campaign, not an empirical estimate of accuracy on commercial CAD files. The source fixture is deliberately small (two IFCWALL roots) and tests root identity/property semantics only. It does not compare geometry, quantities, units, tolerances, material flow, topology, IFC/IDS complete conformance or engineering suitability.

The official buildingSMART paired-corpus inventory (#2036) remains `UNVERIFIED` / `NOT_RUN_EXTERNAL_CORPUS` until independent ground-truth oracles exist. Filename labels are not truth. Real export/import benchmarks must be rights-cleared, tied to a frozen tool/profile and independently reviewed; report measured false acceptance/rejection and unresolved coverage before any quality or productivity claims.
